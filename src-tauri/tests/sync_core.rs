#[allow(dead_code)]
#[path = "../src/model.rs"]
mod model;
#[allow(dead_code)]
#[path = "../src/store.rs"]
mod store;
#[allow(dead_code)]
#[path = "../src/sync.rs"]
mod sync;

use std::{env, ffi::OsString, fs, path::PathBuf};

use model::InventoryEntry;
use store::InventoryDb;
use sync::{
    allocate_local_sequence, build_delete_operation, build_entry_operation,
    canonical_operation_checksum, canonical_operation_json, corrupt_remote_record_id,
    ensure_operation_log_layout, get_or_create_client_identity, operation_file_name,
    read_operation_file, record_corrupt_remote_file, resolve_shared_root_from_env_value,
    scan_operation_files, sha256_hex, write_operation_file, CorruptRemoteFile, CorruptRemoteReason,
    SharedSyncPaths, SyncClientIdentity, SyncCoreErrorKind, SyncOperationEnvelope,
    SyncOperationPayload, SyncOperationType, DEFAULT_SHARED_ROOT,
};
use uuid::Uuid;

#[test]
fn shared_root_prefers_env_override_and_defaults_to_me_path() {
    assert_eq!(
        resolve_shared_root_from_env_value(None),
        PathBuf::from(DEFAULT_SHARED_ROOT)
    );
    assert_eq!(
        resolve_shared_root_from_env_value(Some(OsString::from("  C:\\ME Shared Root  "))),
        PathBuf::from("C:\\ME Shared Root")
    );
}

#[test]
fn client_identity_and_local_sequence_are_persisted_in_inventory_db() {
    let root = unique_test_dir("sync-identity");
    let db_path = root.join("inventory.feox");

    let first_identity;
    {
        let db = InventoryDb::open_at(db_path.clone(), None).unwrap();
        first_identity = get_or_create_client_identity(&db).unwrap();
        assert_eq!(allocate_local_sequence(&db).unwrap(), 1);
        assert_eq!(allocate_local_sequence(&db).unwrap(), 2);
    }

    let db = InventoryDb::open_at(db_path, None).unwrap();
    let second_identity = get_or_create_client_identity(&db).unwrap();
    assert_eq!(second_identity, first_identity);
    assert_eq!(sync::peek_next_local_sequence(&db).unwrap(), 3);
}

#[test]
fn canonical_checksum_ignores_checksum_field_and_uses_sha256() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    let mut operation = sample_operation("client-a", 1, "entry-a");
    let checksum = operation.checksum.clone();

    operation.checksum = "sha256:bad".to_string();
    assert_eq!(canonical_operation_checksum(&operation).unwrap(), checksum);

    operation.entity_id = "entry-b".to_string();
    assert_ne!(canonical_operation_checksum(&operation).unwrap(), checksum);
}

#[test]
fn write_operation_file_uses_final_op_path_and_read_validates_it() {
    let root = unique_test_dir("sync-write");
    let paths = SharedSyncPaths::from_shared_root(&root);
    ensure_operation_log_layout(&paths).unwrap();

    let operation = sample_operation("client-a", 1, "entry-a");
    let final_path = write_operation_file(&paths, &operation).unwrap();

    assert_eq!(
        final_path.file_name().unwrap().to_string_lossy(),
        operation_file_name(1)
    );
    assert!(final_path.exists());
    assert_eq!(
        fs::read_dir(final_path.parent().unwrap())
            .unwrap()
            .filter(|entry| entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".tmp-"))
            .count(),
        0
    );

    let read_back = read_operation_file(&final_path).unwrap();
    assert_eq!(read_back.client_id, "client-a");
    assert_eq!(read_back.local_seq, 1);
    assert_eq!(read_back.checksum, operation.checksum);

    let report = scan_operation_files(&paths).unwrap();
    assert_eq!(report.operations.len(), 1);
    assert!(report.corrupt.is_empty());

    assert_eq!(
        write_operation_file(&paths, &operation).unwrap(),
        final_path
    );

    let mut conflicting = sample_operation("client-a", 1, "entry-a");
    conflicting.op_id = "different-op-id".to_string();
    conflicting.checksum = canonical_operation_checksum(&conflicting).unwrap();
    let error = write_operation_file(&paths, &conflicting).unwrap_err();
    assert_eq!(error.kind, SyncCoreErrorKind::ExistingOperationConflict);
}

#[test]
fn scan_operation_files_ignores_temps_and_reports_corrupt_remote_files() {
    let root = unique_test_dir("sync-scan");
    let paths = SharedSyncPaths::from_shared_root(&root);
    ensure_operation_log_layout(&paths).unwrap();

    let valid = sample_operation("client-a", 1, "entry-a");
    write_operation_file(&paths, &valid).unwrap();

    let client_a_dir = paths.ops_dir.join("client-a");
    fs::write(
        client_a_dir.join("000000000002.op.json.tmp-1234-write"),
        b"partial",
    )
    .unwrap();
    fs::write(client_a_dir.join("notes.txt"), b"ignore me").unwrap();

    let malformed_dir = paths.ops_dir.join("client-malformed");
    fs::create_dir_all(&malformed_dir).unwrap();
    fs::write(malformed_dir.join("000000000001.op.json"), b"{not json").unwrap();

    let mut bad_checksum = sample_operation("client-bad-checksum", 1, "entry-bad");
    bad_checksum.checksum = "sha256:bad".to_string();
    write_raw_operation(&paths, &bad_checksum);

    let identity_mismatch = sample_operation("client-real", 1, "entry-real");
    write_raw_operation_under(&paths, "client-wrong-folder", 1, &identity_mismatch);

    let seq_mismatch = sample_operation("client-seq", 3, "entry-seq");
    write_raw_operation_under(&paths, "client-seq", 4, &seq_mismatch);

    let mut payload_mismatch = sample_operation("client-payload-mismatch", 1, "entry-envelope");
    payload_mismatch.payload.entry.as_mut().unwrap().entry_uuid = "entry-payload".to_string();
    payload_mismatch.checksum = canonical_operation_checksum(&payload_mismatch).unwrap();
    write_raw_operation(&paths, &payload_mismatch);

    let mut delete_payload_mismatch = build_delete_operation(
        &SyncClientIdentity {
            client_id: "client-delete-mismatch".to_string(),
            device_id: "device-delete-mismatch".to_string(),
        },
        1,
        "entry-delete-envelope",
        "2026-04-26T00:00:00.000Z",
        None,
    )
    .unwrap();
    delete_payload_mismatch.payload.entry_uuid = Some("entry-delete-payload".to_string());
    delete_payload_mismatch.checksum =
        canonical_operation_checksum(&delete_payload_mismatch).unwrap();
    write_raw_operation(&paths, &delete_payload_mismatch);

    let report = scan_operation_files(&paths).unwrap();
    assert_eq!(report.operations.len(), 1);
    assert_eq!(report.ignored_temp_files, 1);
    assert_eq!(report.ignored_unknown_files, 1);

    let reasons = report
        .corrupt
        .iter()
        .map(|corrupt| corrupt.reason)
        .collect::<Vec<_>>();
    assert!(reasons.contains(&CorruptRemoteReason::MalformedJson));
    assert!(reasons.contains(&CorruptRemoteReason::InvalidChecksum));
    assert!(reasons.contains(&CorruptRemoteReason::ClientIdMismatch));
    assert!(reasons.contains(&CorruptRemoteReason::LocalSeqMismatch));
    assert!(reasons.contains(&CorruptRemoteReason::InvalidEnvelope));
}

#[test]
fn corrupt_remote_markers_are_written_to_sync_keyspace() {
    let root = unique_test_dir("sync-corrupt-record");
    let db = InventoryDb::open_at(root.join("inventory.feox"), None).unwrap();
    let corrupt = CorruptRemoteFile {
        path: "S:\\shared\\inventory\\ops\\client\\000000000001.op.json".to_string(),
        reason: CorruptRemoteReason::InvalidChecksum,
        detail: "bad checksum".to_string(),
        detected_at_utc: "2026-04-26T00:00:00.000Z".to_string(),
        content_sha256: Some("sha256:abc".to_string()),
    };

    record_corrupt_remote_file(&db, &corrupt).unwrap();

    let stored = db
        .sync_corrupt_record::<CorruptRemoteFile>(&corrupt_remote_record_id(&corrupt))
        .unwrap()
        .unwrap();
    assert_eq!(stored.reason, CorruptRemoteReason::InvalidChecksum);
    assert_eq!(stored.detail, "bad checksum");
}

#[test]
fn entry_operation_builder_carries_full_entry_payload() {
    let identity = SyncClientIdentity {
        client_id: "client-create".to_string(),
        device_id: "device-create".to_string(),
    };
    let operation = build_entry_operation(
        &identity,
        5,
        SyncOperationType::InventoryEntryCreate,
        sample_entry("entry-create"),
        vec!["description".to_string(), "qty".to_string()],
        Some("base-1".to_string()),
    )
    .unwrap();

    assert_eq!(
        operation.operation_type,
        SyncOperationType::InventoryEntryCreate
    );
    assert_eq!(operation.entity_type, "inventory_entry");
    assert_eq!(operation.entity_id, "entry-create");
    assert_eq!(operation.base_version.as_deref(), Some("base-1"));
    assert_eq!(
        operation.payload.entry.as_ref().unwrap().entry_uuid,
        "entry-create"
    );
    assert_eq!(operation.payload.changed_fields, ["description", "qty"]);
    assert_eq!(
        canonical_operation_checksum(&operation).unwrap(),
        operation.checksum
    );
}

#[test]
fn delete_operation_payload_keeps_tombstone_details_only() {
    let identity = SyncClientIdentity {
        client_id: "client-delete".to_string(),
        device_id: "device-delete".to_string(),
    };

    let operation = build_delete_operation(
        &identity,
        7,
        "entry-delete",
        "2026-04-26T00:00:00.000Z",
        None,
    )
    .unwrap();

    assert_eq!(
        operation.operation_type,
        SyncOperationType::InventoryEntryDelete
    );
    assert_eq!(operation.entity_id, "entry-delete");
    assert!(operation.payload.entry.is_none());
    assert_eq!(
        operation.payload.entry_uuid.as_deref(),
        Some("entry-delete")
    );
    assert!(operation.checksum.starts_with("sha256:"));
}

fn sample_operation(client_id: &str, local_seq: u64, entry_uuid: &str) -> SyncOperationEnvelope {
    let entry = sample_entry(entry_uuid);
    let mut operation = SyncOperationEnvelope {
        schema_version: sync::SYNC_SCHEMA_VERSION,
        op_id: format!("op-{client_id}-{local_seq}"),
        client_id: client_id.to_string(),
        device_id: format!("device-{client_id}"),
        local_seq,
        app_version: "0.9.7".to_string(),
        created_at_utc: "2026-04-26T00:00:00.000Z".to_string(),
        operation_type: SyncOperationType::InventoryEntryUpdate,
        entity_type: "inventory_entry".to_string(),
        entity_id: entry_uuid.to_string(),
        base_version: None,
        mutation_ts_utc: "2026-04-26T00:00:00.000Z".to_string(),
        payload: SyncOperationPayload::entry(entry, vec!["description".to_string()]),
        checksum: String::new(),
    };
    operation.checksum = canonical_operation_checksum(&operation).unwrap();
    operation
}

fn write_raw_operation(paths: &SharedSyncPaths, operation: &SyncOperationEnvelope) {
    write_raw_operation_under(paths, &operation.client_id, operation.local_seq, operation);
}

fn write_raw_operation_under(
    paths: &SharedSyncPaths,
    folder_client_id: &str,
    file_seq: u64,
    operation: &SyncOperationEnvelope,
) {
    let dir = paths.ops_dir.join(folder_client_id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(operation_file_name(file_seq)),
        canonical_operation_json(operation).unwrap(),
    )
    .unwrap();
}

fn sample_entry(entry_uuid: &str) -> InventoryEntry {
    InventoryEntry {
        id: "1".to_string(),
        database_id: Some(1),
        entry_uuid: entry_uuid.to_string(),
        asset_number: "ME-1".to_string(),
        serial_number: "SN-1".to_string(),
        qty: Some(1.0),
        manufacturer: "Mitutoyo".to_string(),
        model: "500".to_string(),
        description: "Caliper".to_string(),
        project_name: "ME".to_string(),
        location: "Lab".to_string(),
        assigned_to: String::new(),
        links: String::new(),
        notes: String::new(),
        lifecycle_status: "active".to_string(),
        working_status: "unknown".to_string(),
        condition: String::new(),
        verified_in_survey: false,
        archived: false,
        manual_entry: true,
        picture_path: String::new(),
        created_at: "2026-04-26T00:00:00.000Z".to_string(),
        updated_at: "2026-04-26T00:00:00.000Z".to_string(),
    }
}

fn unique_test_dir(prefix: &str) -> PathBuf {
    env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4().simple()))
}
