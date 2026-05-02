// Each integration test imports the same shared helper surface but uses a subset.
#![allow(dead_code)]

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::{
    model::InventoryEntry,
    store::InventoryDb,
    sync::{
        test_support::{
            build_delete_operation, build_entry_operation, canonical_operation_checksum,
            canonical_operation_json, operation_file_path, CorruptRemoteFile, SharedSyncPaths,
            SyncClientIdentity, SyncConflictRecord, SyncOperationEnvelope,
        },
        SyncOperationType,
    },
};

pub(crate) fn unique_test_dir(prefix: &str) -> PathBuf {
    env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4().simple()))
}

pub(crate) fn test_db(prefix: &str) -> InventoryDb {
    let root = unique_test_dir(prefix);
    fs::create_dir_all(&root).unwrap();
    InventoryDb::open_at(root.join("inventory.feox")).unwrap()
}

pub(crate) fn existing_shared_root(prefix: &str) -> PathBuf {
    let root = unique_test_dir(prefix);
    fs::create_dir_all(&root).unwrap();
    root
}

pub(crate) fn sample_entry(id: &str, entry_uuid: &str, description: &str) -> InventoryEntry {
    InventoryEntry {
        id: id.to_string(),
        database_id: id.parse::<i64>().ok(),
        entry_uuid: entry_uuid.to_string(),
        asset_number: format!("ME-{id}"),
        serial_number: format!("SN-{id}"),
        qty: Some(1.0),
        manufacturer: "Mitutoyo".to_string(),
        model: "500".to_string(),
        description: description.to_string(),
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

pub(crate) fn outbox_count(db: &InventoryDb) -> usize {
    let mut count = 0;
    db.scan_sync_outbox_records::<SyncOperationEnvelope, _>(None, usize::MAX, |_, _| {
        count += 1;
        Ok(true)
    })
    .unwrap();
    count
}

pub(crate) fn read_outbox_operation(db: &InventoryDb, local_seq: u64) -> SyncOperationEnvelope {
    db.sync_outbox_record(local_seq).unwrap().unwrap()
}

pub(crate) fn first_outbox_operation(db: &InventoryDb) -> SyncOperationEnvelope {
    let mut operation = None;
    db.scan_sync_outbox_records::<SyncOperationEnvelope, _>(None, 1, |_, record| {
        operation = Some(record);
        Ok(false)
    })
    .unwrap();
    operation.unwrap()
}

pub(crate) fn conflict_count(db: &InventoryDb) -> usize {
    let mut count = 0;
    db.scan_sync_conflict_records::<SyncConflictRecord, _>(usize::MAX, |_, _| {
        count += 1;
        Ok(true)
    })
    .unwrap();
    count
}

pub(crate) fn corrupt_remote_count(db: &InventoryDb) -> usize {
    let mut count = 0;
    db.scan_sync_corrupt_records::<CorruptRemoteFile, _>(usize::MAX, |_, _| {
        count += 1;
        Ok(true)
    })
    .unwrap();
    count
}

pub(crate) fn remote_upsert_operation(
    client_id: &str,
    local_seq: u64,
    op_id: &str,
    mutation_ts_utc: &str,
    entry: InventoryEntry,
) -> SyncOperationEnvelope {
    remote_upsert_operation_with_fields(
        client_id,
        local_seq,
        op_id,
        mutation_ts_utc,
        entry,
        vec!["description".to_string()],
        None,
    )
}

pub(crate) fn remote_upsert_operation_with_fields(
    client_id: &str,
    local_seq: u64,
    op_id: &str,
    mutation_ts_utc: &str,
    mut entry: InventoryEntry,
    changed_fields: Vec<String>,
    base_version: Option<String>,
) -> SyncOperationEnvelope {
    entry.updated_at = mutation_ts_utc.to_string();
    let identity = SyncClientIdentity {
        client_id: client_id.to_string(),
        device_id: format!("{client_id}-device"),
    };
    let mut operation = build_entry_operation(
        &identity,
        local_seq,
        SyncOperationType::InventoryEntryUpdate,
        entry,
        changed_fields,
        base_version,
    )
    .unwrap();
    operation.op_id = op_id.to_string();
    operation.mutation_ts_utc = mutation_ts_utc.to_string();
    operation.created_at_utc = mutation_ts_utc.to_string();
    operation.checksum = canonical_operation_checksum(&operation).unwrap();
    operation
}

pub(crate) fn remote_delete_operation(
    client_id: &str,
    local_seq: u64,
    op_id: &str,
    mutation_ts_utc: &str,
    entry_uuid: &str,
) -> SyncOperationEnvelope {
    let identity = SyncClientIdentity {
        client_id: client_id.to_string(),
        device_id: format!("{client_id}-device"),
    };
    let mut operation = build_delete_operation(
        &identity,
        local_seq,
        entry_uuid.to_string(),
        mutation_ts_utc,
        None,
    )
    .unwrap();
    operation.op_id = op_id.to_string();
    operation.created_at_utc = mutation_ts_utc.to_string();
    operation.checksum = canonical_operation_checksum(&operation).unwrap();
    operation
}

pub(crate) fn write_remote_operation(shared_root: &Path, operation: &SyncOperationEnvelope) {
    let paths = SharedSyncPaths::from_shared_root(shared_root);
    let path = operation_file_path(&paths, &operation.client_id, operation.local_seq).unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, canonical_operation_json(operation).unwrap()).unwrap();
}

pub(crate) fn operation_file_count(shared_root: &Path) -> usize {
    let ops_dir = shared_root.join("shared").join("inventory").join("ops");
    if !ops_dir.exists() {
        return 0;
    }

    let mut count = 0usize;
    for client_dir in fs::read_dir(ops_dir).unwrap() {
        let client_dir = client_dir.unwrap();
        if !client_dir.file_type().unwrap().is_dir() {
            continue;
        }
        count += fs::read_dir(client_dir.path())
            .unwrap()
            .filter(|entry| {
                entry
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".op.json")
            })
            .count();
    }
    count
}

pub(crate) fn manifest_path(shared_root: &Path) -> PathBuf {
    shared_root
        .join("shared")
        .join("inventory")
        .join("manifest.json")
}

pub(crate) fn snapshot_file_count(shared_root: &Path) -> usize {
    let snapshots_dir = shared_root
        .join("shared")
        .join("inventory")
        .join("snapshots");
    if !snapshots_dir.exists() {
        return 0;
    }

    fs::read_dir(snapshots_dir)
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".snapshot.json")
        })
        .count()
}

pub(crate) fn first_snapshot_path(shared_root: &Path) -> PathBuf {
    let snapshots_dir = shared_root
        .join("shared")
        .join("inventory")
        .join("snapshots");
    fs::read_dir(snapshots_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"))
        .expect("snapshot file should exist")
}

pub(crate) fn remove_json_field(path: &Path, field: &str) {
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    value.as_object_mut().unwrap().remove(field);
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}
