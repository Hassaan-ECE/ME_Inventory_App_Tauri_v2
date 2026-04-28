use tauri::{AppHandle, State};

use crate::{
    legacy_import,
    model::{
        create_entry_from_input, normalize_entry_input, now_timestamp, update_entry_from_input,
        validate_entry_input, CommandResult, InventoryDeleteMutationResult, InventoryEntry,
        InventoryEntryInput, InventoryEntryMutationResult, InventoryQueryInput,
        InventoryQueryResult, InventorySharedStatus, InventorySyncResult, LegacyImportResult,
    },
    query::{get_inventory_counts, query_entries},
    shared_watcher::SharedSyncWatcher,
    store::InventoryDb,
    sync::{self, SyncOperationType},
};

#[tauri::command]
pub(crate) fn load_inventory(db: State<'_, InventoryDb>) -> CommandResult<InventorySyncResult> {
    load_inventory_from_store(&db)
}

#[tauri::command]
pub(crate) fn query_inventory(
    input: InventoryQueryInput,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryQueryResult> {
    query_inventory_from_store(input, &db)
}

#[tauri::command]
pub(crate) async fn sync_inventory(
    app: AppHandle,
    watcher: State<'_, SharedSyncWatcher>,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventorySyncResult> {
    let db = db.inner().clone();
    let (result, entries, db_path) = tauri::async_runtime::spawn_blocking(move || {
        let result = sync::run_shared_sync(&db)?;
        let entries = if result.entries_changed {
            db.load_entries()?
        } else {
            Vec::new()
        };

        Ok::<_, String>((result, entries, db.db_path_string()))
    })
    .await
    .map_err(|error| format!("Shared sync task failed: {error}"))??;

    if result.shared.available {
        let paths = sync::resolved_shared_sync_paths();
        watcher.ensure_watching(app, &paths.ops_dir)?;
    }

    Ok(InventorySyncResult {
        db_path,
        entries,
        entries_changed: Some(result.entries_changed),
        shared: result.shared,
    })
}

#[tauri::command]
pub(crate) fn create_entry(
    input: InventoryEntryInput,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryEntryMutationResult> {
    create_entry_in_store(input, &db)
}

#[tauri::command]
pub(crate) fn update_entry(
    entry_id: String,
    input: InventoryEntryInput,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryEntryMutationResult> {
    update_entry_in_store(&entry_id, input, &db)
}

#[tauri::command]
pub(crate) fn toggle_verified_entry(
    entry_id: String,
    next_verified: bool,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryEntryMutationResult> {
    toggle_verified_entry_in_store(&entry_id, next_verified, &db)
}

#[tauri::command]
pub(crate) fn set_archived_entry(
    entry_id: String,
    archived: bool,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryEntryMutationResult> {
    set_archived_entry_in_store(&entry_id, archived, &db)
}

#[tauri::command]
pub(crate) fn delete_entry(
    entry_id: String,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryDeleteMutationResult> {
    delete_entry_in_store(&entry_id, &db)
}

#[tauri::command]
pub(crate) fn import_legacy_sqlite(
    db: State<'_, InventoryDb>,
) -> CommandResult<LegacyImportResult> {
    legacy_import::import_legacy_sqlite(&db)
}

fn load_inventory_from_store(db: &InventoryDb) -> CommandResult<InventorySyncResult> {
    let imported = legacy_import::ensure_legacy_imported(db)?;
    let entries = db.load_entries()?;
    let message = if imported > 0 {
        format!("Imported {imported} entries from legacy SQLite into FeOxDB.")
    } else if entries.is_empty() && db.legacy_sqlite_path().is_none() {
        "Legacy SQLite database was not found. FeOxDB is ready for new entries.".to_string()
    } else {
        "FeOxDB local store ready.".to_string()
    };

    Ok(InventorySyncResult {
        db_path: db.db_path_string(),
        entries,
        entries_changed: Some(true),
        shared: sync::shared_inventory_status(db, message),
    })
}

fn query_inventory_from_store(
    input: InventoryQueryInput,
    db: &InventoryDb,
) -> CommandResult<InventoryQueryResult> {
    let imported = legacy_import::ensure_legacy_imported(db)?;
    let all_entries = db.load_entries()?;
    let counts = get_inventory_counts(&all_entries);
    let (entries, total_filtered) = query_entries(&all_entries, input);
    let message = if imported > 0 {
        format!("Imported {imported} entries from legacy SQLite into FeOxDB.")
    } else {
        "FeOxDB local store ready.".to_string()
    };

    Ok(InventoryQueryResult {
        counts,
        db_path: db.db_path_string(),
        entries,
        shared: sync::shared_inventory_status(db, message),
        total_filtered,
    })
}

fn create_entry_in_store(
    input: InventoryEntryInput,
    db: &InventoryDb,
) -> CommandResult<InventoryEntryMutationResult> {
    let input = normalize_entry_input(input);
    validate_entry_input(&input)?;

    let id = db.next_entry_id()?;
    let entry = create_entry_from_input(id, input);
    db.put_entry(&entry)?;
    db.set_next_entry_id(id + 1)?;
    let sync_state = match queue_entry_sync_operation_before_flush(
        db,
        SyncOperationType::InventoryEntryCreate,
        entry.clone(),
        Vec::new(),
        None,
    ) {
        Ok(sync_state) => sync_state,
        Err(error) => {
            let _ = db.delete_entry(&entry);
            let _ = db.set_next_entry_id(id);
            db.flush();
            return Err(error);
        }
    };
    db.flush();

    Ok(InventoryEntryMutationResult {
        entry,
        message: "Entry added to the ME Inventory database.".to_string(),
        mutation_mode: sync_state.mutation_mode,
        shared: sync_state.shared,
    })
}

fn update_entry_in_store(
    entry_id: &str,
    input: InventoryEntryInput,
    db: &InventoryDb,
) -> CommandResult<InventoryEntryMutationResult> {
    let input = normalize_entry_input(input);
    validate_entry_input(&input)?;

    let existing = db
        .find_entry(entry_id)?
        .ok_or_else(|| "The selected entry could not be found.".to_string())?;
    let base_version = entry_base_version(&existing);
    let entry = update_entry_from_input(existing.clone(), input);
    let changed_fields = changed_entry_fields(&existing, &entry);
    db.put_entry(&entry)?;
    let sync_state = match queue_entry_sync_operation_before_flush(
        db,
        SyncOperationType::InventoryEntryUpdate,
        entry.clone(),
        changed_fields,
        base_version,
    ) {
        Ok(sync_state) => sync_state,
        Err(error) => {
            let _ = db.put_entry(&existing);
            db.flush();
            return Err(error);
        }
    };
    db.flush();

    Ok(InventoryEntryMutationResult {
        entry,
        message: "Entry updated in the ME Inventory database.".to_string(),
        mutation_mode: sync_state.mutation_mode,
        shared: sync_state.shared,
    })
}

fn toggle_verified_entry_in_store(
    entry_id: &str,
    next_verified: bool,
    db: &InventoryDb,
) -> CommandResult<InventoryEntryMutationResult> {
    let mut entry = db
        .find_entry(entry_id)?
        .ok_or_else(|| "The selected entry could not be found.".to_string())?;
    let base_version = entry_base_version(&entry);
    let existing = entry.clone();
    entry.verified_in_survey = next_verified;
    entry.updated_at = now_timestamp();
    db.put_entry(&entry)?;
    let sync_state = match queue_entry_sync_operation_before_flush(
        db,
        SyncOperationType::InventoryEntryVerify,
        entry.clone(),
        vec!["verified_in_survey".to_string()],
        base_version,
    ) {
        Ok(sync_state) => sync_state,
        Err(error) => {
            let _ = db.put_entry(&existing);
            db.flush();
            return Err(error);
        }
    };
    db.flush();

    Ok(InventoryEntryMutationResult {
        entry,
        message: "Verified state updated.".to_string(),
        mutation_mode: sync_state.mutation_mode,
        shared: sync_state.shared,
    })
}

fn set_archived_entry_in_store(
    entry_id: &str,
    archived: bool,
    db: &InventoryDb,
) -> CommandResult<InventoryEntryMutationResult> {
    let mut entry = db
        .find_entry(entry_id)?
        .ok_or_else(|| "The selected entry could not be found.".to_string())?;
    let base_version = entry_base_version(&entry);
    let existing = entry.clone();
    entry.archived = archived;
    entry.updated_at = now_timestamp();
    db.put_entry(&entry)?;
    let sync_state = match queue_entry_sync_operation_before_flush(
        db,
        SyncOperationType::InventoryEntryArchive,
        entry.clone(),
        vec!["archived".to_string()],
        base_version,
    ) {
        Ok(sync_state) => sync_state,
        Err(error) => {
            let _ = db.put_entry(&existing);
            db.flush();
            return Err(error);
        }
    };
    db.flush();

    Ok(InventoryEntryMutationResult {
        entry,
        message: if archived {
            "Entry moved to the archive.".to_string()
        } else {
            "Entry restored to inventory.".to_string()
        },
        mutation_mode: sync_state.mutation_mode,
        shared: sync_state.shared,
    })
}

fn delete_entry_in_store(
    entry_id: &str,
    db: &InventoryDb,
) -> CommandResult<InventoryDeleteMutationResult> {
    let entry = db
        .find_entry(entry_id)?
        .ok_or_else(|| "The selected entry could not be found.".to_string())?;
    let deleted_at_utc = now_timestamp();
    db.delete_entry(&entry)?;
    let sync_state = match queue_delete_sync_operation_before_flush(
        db,
        &entry.entry_uuid,
        deleted_at_utc,
        entry_base_version(&entry),
    ) {
        Ok(sync_state) => sync_state,
        Err(error) => {
            let _ = db.put_entry(&entry);
            db.flush();
            return Err(error);
        }
    };
    db.flush();

    Ok(InventoryDeleteMutationResult {
        entry_id: entry.id,
        message: "Entry deleted.".to_string(),
        mutation_mode: sync_state.mutation_mode,
        shared: sync_state.shared,
    })
}

#[derive(Debug, Clone)]
struct QueuedMutationState {
    mutation_mode: String,
    shared: InventorySharedStatus,
}

fn queue_entry_sync_operation_before_flush(
    db: &InventoryDb,
    operation_type: SyncOperationType,
    entry: InventoryEntry,
    changed_fields: Vec<String>,
    base_version: Option<String>,
) -> CommandResult<QueuedMutationState> {
    sync::queue_entry_operation(db, operation_type, entry, changed_fields, base_version)?;

    Ok(QueuedMutationState {
        mutation_mode: "local".to_string(),
        shared: sync::queued_local_status(db),
    })
}

fn queue_delete_sync_operation_before_flush(
    db: &InventoryDb,
    entry_uuid: &str,
    deleted_at_utc: String,
    base_version: Option<String>,
) -> CommandResult<QueuedMutationState> {
    sync::queue_delete_operation(db, entry_uuid, deleted_at_utc, base_version)?;

    Ok(QueuedMutationState {
        mutation_mode: "local".to_string(),
        shared: sync::queued_local_status(db),
    })
}

fn entry_base_version(entry: &InventoryEntry) -> Option<String> {
    (!entry.updated_at.is_empty()).then(|| entry.updated_at.clone())
}

fn changed_entry_fields(before: &InventoryEntry, after: &InventoryEntry) -> Vec<String> {
    let mut fields = Vec::new();

    if before.asset_number != after.asset_number {
        fields.push("asset_number".to_string());
    }
    if before.serial_number != after.serial_number {
        fields.push("serial_number".to_string());
    }
    if before.qty != after.qty {
        fields.push("qty".to_string());
    }
    if before.manufacturer != after.manufacturer {
        fields.push("manufacturer".to_string());
    }
    if before.model != after.model {
        fields.push("model".to_string());
    }
    if before.description != after.description {
        fields.push("description".to_string());
    }
    if before.project_name != after.project_name {
        fields.push("project_name".to_string());
    }
    if before.location != after.location {
        fields.push("location".to_string());
    }
    if before.assigned_to != after.assigned_to {
        fields.push("assigned_to".to_string());
    }
    if before.links != after.links {
        fields.push("links".to_string());
    }
    if before.notes != after.notes {
        fields.push("notes".to_string());
    }
    if before.lifecycle_status != after.lifecycle_status {
        fields.push("lifecycle_status".to_string());
    }
    if before.working_status != after.working_status {
        fields.push("working_status".to_string());
    }
    if before.condition != after.condition {
        fields.push("condition".to_string());
    }
    if before.verified_in_survey != after.verified_in_survey {
        fields.push("verified_in_survey".to_string());
    }
    if before.archived != after.archived {
        fields.push("archived".to_string());
    }
    if before.picture_path != after.picture_path {
        fields.push("picture_path".to_string());
    }

    fields
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn create_entry_assigns_incrementing_ids() {
        let db = test_db();

        let first = create_entry_in_store(test_input("First"), &db).unwrap();
        let second = create_entry_in_store(test_input("Second"), &db).unwrap();

        assert_eq!(first.entry.id, "1");
        assert_eq!(second.entry.id, "2");
        assert_eq!(db.load_entries().unwrap().len(), 2);
        assert_local_outbox_status(&first.mutation_mode, &first.shared);
        assert_local_outbox_status(&second.mutation_mode, &second.shared);

        let first_op = read_outbox_operation(&db, 1);
        let second_op = read_outbox_operation(&db, 2);
        assert_eq!(
            first_op.operation_type,
            SyncOperationType::InventoryEntryCreate
        );
        assert_eq!(first_op.entity_id, first.entry.entry_uuid);
        assert_eq!(first_op.payload.entry.unwrap().description, "First");
        assert!(first_op.payload.changed_fields.is_empty());
        assert_eq!(
            second_op.operation_type,
            SyncOperationType::InventoryEntryCreate
        );
        assert_eq!(second_op.entity_id, second.entry.entry_uuid);
        assert_eq!(db.next_local_seq().unwrap(), 3);
        assert_eq!(
            db.sync_client_seq_marker::<String>(&first_op.client_id, first_op.local_seq)
                .unwrap()
                .unwrap(),
            first_op.op_id
        );
        assert!(db.has_sync_applied_marker(&second_op.op_id).unwrap());
    }

    #[test]
    fn update_and_delete_missing_entries_return_errors() {
        let db = test_db();

        assert!(update_entry_in_store("missing", test_input("Missing"), &db)
            .unwrap_err()
            .contains("could not be found"));
        assert!(delete_entry_in_store("missing", &db)
            .unwrap_err()
            .contains("could not be found"));
    }

    #[test]
    fn update_and_delete_entries_by_uuid() {
        let db = test_db();
        let created = create_entry_in_store(test_input("Original"), &db).unwrap();

        let updated = update_entry_in_store(
            &created.entry.entry_uuid,
            InventoryEntryInput {
                description: "Updated".to_string(),
                ..test_input("Original")
            },
            &db,
        )
        .unwrap();
        assert_eq!(updated.entry.description, "Updated");
        assert_local_outbox_status(&updated.mutation_mode, &updated.shared);

        let deleted = delete_entry_in_store(&created.entry.entry_uuid, &db).unwrap();
        assert_eq!(deleted.entry_id, "1");
        assert_local_outbox_status(&deleted.mutation_mode, &deleted.shared);
        assert!(db.load_entries().unwrap().is_empty());
    }

    #[test]
    fn verify_archive_and_delete_mutations_report_queued_status() {
        let db = test_db();
        let created = create_entry_in_store(test_input("Original"), &db).unwrap();

        let verified =
            toggle_verified_entry_in_store(&created.entry.entry_uuid, true, &db).unwrap();
        assert!(verified.entry.verified_in_survey);
        assert_local_outbox_status(&verified.mutation_mode, &verified.shared);

        let archived = set_archived_entry_in_store(&created.entry.entry_uuid, true, &db).unwrap();
        assert!(archived.entry.archived);
        assert_local_outbox_status(&archived.mutation_mode, &archived.shared);

        let deleted = delete_entry_in_store(&created.entry.entry_uuid, &db).unwrap();
        assert_eq!(deleted.entry_id, created.entry.id);
        assert_local_outbox_status(&deleted.mutation_mode, &deleted.shared);
    }

    #[test]
    fn delete_sync_mutation_uses_tombstone_payload() {
        let db = test_db();
        let created = create_entry_in_store(test_input("Tombstone"), &db).unwrap();

        delete_entry_in_store(&created.entry.entry_uuid, &db).unwrap();
        let delete_op = read_outbox_operation(&db, 2);
        let tombstone = db
            .sync_tombstone::<sync::SyncTombstoneRecord>(&created.entry.entry_uuid)
            .unwrap()
            .unwrap();

        assert_eq!(
            delete_op.operation_type,
            SyncOperationType::InventoryEntryDelete
        );
        assert_eq!(delete_op.entity_id, created.entry.entry_uuid);
        assert!(delete_op.payload.entry.is_none());
        assert_eq!(
            delete_op.payload.entry_uuid.as_deref(),
            Some(created.entry.entry_uuid.as_str())
        );
        assert_eq!(tombstone.entry_uuid, created.entry.entry_uuid);
        assert_eq!(tombstone.op_id, delete_op.op_id);
        assert_eq!(tombstone.local_seq, delete_op.local_seq);
    }

    #[test]
    fn update_sync_mutation_tracks_changed_fields_and_projection() {
        let db = test_db();
        let created = create_entry_in_store(test_input("Original"), &db).unwrap();
        let updated = update_entry_from_input(
            created.entry.clone(),
            InventoryEntryInput {
                description: "Updated".to_string(),
                location: "Lab A".to_string(),
                ..test_input("Original")
            },
        );
        let changed_fields = changed_entry_fields(&created.entry, &updated);

        sync::queue_entry_operation(
            &db,
            SyncOperationType::InventoryEntryUpdate,
            updated.clone(),
            changed_fields,
            entry_base_version(&created.entry),
        )
        .unwrap();
        let operation = read_outbox_operation(&db, 2);

        assert_eq!(
            operation.operation_type,
            SyncOperationType::InventoryEntryUpdate
        );
        assert_eq!(
            operation.base_version.as_deref(),
            Some(created.entry.updated_at.as_str())
        );
        assert_eq!(
            operation.payload.changed_fields,
            vec!["description".to_string(), "location".to_string()]
        );
        let payload_entry = operation.payload.entry.unwrap();
        assert_eq!(payload_entry.description, "Updated");
        assert_eq!(payload_entry.location, "Lab A");
    }

    fn test_input(description: &str) -> InventoryEntryInput {
        InventoryEntryInput {
            description: description.to_string(),
            lifecycle_status: "active".to_string(),
            working_status: "unknown".to_string(),
            ..InventoryEntryInput::default()
        }
    }

    fn test_db() -> InventoryDb {
        let root = unique_test_dir("commands");
        fs::create_dir_all(&root).unwrap();
        InventoryDb::open_at(root.join("inventory.feox"), None).unwrap()
    }

    fn assert_local_outbox_status(mutation_mode: &str, shared: &InventorySharedStatus) {
        assert_eq!(mutation_mode, "local");
        assert_eq!(shared.has_local_only_changes, Some(true));
        assert!(shared.message.contains("queued for shared sync"));
        assert_eq!(shared.mutation_mode, "local");
    }

    fn read_outbox_operation(db: &InventoryDb, local_seq: u64) -> sync::SyncOperationEnvelope {
        db.sync_outbox_record(local_seq).unwrap().unwrap()
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4().simple()))
    }
}
