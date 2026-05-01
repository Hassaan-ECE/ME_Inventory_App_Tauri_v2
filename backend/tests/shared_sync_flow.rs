#[allow(dead_code)]
#[path = "../src/domain/model.rs"]
mod model;
#[allow(dead_code)]
#[path = "../src/storage/mod.rs"]
mod store;
#[allow(dead_code)]
#[path = "../src/sync/mod.rs"]
mod sync;

use std::{env, fs, path::PathBuf};

use model::InventoryEntry;
use store::InventoryDb;
use sync::{
    build_entry_operation, canonical_operation_checksum, canonical_operation_json,
    operation_file_path, run_shared_sync_with_root, SharedSyncPaths, SyncClientIdentity,
    SyncConflictRecord, SyncOperationEnvelope, SyncOperationType,
};
use uuid::Uuid;

#[test]
fn unavailable_shared_root_bootstraps_local_outbox_without_creating_root() {
    let db = test_db("sync-unavailable-root");
    db.put_entry(&sample_entry("1", "entry-offline", "Offline item"))
        .unwrap();
    db.flush();

    let missing_root = unique_test_dir("missing-shared-root");
    assert!(!missing_root.exists());

    let result = run_shared_sync_with_root(&db, &missing_root).unwrap();

    assert!(!result.shared.available);
    assert_eq!(result.shared.enabled, true);
    assert_eq!(result.shared.can_modify, true);
    assert_eq!(result.shared.has_local_only_changes, Some(true));
    assert!(result
        .shared
        .message
        .contains("Shared workspace unavailable"));
    assert!(!missing_root.exists());
    assert_eq!(outbox_count(&db), 1);
}

#[test]
fn bootstrap_writes_one_operation_per_existing_entry_once() {
    let db = test_db("sync-bootstrap");
    db.put_entry(&sample_entry("1", "entry-a", "A")).unwrap();
    db.put_entry(&sample_entry("2", "entry-b", "B")).unwrap();
    db.set_next_entry_id(3).unwrap();
    db.flush();
    let shared_root = existing_shared_root("sync-bootstrap-root");

    let first = run_shared_sync_with_root(&db, &shared_root).unwrap();
    assert_eq!(first.shared.available, true);
    assert!(manifest_path(&shared_root).exists());
    assert_eq!(snapshot_file_count(&shared_root), 1);
    assert_eq!(operation_file_count(&shared_root), 0);

    let second = run_shared_sync_with_root(&db, &shared_root).unwrap();
    assert_eq!(second.shared.available, true);
    assert_eq!(operation_file_count(&shared_root), 0);
    assert_eq!(outbox_count(&db), 2);
}

#[test]
fn fresh_database_hydrates_from_snapshot_then_newer_operations() {
    let db_source = test_db("sync-snapshot-source");
    let shared_root = existing_shared_root("sync-snapshot-root");
    let mut entry = sample_entry("1", "entry-snapshot", "Snapshot seed");
    entry.updated_at = "2026-04-26T09:00:00.000Z".to_string();
    db_source.put_entry(&entry).unwrap();
    db_source.set_next_entry_id(2).unwrap();
    db_source.flush();

    run_shared_sync_with_root(&db_source, &shared_root).unwrap();
    assert!(manifest_path(&shared_root).exists());
    assert_eq!(operation_file_count(&shared_root), 0);

    let mut newer = entry.clone();
    newer.description = "Newer operation after snapshot".to_string();
    newer.updated_at = "2026-04-26T10:00:00.000Z".to_string();
    let newer_operation = remote_upsert_operation(
        "snapshot-newer-client",
        1,
        "op-snapshot-newer",
        "2026-04-26T10:00:00.000Z",
        newer,
    );
    write_remote_operation(&shared_root, &newer_operation);

    let db_target = test_db("sync-snapshot-target");
    let result = run_shared_sync_with_root(&db_target, &shared_root).unwrap();

    assert!(result.entries_changed);
    assert_eq!(
        db_target
            .find_entry("entry-snapshot")
            .unwrap()
            .unwrap()
            .description,
        "Newer operation after snapshot"
    );
    assert!(db_target.last_snapshot_id().unwrap().is_some());
    assert_eq!(
        db_target.sync_watermark("snapshot-newer-client").unwrap(),
        Some(1)
    );
}

#[test]
fn corrupt_snapshot_manifest_does_not_replace_local_entries() {
    let db = test_db("sync-corrupt-snapshot");
    db.put_entry(&sample_entry("1", "entry-local", "Local survives"))
        .unwrap();
    db.set_next_entry_id(2).unwrap();
    db.flush();
    let shared_root = existing_shared_root("sync-corrupt-snapshot-root");
    let manifest = manifest_path(&shared_root);
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    fs::write(&manifest, b"{not json").unwrap();

    let result = run_shared_sync_with_root(&db, &shared_root).unwrap();

    assert!(result.shared.message.contains("corrupt"));
    assert_eq!(
        db.find_entry("entry-local").unwrap().unwrap().description,
        "Local survives"
    );
}

#[test]
fn two_databases_push_and_pull_create_update_and_delete() {
    let db_a = test_db("sync-db-a");
    let db_b = test_db("sync-db-b");
    let shared_root = existing_shared_root("sync-two-db-root");

    run_shared_sync_with_root(&db_a, &shared_root).unwrap();
    run_shared_sync_with_root(&db_b, &shared_root).unwrap();

    let entry = sample_entry("1", "entry-shared", "Created on A");
    db_a.put_entry(&entry).unwrap();
    db_a.set_next_entry_id(2).unwrap();
    sync::queue_entry_operation(
        &db_a,
        SyncOperationType::InventoryEntryCreate,
        entry.clone(),
        Vec::new(),
        None,
    )
    .unwrap();
    db_a.flush();

    run_shared_sync_with_root(&db_a, &shared_root).unwrap();
    let pulled_create = run_shared_sync_with_root(&db_b, &shared_root).unwrap();
    assert!(pulled_create.entries_changed);
    assert_eq!(
        db_b.find_entry("entry-shared")
            .unwrap()
            .unwrap()
            .description,
        "Created on A"
    );

    let mut updated = db_b.find_entry("entry-shared").unwrap().unwrap();
    let base_version = Some(updated.updated_at.clone());
    updated.description = "Updated on B".to_string();
    updated.updated_at = "2026-04-26T12:00:00.000Z".to_string();
    db_b.put_entry(&updated).unwrap();
    sync::queue_entry_operation(
        &db_b,
        SyncOperationType::InventoryEntryUpdate,
        updated.clone(),
        vec!["description".to_string()],
        base_version,
    )
    .unwrap();
    db_b.flush();

    run_shared_sync_with_root(&db_b, &shared_root).unwrap();
    let pulled_update = run_shared_sync_with_root(&db_a, &shared_root).unwrap();
    assert!(pulled_update.entries_changed);
    assert_eq!(
        db_a.find_entry("entry-shared")
            .unwrap()
            .unwrap()
            .description,
        "Updated on B"
    );

    let entry_to_delete = db_a.find_entry("entry-shared").unwrap().unwrap();
    sync::queue_delete_operation(
        &db_a,
        entry_to_delete.entry_uuid.clone(),
        "2026-04-26T13:00:00.000Z",
        Some(entry_to_delete.updated_at.clone()),
    )
    .unwrap();
    db_a.delete_entry(&entry_to_delete).unwrap();
    db_a.flush();

    run_shared_sync_with_root(&db_a, &shared_root).unwrap();
    let pulled_delete = run_shared_sync_with_root(&db_b, &shared_root).unwrap();
    assert!(pulled_delete.entries_changed);
    assert!(db_b.find_entry("entry-shared").unwrap().is_none());
    assert!(db_b
        .sync_tombstone::<sync::SyncTombstoneRecord>("entry-shared")
        .unwrap()
        .is_some());
}

#[test]
fn repeated_pull_ignores_already_applied_operation() {
    let db_source = test_db("sync-idempotent-source");
    let db_target = test_db("sync-idempotent-target");
    let shared_root = existing_shared_root("sync-idempotent-root");

    run_shared_sync_with_root(&db_source, &shared_root).unwrap();
    run_shared_sync_with_root(&db_target, &shared_root).unwrap();

    let entry = sample_entry("1", "entry-idempotent", "Idempotent");
    db_source.put_entry(&entry).unwrap();
    sync::queue_entry_operation(
        &db_source,
        SyncOperationType::InventoryEntryCreate,
        entry,
        Vec::new(),
        None,
    )
    .unwrap();
    db_source.flush();
    run_shared_sync_with_root(&db_source, &shared_root).unwrap();

    assert!(
        run_shared_sync_with_root(&db_target, &shared_root)
            .unwrap()
            .entries_changed
    );
    assert!(
        !run_shared_sync_with_root(&db_target, &shared_root)
            .unwrap()
            .entries_changed
    );
}

#[test]
fn pushed_local_operations_advance_watermark_for_fast_repeated_sync() {
    let db = test_db("sync-local-watermark");
    let shared_root = existing_shared_root("sync-local-watermark-root");

    run_shared_sync_with_root(&db, &shared_root).unwrap();

    let entry = sample_entry("1", "entry-watermark", "Watermark");
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
    let operation = read_outbox_operation(&db, 1);

    let result = run_shared_sync_with_root(&db, &shared_root).unwrap();

    assert_eq!(
        db.sync_watermark(&operation.client_id).unwrap(),
        Some(operation.local_seq)
    );
    assert_eq!(result.shared.has_local_only_changes, Some(false));
    assert!(
        !run_shared_sync_with_root(&db, &shared_root)
            .unwrap()
            .entries_changed
    );
}

#[test]
fn local_mutation_increments_sync_revision() {
    let db = test_db("sync-local-revision");
    let entry = sample_entry("1", "entry-local-revision", "Revision");
    db.put_entry(&entry).unwrap();
    let before = db.sync_revision().unwrap();

    sync::queue_entry_operation(
        &db,
        SyncOperationType::InventoryEntryCreate,
        entry,
        Vec::new(),
        None,
    )
    .unwrap();

    assert_eq!(db.sync_revision().unwrap(), before + 1);
}

#[test]
fn scripted_one_machine_smoke_uses_env_shared_root() {
    let Some(smoke_root) = env::var_os("ME_SYNC_SMOKE_ROOT").map(PathBuf::from) else {
        println!("ME_SYNC_SMOKE_ROOT is not set; skipping script-only smoke scenario.");
        return;
    };
    fs::create_dir_all(&smoke_root).unwrap();
    let shared_root = smoke_root.join("shared-root");
    fs::create_dir_all(&shared_root).unwrap();
    let db_a = InventoryDb::open_at(smoke_root.join("client-a").join("inventory.feox")).unwrap();
    let db_b = InventoryDb::open_at(smoke_root.join("client-b").join("inventory.feox")).unwrap();

    run_shared_sync_with_root(&db_a, &shared_root).unwrap();
    run_shared_sync_with_root(&db_b, &shared_root).unwrap();

    let mut created = sample_entry("1", "entry-smoke-sync", "Created on client A");
    created.updated_at = "2026-04-26T10:00:00.000Z".to_string();
    db_a.put_entry(&created).unwrap();
    db_a.set_next_entry_id(2).unwrap();
    sync::queue_entry_operation(
        &db_a,
        SyncOperationType::InventoryEntryCreate,
        created.clone(),
        Vec::new(),
        None,
    )
    .unwrap();
    db_a.flush();
    run_shared_sync_with_root(&db_a, &shared_root).unwrap();
    assert!(
        run_shared_sync_with_root(&db_b, &shared_root)
            .unwrap()
            .entries_changed
    );

    let mut updated = db_b.find_entry("entry-smoke-sync").unwrap().unwrap();
    updated.description = "Updated on client B".to_string();
    updated.updated_at = "2026-04-26T11:00:00.000Z".to_string();
    db_b.put_entry(&updated).unwrap();
    sync::queue_entry_operation(
        &db_b,
        SyncOperationType::InventoryEntryUpdate,
        updated.clone(),
        vec!["description".to_string()],
        Some("2026-04-26T10:00:00.000Z".to_string()),
    )
    .unwrap();
    db_b.flush();
    run_shared_sync_with_root(&db_b, &shared_root).unwrap();
    assert!(
        run_shared_sync_with_root(&db_a, &shared_root)
            .unwrap()
            .entries_changed
    );

    let mut stale = updated.clone();
    stale.description = "Stale update should lose".to_string();
    stale.updated_at = "2026-04-26T10:30:00.000Z".to_string();
    let stale_operation = remote_upsert_operation(
        "smoke-stale-client",
        1,
        "op-smoke-stale",
        "2026-04-26T10:30:00.000Z",
        stale,
    );
    write_remote_operation(&shared_root, &stale_operation);
    run_shared_sync_with_root(&db_a, &shared_root).unwrap();
    assert_eq!(conflict_count(&db_a), 1);

    let deleted = db_a.find_entry("entry-smoke-sync").unwrap().unwrap();
    sync::queue_delete_operation(
        &db_a,
        &deleted.entry_uuid,
        "2026-04-26T12:00:00.000Z",
        Some(deleted.updated_at.clone()),
    )
    .unwrap();
    db_a.delete_entry(&deleted).unwrap();
    db_a.flush();
    run_shared_sync_with_root(&db_a, &shared_root).unwrap();
    assert!(
        run_shared_sync_with_root(&db_b, &shared_root)
            .unwrap()
            .entries_changed
    );
    assert!(db_b.find_entry("entry-smoke-sync").unwrap().is_none());

    let mut restored = sample_entry("1", "entry-smoke-sync", "Restored on client B");
    restored.updated_at = "2026-04-26T13:00:00.000Z".to_string();
    db_b.put_entry(&restored).unwrap();
    sync::queue_entry_operation(
        &db_b,
        SyncOperationType::InventoryEntryUpdate,
        restored.clone(),
        Vec::new(),
        Some("2026-04-26T12:00:00.000Z".to_string()),
    )
    .unwrap();
    db_b.flush();
    run_shared_sync_with_root(&db_b, &shared_root).unwrap();
    assert!(
        run_shared_sync_with_root(&db_a, &shared_root)
            .unwrap()
            .entries_changed
    );

    assert_eq!(
        db_a.find_entry("entry-smoke-sync")
            .unwrap()
            .unwrap()
            .description,
        "Restored on client B"
    );
    assert_eq!(
        db_b.find_entry("entry-smoke-sync")
            .unwrap()
            .unwrap()
            .description,
        "Restored on client B"
    );

    let ops_dir = shared_root.join("shared").join("inventory").join("ops");
    println!("shared_ops={}", ops_dir.display());
    println!("client_a_conflicts={}", conflict_count(&db_a));
    println!("client_a_revision={}", db_a.sync_revision().unwrap());
    println!("client_b_revision={}", db_b.sync_revision().unwrap());
    println!("PASS one-machine sync smoke converged");
}

fn outbox_count(db: &InventoryDb) -> usize {
    let mut count = 0;
    db.scan_sync_outbox_records::<sync::SyncOperationEnvelope, _>(None, usize::MAX, |_, _| {
        count += 1;
        Ok(true)
    })
    .unwrap();
    count
}

fn read_outbox_operation(db: &InventoryDb, local_seq: u64) -> sync::SyncOperationEnvelope {
    db.sync_outbox_record(local_seq).unwrap().unwrap()
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

fn write_remote_operation(shared_root: &PathBuf, operation: &SyncOperationEnvelope) {
    let paths = SharedSyncPaths::from_shared_root(shared_root);
    let path = operation_file_path(&paths, &operation.client_id, operation.local_seq).unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, canonical_operation_json(operation).unwrap()).unwrap();
}

fn operation_file_count(shared_root: &PathBuf) -> usize {
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

fn manifest_path(shared_root: &PathBuf) -> PathBuf {
    shared_root
        .join("shared")
        .join("inventory")
        .join("manifest.json")
}

fn snapshot_file_count(shared_root: &PathBuf) -> usize {
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

fn test_db(prefix: &str) -> InventoryDb {
    let root = unique_test_dir(prefix);
    fs::create_dir_all(&root).unwrap();
    InventoryDb::open_at(root.join("inventory.feox")).unwrap()
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
