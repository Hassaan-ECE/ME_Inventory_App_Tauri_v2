#[allow(dead_code)]
#[path = "../src/model.rs"]
mod model;
#[allow(dead_code)]
#[path = "../src/store.rs"]
mod store;
#[allow(dead_code)]
#[path = "../src/sync.rs"]
mod sync;

use std::{env, fs, path::PathBuf};

use model::InventoryEntry;
use store::InventoryDb;
use sync::{
    build_delete_operation, build_entry_operation, canonical_operation_checksum,
    canonical_operation_json, operation_file_path, run_shared_sync_with_root, SharedSyncPaths,
    SyncClientIdentity, SyncConflictRecord, SyncEntryState, SyncOperationEnvelope,
    SyncOperationType,
};
use uuid::Uuid;

#[test]
fn conflicting_existing_operation_file_keeps_local_operation_pending() {
    let db = test_db("sync-conflicting-push");
    let shared_root = existing_shared_root("sync-conflicting-push-root");
    run_shared_sync_with_root(&db, &shared_root).unwrap();

    let entry = sample_entry("1", "entry-conflict", "Local pending");
    db.put_entry(&entry).unwrap();
    sync::queue_entry_operation(
        &db,
        SyncOperationType::InventoryEntryCreate,
        entry,
        Vec::new(),
        None,
    )
    .unwrap();
    db.flush();

    let local_operation = first_outbox_operation(&db);
    let mut conflicting_operation = local_operation.clone();
    conflicting_operation.op_id = "different-op-id".to_string();
    conflicting_operation.checksum = canonical_operation_checksum(&conflicting_operation).unwrap();

    let paths = SharedSyncPaths::from_shared_root(&shared_root);
    let conflict_path = operation_file_path(
        &paths,
        &conflicting_operation.client_id,
        conflicting_operation.local_seq,
    )
    .unwrap();
    fs::create_dir_all(conflict_path.parent().unwrap()).unwrap();
    fs::write(
        &conflict_path,
        canonical_operation_json(&conflicting_operation).unwrap(),
    )
    .unwrap();

    let result = run_shared_sync_with_root(&db, &shared_root).unwrap();

    assert_eq!(result.shared.has_local_only_changes, Some(true));
    assert!(result.shared.message.contains("Pending local changes"));
    assert!(corrupt_remote_count(&db) >= 1);
}

#[test]
fn delete_tombstone_for_absent_entry_is_persisted_after_pull() {
    let db_source = test_db("sync-delete-source");
    let target_root = unique_test_dir("sync-delete-target");
    fs::create_dir_all(&target_root).unwrap();
    let target_path = target_root.join("inventory.feox");
    let shared_root = existing_shared_root("sync-delete-root");

    sync::queue_delete_operation(&db_source, "entry-absent", "2026-04-26T13:00:00.000Z", None)
        .unwrap();
    db_source.flush();
    run_shared_sync_with_root(&db_source, &shared_root).unwrap();

    {
        let db_target = InventoryDb::open_at(target_path.clone(), None).unwrap();
        let result = run_shared_sync_with_root(&db_target, &shared_root).unwrap();
        assert!(!result.entries_changed);
        assert!(db_target
            .sync_tombstone::<sync::SyncTombstoneRecord>("entry-absent")
            .unwrap()
            .is_some());
    }

    let reopened = InventoryDb::open_at(target_path, None).unwrap();
    assert!(reopened
        .sync_tombstone::<sync::SyncTombstoneRecord>("entry-absent")
        .unwrap()
        .is_some());
}

#[test]
fn tombstone_blocks_older_remote_upsert_from_resurrecting_entry() {
    let db_deleted = test_db("sync-tombstone-target");
    let db_source = test_db("sync-tombstone-source");
    let shared_root = existing_shared_root("sync-tombstone-root");

    run_shared_sync_with_root(&db_deleted, &shared_root).unwrap();
    run_shared_sync_with_root(&db_source, &shared_root).unwrap();

    sync::queue_delete_operation(
        &db_deleted,
        "entry-deleted",
        "2026-04-26T13:00:00.000Z",
        None,
    )
    .unwrap();
    db_deleted.flush();

    let mut old_entry = sample_entry("1", "entry-deleted", "Old upsert");
    old_entry.updated_at = "2026-04-26T12:00:00.000Z".to_string();
    db_source.put_entry(&old_entry).unwrap();
    sync::queue_entry_operation(
        &db_source,
        SyncOperationType::InventoryEntryCreate,
        old_entry,
        Vec::new(),
        None,
    )
    .unwrap();
    db_source.flush();

    run_shared_sync_with_root(&db_source, &shared_root).unwrap();
    let result = run_shared_sync_with_root(&db_deleted, &shared_root).unwrap();

    assert!(!result.entries_changed);
    assert!(db_deleted.find_entry("entry-deleted").unwrap().is_none());
    assert_eq!(conflict_count(&db_deleted), 1);
}

#[test]
fn newer_remote_update_overwrites_older_local_state() {
    let db = test_db("sync-lww-newer-target");
    let shared_root = existing_shared_root("sync-lww-newer-root");
    let mut local_entry = sample_entry("1", "entry-lww-newer", "Older local");
    local_entry.updated_at = "2026-04-26T12:00:00.000Z".to_string();
    db.put_entry(&local_entry).unwrap();
    db.put_sync_value("meta:sync_bootstrap_complete", b"test")
        .unwrap();
    db.put_sync_entry_state(
        &local_entry.entry_uuid,
        &entry_state_for_test(&local_entry, "op-local", false),
    )
    .unwrap();
    db.flush();

    let mut remote_entry = local_entry.clone();
    remote_entry.description = "Newer remote".to_string();
    remote_entry.updated_at = "2026-04-26T13:00:00.000Z".to_string();
    let remote_operation = remote_upsert_operation(
        "remote-newer-client",
        1,
        "op-remote-newer",
        "2026-04-26T13:00:00.000Z",
        remote_entry,
    );
    write_remote_operation(&shared_root, &remote_operation);

    let before_revision = db.sync_revision().unwrap();
    let result = run_shared_sync_with_root(&db, &shared_root).unwrap();

    assert!(result.entries_changed);
    assert_eq!(
        db.find_entry("entry-lww-newer")
            .unwrap()
            .unwrap()
            .description,
        "Newer remote"
    );
    assert_eq!(conflict_count(&db), 0);
    assert_eq!(db.sync_revision().unwrap(), before_revision + 1);
}

#[test]
fn older_remote_update_is_skipped_and_logged_as_conflict() {
    let db = test_db("sync-lww-older-target");
    let shared_root = existing_shared_root("sync-lww-older-root");
    let mut local_entry = sample_entry("1", "entry-lww-older", "Newer local");
    local_entry.updated_at = "2026-04-26T13:00:00.000Z".to_string();
    db.put_entry(&local_entry).unwrap();
    db.put_sync_value("meta:sync_bootstrap_complete", b"test")
        .unwrap();
    db.put_sync_entry_state(
        &local_entry.entry_uuid,
        &entry_state_for_test(&local_entry, "op-local-newer", false),
    )
    .unwrap();
    db.flush();

    let mut remote_entry = local_entry.clone();
    remote_entry.description = "Older remote".to_string();
    remote_entry.updated_at = "2026-04-26T12:00:00.000Z".to_string();
    let remote_operation = remote_upsert_operation(
        "remote-older-client",
        1,
        "op-remote-older",
        "2026-04-26T12:00:00.000Z",
        remote_entry,
    );
    write_remote_operation(&shared_root, &remote_operation);

    let before_revision = db.sync_revision().unwrap();
    let first = run_shared_sync_with_root(&db, &shared_root).unwrap();
    let after_first_revision = db.sync_revision().unwrap();
    let second = run_shared_sync_with_root(&db, &shared_root).unwrap();

    assert!(!first.entries_changed);
    assert!(!second.entries_changed);
    assert_eq!(
        db.find_entry("entry-lww-older")
            .unwrap()
            .unwrap()
            .description,
        "Newer local"
    );
    assert_eq!(conflict_count(&db), 1);
    assert_eq!(after_first_revision, before_revision);
    assert_eq!(db.sync_revision().unwrap(), after_first_revision);
}

#[test]
fn equal_timestamp_uses_op_id_tie_breaker() {
    let db = test_db("sync-lww-tie-target");
    let shared_root = existing_shared_root("sync-lww-tie-root");
    let mut local_entry = sample_entry("1", "entry-lww-tie", "Tie loser");
    local_entry.updated_at = "2026-04-26T12:00:00.000Z".to_string();
    db.put_entry(&local_entry).unwrap();
    db.put_sync_value("meta:sync_bootstrap_complete", b"test")
        .unwrap();
    db.put_sync_entry_state(
        &local_entry.entry_uuid,
        &entry_state_for_test(&local_entry, "op-m", false),
    )
    .unwrap();
    db.flush();

    let mut remote_entry = local_entry.clone();
    remote_entry.description = "Tie winner".to_string();
    remote_entry.updated_at = "2026-04-26T12:00:00.000Z".to_string();
    let remote_operation = remote_upsert_operation(
        "remote-tie-client",
        1,
        "op-z",
        "2026-04-26T12:00:00.000Z",
        remote_entry,
    );
    write_remote_operation(&shared_root, &remote_operation);

    assert!(
        run_shared_sync_with_root(&db, &shared_root)
            .unwrap()
            .entries_changed
    );
    assert_eq!(
        db.find_entry("entry-lww-tie").unwrap().unwrap().description,
        "Tie winner"
    );
}

#[test]
fn newer_delete_wins_and_older_upsert_after_delete_is_logged() {
    let db = test_db("sync-lww-delete-target");
    let shared_root = existing_shared_root("sync-lww-delete-root");
    let mut local_entry = sample_entry("1", "entry-lww-delete", "Delete target");
    local_entry.updated_at = "2026-04-26T12:00:00.000Z".to_string();
    db.put_entry(&local_entry).unwrap();
    db.put_sync_value("meta:sync_bootstrap_complete", b"test")
        .unwrap();
    db.put_sync_entry_state(
        &local_entry.entry_uuid,
        &entry_state_for_test(&local_entry, "op-local", false),
    )
    .unwrap();
    db.flush();

    let delete_operation = remote_delete_operation(
        "remote-delete-client",
        1,
        "op-remote-delete",
        "2026-04-26T13:00:00.000Z",
        "entry-lww-delete",
    );
    write_remote_operation(&shared_root, &delete_operation);

    let delete_result = run_shared_sync_with_root(&db, &shared_root).unwrap();
    assert!(delete_result.entries_changed);
    assert!(db.find_entry("entry-lww-delete").unwrap().is_none());
    assert!(db
        .sync_tombstone::<sync::SyncTombstoneRecord>("entry-lww-delete")
        .unwrap()
        .is_some());

    let mut old_upsert = local_entry.clone();
    old_upsert.description = "Older restore attempt".to_string();
    old_upsert.updated_at = "2026-04-26T12:30:00.000Z".to_string();
    let old_operation = remote_upsert_operation(
        "remote-delete-client",
        2,
        "op-remote-old-upsert",
        "2026-04-26T12:30:00.000Z",
        old_upsert,
    );
    write_remote_operation(&shared_root, &old_operation);

    let old_result = run_shared_sync_with_root(&db, &shared_root).unwrap();
    assert!(!old_result.entries_changed);
    assert!(db.find_entry("entry-lww-delete").unwrap().is_none());
    assert_eq!(conflict_count(&db), 1);
}

#[test]
fn newer_upsert_after_delete_restores_entry() {
    let db = test_db("sync-lww-restore-target");
    let shared_root = existing_shared_root("sync-lww-restore-root");
    db.put_sync_value("meta:sync_bootstrap_complete", b"test")
        .unwrap();
    db.put_sync_entry_state(
        "entry-lww-restore",
        &SyncEntryState {
            entry_uuid: "entry-lww-restore".to_string(),
            last_op_id: "op-delete".to_string(),
            mutation_ts_utc: "2026-04-26T13:00:00.000Z".to_string(),
            deleted: true,
            source_client_id: "remote-delete-client".to_string(),
            source_local_seq: 1,
            operation_type: SyncOperationType::InventoryEntryDelete,
            updated_at_utc: "2026-04-26T13:00:00.000Z".to_string(),
        },
    )
    .unwrap();
    db.put_sync_tombstone(
        "entry-lww-restore",
        &sync::SyncTombstoneRecord {
            entry_uuid: "entry-lww-restore".to_string(),
            deleted_at_utc: "2026-04-26T13:00:00.000Z".to_string(),
            op_id: "op-delete".to_string(),
            client_id: "remote-delete-client".to_string(),
            local_seq: 1,
            base_version: None,
        },
    )
    .unwrap();
    db.flush();

    let mut restored = sample_entry("1", "entry-lww-restore", "Newer restore");
    restored.updated_at = "2026-04-26T14:00:00.000Z".to_string();
    let restore_operation = remote_upsert_operation(
        "remote-restore-client",
        1,
        "op-remote-restore",
        "2026-04-26T14:00:00.000Z",
        restored,
    );
    write_remote_operation(&shared_root, &restore_operation);

    let result = run_shared_sync_with_root(&db, &shared_root).unwrap();

    assert!(result.entries_changed);
    assert_eq!(
        db.find_entry("entry-lww-restore")
            .unwrap()
            .unwrap()
            .description,
        "Newer restore"
    );
    assert!(db
        .sync_tombstone::<sync::SyncTombstoneRecord>("entry-lww-restore")
        .unwrap()
        .is_none());
}

fn first_outbox_operation(db: &InventoryDb) -> sync::SyncOperationEnvelope {
    let mut operation = None;
    db.scan_sync_outbox_records::<sync::SyncOperationEnvelope, _>(None, 1, |_, record| {
        operation = Some(record);
        Ok(false)
    })
    .unwrap();
    operation.unwrap()
}

fn corrupt_remote_count(db: &InventoryDb) -> usize {
    let mut count = 0;
    db.scan_sync_corrupt_records::<sync::CorruptRemoteFile, _>(usize::MAX, |_, _| {
        count += 1;
        Ok(true)
    })
    .unwrap();
    count
}

fn conflict_count(db: &InventoryDb) -> usize {
    let mut count = 0;
    db.scan_sync_conflict_records::<SyncConflictRecord, _>(usize::MAX, |_, _| {
        count += 1;
        Ok(true)
    })
    .unwrap();
    count
}

fn entry_state_for_test(entry: &InventoryEntry, op_id: &str, deleted: bool) -> SyncEntryState {
    SyncEntryState {
        entry_uuid: entry.entry_uuid.clone(),
        last_op_id: op_id.to_string(),
        mutation_ts_utc: entry.updated_at.clone(),
        deleted,
        source_client_id: "test-client".to_string(),
        source_local_seq: 1,
        operation_type: if deleted {
            SyncOperationType::InventoryEntryDelete
        } else {
            SyncOperationType::InventoryEntryUpdate
        },
        updated_at_utc: entry.updated_at.clone(),
    }
}

fn remote_upsert_operation(
    client_id: &str,
    local_seq: u64,
    op_id: &str,
    mutation_ts_utc: &str,
    mut entry: InventoryEntry,
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
        vec!["description".to_string()],
        None,
    )
    .unwrap();
    operation.op_id = op_id.to_string();
    operation.mutation_ts_utc = mutation_ts_utc.to_string();
    operation.created_at_utc = mutation_ts_utc.to_string();
    operation.checksum = canonical_operation_checksum(&operation).unwrap();
    operation
}

fn remote_delete_operation(
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

fn write_remote_operation(shared_root: &PathBuf, operation: &SyncOperationEnvelope) {
    let paths = SharedSyncPaths::from_shared_root(shared_root);
    let path = operation_file_path(&paths, &operation.client_id, operation.local_seq).unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, canonical_operation_json(operation).unwrap()).unwrap();
}

fn test_db(prefix: &str) -> InventoryDb {
    let root = unique_test_dir(prefix);
    fs::create_dir_all(&root).unwrap();
    InventoryDb::open_at(root.join("inventory.feox"), None).unwrap()
}

fn existing_shared_root(prefix: &str) -> PathBuf {
    let root = unique_test_dir(prefix);
    fs::create_dir_all(&root).unwrap();
    root
}

fn sample_entry(id: &str, entry_uuid: &str, description: &str) -> InventoryEntry {
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

fn unique_test_dir(prefix: &str) -> PathBuf {
    env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4().simple()))
}
