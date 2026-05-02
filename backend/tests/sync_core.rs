#[path = "support/backend.rs"]
mod backend;
pub(crate) use backend::{model, store, sync};

use std::{collections::HashMap, env, ffi::OsString, fs, path::PathBuf};

use model::InventoryEntry;
use store::InventoryDb;
use sync::test_support::{
    allocate_local_sequence, build_delete_operation, build_entry_operation,
    canonical_operation_checksum, canonical_operation_json, corrupt_remote_record_id,
    ensure_operation_log_layout, get_or_create_client_identity, operation_file_name,
    operation_file_path, peek_next_local_sequence, read_operation_file, record_corrupt_remote_file,
    resolve_shared_root_from_env_value, scan_operation_files,
    scan_operation_files_after_watermarks, set_test_hmac_key, sha256_hex, write_operation_file,
    CorruptRemoteFile, CorruptRemoteReason, SharedSyncPaths, SyncClientIdentity, SyncCoreErrorKind,
    SyncEntryState, SyncOperationEnvelope, SyncOperationPayload, SyncOperationType,
    SyncTombstoneRecord, DEFAULT_SHARED_ROOT, SNAPSHOT_APPLY_PENDING_KEY, SYNC_SCHEMA_VERSION,
};
use sync::{last_local_recovery_message, queue_entry_operation, recover_local_sync_state};
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
        let db = InventoryDb::open_at(db_path.clone()).unwrap();
        first_identity = get_or_create_client_identity(&db).unwrap();
        assert_eq!(allocate_local_sequence(&db).unwrap(), 1);
        assert_eq!(allocate_local_sequence(&db).unwrap(), 2);
    }

    let db = InventoryDb::open_at(db_path).unwrap();
    let second_identity = get_or_create_client_identity(&db).unwrap();
    assert_eq!(second_identity, first_identity);
    assert_eq!(peek_next_local_sequence(&db).unwrap(), 3);
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
fn hmac_signed_operation_reads_successfully_when_key_is_configured() {
    let _hmac = set_test_hmac_key(Some("0123456789abcdef"));
    let root = unique_test_dir("sync-hmac-signed-operation");
    let paths = SharedSyncPaths::from_shared_root(&root);
    ensure_operation_log_layout(&paths).unwrap();
    let identity = SyncClientIdentity {
        client_id: "client-hmac-signed".to_string(),
        device_id: "device-hmac-signed".to_string(),
    };
    let operation = build_entry_operation(
        &identity,
        1,
        SyncOperationType::InventoryEntryCreate,
        sample_entry("entry-hmac-signed"),
        Vec::new(),
        None,
    )
    .unwrap();

    assert!(operation
        .auth
        .as_deref()
        .unwrap_or("")
        .starts_with("hmac-sha256:"));
    let path = write_operation_file(&paths, &operation).unwrap();
    let read_back = read_operation_file(&path).unwrap();

    assert_eq!(read_back.op_id, operation.op_id);
    assert_eq!(read_back.auth, operation.auth);
}

#[test]
fn hmac_rejects_unsigned_tampered_and_wrong_key_operations() {
    let root = unique_test_dir("sync-hmac-rejected-operation");
    let paths = SharedSyncPaths::from_shared_root(&root);
    ensure_operation_log_layout(&paths).unwrap();

    let _hmac = set_test_hmac_key(Some("0123456789abcdef"));
    let unsigned = sample_operation("client-hmac-unsigned", 1, "entry-hmac-unsigned");
    write_raw_operation(&paths, &unsigned);
    let unsigned_path =
        operation_file_path(&paths, &unsigned.client_id, unsigned.local_seq).unwrap();
    let unsigned_corrupt = read_operation_file(&unsigned_path).unwrap_err();
    assert_eq!(
        unsigned_corrupt.reason,
        CorruptRemoteReason::InvalidEnvelope
    );
    assert!(unsigned_corrupt.detail.contains("authentication"));

    let identity = SyncClientIdentity {
        client_id: "client-hmac-tampered".to_string(),
        device_id: "device-hmac-tampered".to_string(),
    };
    let mut tampered = build_entry_operation(
        &identity,
        1,
        SyncOperationType::InventoryEntryUpdate,
        sample_entry("entry-hmac-tampered"),
        vec!["description".to_string()],
        None,
    )
    .unwrap();
    tampered.payload.entry.as_mut().unwrap().description = "Tampered".to_string();
    tampered.checksum = canonical_operation_checksum(&tampered).unwrap();
    write_raw_operation(&paths, &tampered);
    let tampered_path =
        operation_file_path(&paths, &tampered.client_id, tampered.local_seq).unwrap();
    let tampered_corrupt = read_operation_file(&tampered_path).unwrap_err();
    assert_eq!(
        tampered_corrupt.reason,
        CorruptRemoteReason::InvalidEnvelope
    );
    assert!(tampered_corrupt.detail.contains("HMAC"));

    let identity = SyncClientIdentity {
        client_id: "client-hmac-wrong-key".to_string(),
        device_id: "device-hmac-wrong-key".to_string(),
    };
    let signed_with_first_key = build_entry_operation(
        &identity,
        1,
        SyncOperationType::InventoryEntryCreate,
        sample_entry("entry-hmac-wrong-key"),
        Vec::new(),
        None,
    )
    .unwrap();
    write_raw_operation(&paths, &signed_with_first_key);

    drop(_hmac);
    let _wrong_hmac = set_test_hmac_key(Some("fedcba9876543210"));
    let wrong_key_path = operation_file_path(
        &paths,
        &signed_with_first_key.client_id,
        signed_with_first_key.local_seq,
    )
    .unwrap();
    let wrong_key_corrupt = read_operation_file(&wrong_key_path).unwrap_err();
    assert_eq!(
        wrong_key_corrupt.reason,
        CorruptRemoteReason::InvalidEnvelope
    );
    assert!(wrong_key_corrupt.detail.contains("HMAC"));
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
fn read_operation_file_rejects_malformed_remote_timestamps() {
    let root = unique_test_dir("sync-invalid-timestamp");
    let paths = SharedSyncPaths::from_shared_root(&root);
    ensure_operation_log_layout(&paths).unwrap();

    let mut operation = sample_operation("client-invalid-timestamp", 1, "entry-invalid-timestamp");
    operation.mutation_ts_utc = "not-a-timestamp".to_string();
    operation.checksum = canonical_operation_checksum(&operation).unwrap();
    write_raw_operation(&paths, &operation);

    let path = operation_file_path(&paths, &operation.client_id, operation.local_seq).unwrap();
    let corrupt = read_operation_file(&path).unwrap_err();

    assert_eq!(corrupt.reason, CorruptRemoteReason::InvalidEnvelope);
    assert!(corrupt.detail.contains("mutation_ts_utc"));
}

#[test]
fn read_operation_file_rejects_non_utc_remote_timestamps() {
    let root = unique_test_dir("sync-non-utc-timestamp");
    let paths = SharedSyncPaths::from_shared_root(&root);
    ensure_operation_log_layout(&paths).unwrap();

    let mut operation = sample_operation("client-non-utc-timestamp", 1, "entry-non-utc-timestamp");
    operation.created_at_utc = "2026-04-26T08:00:00.000-05:00".to_string();
    operation.checksum = canonical_operation_checksum(&operation).unwrap();
    write_raw_operation(&paths, &operation);

    let path = operation_file_path(&paths, &operation.client_id, operation.local_seq).unwrap();
    let corrupt = read_operation_file(&path).unwrap_err();

    assert_eq!(corrupt.reason, CorruptRemoteReason::InvalidEnvelope);
    assert!(corrupt.detail.contains("created_at_utc"));
    assert!(corrupt.detail.contains("UTC"));
}

#[test]
fn read_operation_file_accepts_old_and_future_utc_timestamps() {
    let root = unique_test_dir("sync-valid-skewed-timestamps");
    let paths = SharedSyncPaths::from_shared_root(&root);
    ensure_operation_log_layout(&paths).unwrap();

    let mut old_operation = sample_operation("client-old-timestamp", 1, "entry-old-timestamp");
    old_operation.created_at_utc = "2001-01-01T00:00:00.000Z".to_string();
    old_operation.mutation_ts_utc = "2001-01-01T00:00:00.000Z".to_string();
    old_operation.payload.entry.as_mut().unwrap().updated_at =
        old_operation.mutation_ts_utc.clone();
    old_operation.checksum = canonical_operation_checksum(&old_operation).unwrap();
    write_raw_operation(&paths, &old_operation);

    let mut future_operation =
        sample_operation("client-future-timestamp", 1, "entry-future-timestamp");
    future_operation.created_at_utc = "2099-01-01T00:00:00.000Z".to_string();
    future_operation.mutation_ts_utc = "2099-01-01T00:00:00.000Z".to_string();
    future_operation.payload.entry.as_mut().unwrap().updated_at =
        future_operation.mutation_ts_utc.clone();
    future_operation.checksum = canonical_operation_checksum(&future_operation).unwrap();
    write_raw_operation(&paths, &future_operation);

    assert!(read_operation_file(
        &operation_file_path(&paths, &old_operation.client_id, old_operation.local_seq).unwrap()
    )
    .is_ok());
    assert!(read_operation_file(
        &operation_file_path(
            &paths,
            &future_operation.client_id,
            future_operation.local_seq
        )
        .unwrap()
    )
    .is_ok());
}

#[test]
fn read_operation_file_rejects_non_utc_mutation_and_delete_payload_timestamps() {
    let root = unique_test_dir("sync-non-utc-mutation-delete");
    let paths = SharedSyncPaths::from_shared_root(&root);
    ensure_operation_log_layout(&paths).unwrap();

    let mut update_operation =
        sample_operation("client-non-utc-mutation", 1, "entry-non-utc-mutation");
    update_operation.mutation_ts_utc = "2026-04-26T08:00:00.000-05:00".to_string();
    update_operation.payload.entry.as_mut().unwrap().updated_at =
        update_operation.mutation_ts_utc.clone();
    update_operation.checksum = canonical_operation_checksum(&update_operation).unwrap();
    write_raw_operation(&paths, &update_operation);
    let update_corrupt = read_operation_file(
        &operation_file_path(
            &paths,
            &update_operation.client_id,
            update_operation.local_seq,
        )
        .unwrap(),
    )
    .unwrap_err();
    assert_eq!(update_corrupt.reason, CorruptRemoteReason::InvalidEnvelope);
    assert!(update_corrupt.detail.contains("mutation_ts_utc"));

    let mut delete_operation = build_delete_operation(
        &SyncClientIdentity {
            client_id: "client-non-utc-delete".to_string(),
            device_id: "device-non-utc-delete".to_string(),
        },
        1,
        "entry-non-utc-delete",
        "2026-04-26T13:00:00.000Z",
        None,
    )
    .unwrap();
    delete_operation.payload.deleted_at_utc = Some("2026-04-26T08:00:00.000-05:00".to_string());
    delete_operation.checksum = canonical_operation_checksum(&delete_operation).unwrap();
    write_raw_operation(&paths, &delete_operation);
    let delete_corrupt = read_operation_file(
        &operation_file_path(
            &paths,
            &delete_operation.client_id,
            delete_operation.local_seq,
        )
        .unwrap(),
    )
    .unwrap_err();
    assert_eq!(delete_corrupt.reason, CorruptRemoteReason::InvalidEnvelope);
    assert!(delete_corrupt.detail.contains("payload.deleted_at_utc"));
}

#[test]
fn recovery_repairs_partial_local_outbox_operation() {
    let root = unique_test_dir("sync-recovery-outbox");
    let db = InventoryDb::open_at(root.join("inventory.feox")).unwrap();
    let operation = sample_operation("client-recovery", 1, "entry-recovery");

    db.put_sync_outbox_record(operation.local_seq, &operation)
        .unwrap();
    db.flush();

    let report = recover_local_sync_state(&db).unwrap();

    assert_eq!(report.repaired_outbox_operations, 1);
    assert!(db.has_sync_applied_marker(&operation.op_id).unwrap());
    assert_eq!(
        db.sync_client_seq_marker::<String>(&operation.client_id, operation.local_seq)
            .unwrap()
            .as_deref(),
        Some(operation.op_id.as_str())
    );
    assert!(db.find_entry(&operation.entity_id).unwrap().is_some());
    assert_eq!(
        db.sync_entry_state::<SyncEntryState>(&operation.entity_id)
            .unwrap()
            .unwrap()
            .last_op_id,
        operation.op_id
    );
}

#[test]
fn recovery_replays_current_outbox_update_over_stale_entry() {
    let root = unique_test_dir("sync-recovery-stale-entry");
    let db = InventoryDb::open_at(root.join("inventory.feox")).unwrap();
    let mut stale_entry = sample_entry("entry-recovery-stale");
    stale_entry.description = "Before crash".to_string();
    stale_entry.updated_at = "2026-04-26T00:00:00.000Z".to_string();
    db.put_entry(&stale_entry).unwrap();

    let mut operation = sample_operation("client-recovery-stale", 1, "entry-recovery-stale");
    operation.mutation_ts_utc = "2026-04-26T00:00:01.000Z".to_string();
    operation.created_at_utc = operation.mutation_ts_utc.clone();
    operation.payload.entry.as_mut().unwrap().description = "Recovered update".to_string();
    operation.payload.entry.as_mut().unwrap().updated_at = operation.mutation_ts_utc.clone();
    operation.checksum = canonical_operation_checksum(&operation).unwrap();
    db.put_sync_outbox_record(operation.local_seq, &operation)
        .unwrap();
    db.flush();

    let report = recover_local_sync_state(&db).unwrap();

    assert_eq!(report.repaired_outbox_operations, 1);
    assert_eq!(report.repaired_entries, 1);
    assert_eq!(
        db.find_entry("entry-recovery-stale")
            .unwrap()
            .unwrap()
            .description,
        "Recovered update"
    );
    assert_eq!(
        db.sync_entry_state::<SyncEntryState>("entry-recovery-stale")
            .unwrap()
            .unwrap()
            .last_op_id,
        operation.op_id
    );
}

#[test]
fn recovery_does_not_replay_stale_outbox_over_newer_state() {
    let root = unique_test_dir("sync-recovery-stale-outbox");
    let db = InventoryDb::open_at(root.join("inventory.feox")).unwrap();
    let mut current_entry = sample_entry("entry-recovery-current");
    current_entry.description = "Newer state".to_string();
    current_entry.updated_at = "2026-04-26T00:00:02.000Z".to_string();
    db.put_entry(&current_entry).unwrap();
    db.put_sync_entry_state(
        &current_entry.entry_uuid,
        &entry_state(
            &current_entry.entry_uuid,
            "op-newer-state",
            "2026-04-26T00:00:02.000Z",
            false,
        ),
    )
    .unwrap();

    let mut operation = sample_operation("client-recovery-old", 1, "entry-recovery-current");
    operation.payload.entry.as_mut().unwrap().description = "Old outbox".to_string();
    operation.checksum = canonical_operation_checksum(&operation).unwrap();
    db.put_sync_outbox_record(operation.local_seq, &operation)
        .unwrap();
    db.flush();

    let report = recover_local_sync_state(&db).unwrap();

    assert_eq!(report.repaired_entries, 0);
    assert_eq!(
        db.find_entry("entry-recovery-current")
            .unwrap()
            .unwrap()
            .description,
        "Newer state"
    );
    assert_eq!(
        db.sync_entry_state::<SyncEntryState>("entry-recovery-current")
            .unwrap()
            .unwrap()
            .last_op_id,
        "op-newer-state"
    );
}

#[test]
fn recovery_repairs_partial_delete_outbox_operation() {
    let root = unique_test_dir("sync-recovery-delete");
    let db = InventoryDb::open_at(root.join("inventory.feox")).unwrap();
    let entry = sample_entry("entry-recovery-delete");
    db.put_entry(&entry).unwrap();
    let operation = build_delete_operation(
        &SyncClientIdentity {
            client_id: "client-recovery-delete".to_string(),
            device_id: "device-recovery-delete".to_string(),
        },
        1,
        entry.entry_uuid.clone(),
        "2026-04-26T00:00:01.000Z",
        None,
    )
    .unwrap();
    db.put_sync_outbox_record(operation.local_seq, &operation)
        .unwrap();
    db.flush();

    let report = recover_local_sync_state(&db).unwrap();

    assert_eq!(report.repaired_entries, 1);
    assert_eq!(report.repaired_tombstones, 1);
    assert!(db.find_entry("entry-recovery-delete").unwrap().is_none());
    assert_eq!(
        db.sync_tombstone::<SyncTombstoneRecord>("entry-recovery-delete")
            .unwrap()
            .unwrap()
            .op_id,
        operation.op_id
    );
    assert!(
        db.sync_entry_state::<SyncEntryState>("entry-recovery-delete")
            .unwrap()
            .unwrap()
            .deleted
    );
}

#[test]
fn recovery_removes_stale_tombstone_when_restored_state_is_newer() {
    let root = unique_test_dir("sync-recovery-stale-tombstone");
    let db = InventoryDb::open_at(root.join("inventory.feox")).unwrap();
    let mut entry = sample_entry("entry-recovery-restore");
    entry.description = "Restored".to_string();
    entry.updated_at = "2026-04-26T00:00:02.000Z".to_string();
    db.put_entry(&entry).unwrap();
    db.put_sync_tombstone(
        &entry.entry_uuid,
        &SyncTombstoneRecord {
            entry_uuid: entry.entry_uuid.clone(),
            deleted_at_utc: "2026-04-26T00:00:01.000Z".to_string(),
            op_id: "op-old-delete".to_string(),
            client_id: "client-old-delete".to_string(),
            local_seq: 1,
            base_version: None,
        },
    )
    .unwrap();
    db.put_sync_entry_state(
        &entry.entry_uuid,
        &entry_state(
            &entry.entry_uuid,
            "op-newer-restore",
            "2026-04-26T00:00:02.000Z",
            false,
        ),
    )
    .unwrap();
    db.flush();

    let report = recover_local_sync_state(&db).unwrap();

    assert_eq!(report.repaired_tombstones, 1);
    assert!(db
        .sync_tombstone::<SyncTombstoneRecord>("entry-recovery-restore")
        .unwrap()
        .is_none());
    assert!(db.find_entry("entry-recovery-restore").unwrap().is_some());
}

#[test]
fn recovery_advances_next_local_sequence_after_existing_outbox() {
    let root = unique_test_dir("sync-recovery-local-seq");
    let db = InventoryDb::open_at(root.join("inventory.feox")).unwrap();
    db.set_next_local_seq(1).unwrap();
    let operation = sample_operation("client-recovery-seq", 5, "entry-recovery-seq");
    db.put_sync_outbox_record(operation.local_seq, &operation)
        .unwrap();
    db.flush();

    let report = recover_local_sync_state(&db).unwrap();

    assert_eq!(report.repaired_local_sequence_markers, 1);
    assert_eq!(peek_next_local_sequence(&db).unwrap(), 6);
}

#[test]
fn recovery_reports_interrupted_snapshot_apply() {
    let root = unique_test_dir("sync-recovery-pending-snapshot");
    let db = InventoryDb::open_at(root.join("inventory.feox")).unwrap();
    db.put_sync_value(SNAPSHOT_APPLY_PENDING_KEY, b"snapshot-interrupted")
        .unwrap();
    db.flush();

    let report = recover_local_sync_state(&db).unwrap();

    assert_eq!(
        report.snapshot_apply_pending.as_deref(),
        Some("snapshot-interrupted")
    );
    assert!(last_local_recovery_message(&db)
        .unwrap()
        .contains("interrupted snapshot"));
}

#[test]
fn recovery_reports_pending_snapshot_when_keyspaces_were_partially_replaced() {
    let root = unique_test_dir("sync-recovery-partial-snapshot");
    let db = InventoryDb::open_at(root.join("inventory.feox")).unwrap();
    let entry = sample_entry("entry-partial-snapshot");
    db.put_sync_value(SNAPSHOT_APPLY_PENDING_KEY, b"snapshot-partial")
        .unwrap();
    db.put_entry(&entry).unwrap();
    db.put_sync_entry_state(
        &entry.entry_uuid,
        &entry_state(
            &entry.entry_uuid,
            "op-partial-snapshot",
            "2026-04-26T00:00:00.000Z",
            false,
        ),
    )
    .unwrap();
    db.set_sync_watermark("client-partial-snapshot", 3).unwrap();
    db.flush();

    let report = recover_local_sync_state(&db).unwrap();

    assert_eq!(
        report.snapshot_apply_pending.as_deref(),
        Some("snapshot-partial")
    );
    assert!(db.find_entry(&entry.entry_uuid).unwrap().is_some());
    assert_eq!(
        db.sync_watermark("client-partial-snapshot").unwrap(),
        Some(3)
    );
}

#[test]
fn recovery_clears_completed_pending_snapshot_apply_marker() {
    let root = unique_test_dir("sync-recovery-completed-snapshot");
    let db = InventoryDb::open_at(root.join("inventory.feox")).unwrap();
    db.put_sync_value(SNAPSHOT_APPLY_PENDING_KEY, b"snapshot-complete")
        .unwrap();
    db.set_last_snapshot_id("snapshot-complete").unwrap();
    db.flush();

    let report = recover_local_sync_state(&db).unwrap();

    assert!(report.snapshot_apply_pending.is_none());
    assert!(db
        .get_sync_value(SNAPSHOT_APPLY_PENDING_KEY)
        .unwrap()
        .is_none());
}

#[test]
fn recovery_leaves_clean_local_operation_state_unchanged() {
    let root = unique_test_dir("sync-recovery-clean");
    let db = InventoryDb::open_at(root.join("inventory.feox")).unwrap();
    let entry = sample_entry("entry-recovery-clean");
    db.put_entry(&entry).unwrap();
    queue_entry_operation(
        &db,
        SyncOperationType::InventoryEntryCreate,
        entry,
        Vec::new(),
        None,
    )
    .unwrap();
    db.flush();
    let revision_before = db.sync_revision().unwrap();

    let report = recover_local_sync_state(&db).unwrap();

    assert_eq!(report.repaired_client_seq_markers, 0);
    assert_eq!(report.repaired_entry_states, 0);
    assert_eq!(report.repaired_entries, 0);
    assert_eq!(report.repaired_local_sequence_markers, 0);
    assert_eq!(report.repaired_outbox_operations, 0);
    assert_eq!(report.repaired_tombstones, 0);
    assert!(report.snapshot_apply_pending.is_none());
    assert_eq!(db.sync_revision().unwrap(), revision_before);
}

#[test]
fn scan_operation_files_can_skip_watermarked_sequences() {
    let root = unique_test_dir("sync-scan-watermark");
    let paths = SharedSyncPaths::from_shared_root(&root);
    ensure_operation_log_layout(&paths).unwrap();

    write_operation_file(&paths, &sample_operation("client-a", 1, "entry-a")).unwrap();
    write_operation_file(&paths, &sample_operation("client-a", 2, "entry-a")).unwrap();
    write_operation_file(&paths, &sample_operation("client-b", 1, "entry-b")).unwrap();

    let mut watermarks = HashMap::new();
    watermarks.insert("client-a".to_string(), 1);
    let report = scan_operation_files_after_watermarks(&paths, &watermarks).unwrap();

    let scanned_sequences = report
        .operations
        .iter()
        .map(|operation| (operation.client_id.as_str(), operation.local_seq))
        .collect::<Vec<_>>();
    assert_eq!(scanned_sequences, vec![("client-a", 2), ("client-b", 1)]);
    assert_eq!(report.ignored_watermarked_files, 1);
}

#[test]
fn corrupt_remote_markers_are_written_to_sync_keyspace() {
    let root = unique_test_dir("sync-corrupt-record");
    let db = InventoryDb::open_at(root.join("inventory.feox")).unwrap();
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
        schema_version: SYNC_SCHEMA_VERSION,
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
        auth: None,
    };
    operation.checksum = canonical_operation_checksum(&operation).unwrap();
    operation
}

fn entry_state(
    entry_uuid: &str,
    op_id: &str,
    mutation_ts_utc: &str,
    deleted: bool,
) -> SyncEntryState {
    SyncEntryState {
        entry_uuid: entry_uuid.to_string(),
        last_op_id: op_id.to_string(),
        mutation_ts_utc: mutation_ts_utc.to_string(),
        deleted,
        base_version: None,
        changed_fields: Vec::new(),
        source_client_id: "client-state".to_string(),
        source_local_seq: 1,
        operation_type: if deleted {
            SyncOperationType::InventoryEntryDelete
        } else {
            SyncOperationType::InventoryEntryUpdate
        },
        updated_at_utc: mutation_ts_utc.to_string(),
    }
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
