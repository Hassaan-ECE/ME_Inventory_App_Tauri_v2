use tauri::State;

use crate::{
    legacy_import,
    model::{
        create_entry_from_input, normalize_entry_input, now_timestamp, shared_status,
        update_entry_from_input, validate_entry_input, CommandResult,
        InventoryDeleteMutationResult, InventoryEntryInput, InventoryEntryMutationResult,
        InventoryQueryInput, InventoryQueryResult, InventorySyncResult, LegacyImportResult,
    },
    query::{get_inventory_counts, query_entries},
    store::InventoryDb,
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
pub(crate) fn sync_inventory(db: State<'_, InventoryDb>) -> CommandResult<InventorySyncResult> {
    Ok(InventorySyncResult {
        db_path: db.db_path_string(),
        entries: Vec::new(),
        entries_changed: Some(false),
        shared: shared_status("FeOxDB local store ready."),
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
        shared: shared_status(message),
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
        shared: shared_status(message),
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

    Ok(InventoryEntryMutationResult {
        entry,
        message: "Entry added to the ME Inventory database.".to_string(),
        mutation_mode: "local".to_string(),
        shared: shared_status("FeOxDB local store ready."),
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
    let entry = update_entry_from_input(existing, input);
    db.put_entry(&entry)?;

    Ok(InventoryEntryMutationResult {
        entry,
        message: "Entry updated in the ME Inventory database.".to_string(),
        mutation_mode: "local".to_string(),
        shared: shared_status("FeOxDB local store ready."),
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
    entry.verified_in_survey = next_verified;
    entry.updated_at = now_timestamp();
    db.put_entry(&entry)?;

    Ok(InventoryEntryMutationResult {
        entry,
        message: "Verified state updated.".to_string(),
        mutation_mode: "local".to_string(),
        shared: shared_status("FeOxDB local store ready."),
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
    entry.archived = archived;
    entry.updated_at = now_timestamp();
    db.put_entry(&entry)?;

    Ok(InventoryEntryMutationResult {
        entry,
        message: if archived {
            "Entry moved to the archive.".to_string()
        } else {
            "Entry restored to inventory.".to_string()
        },
        mutation_mode: "local".to_string(),
        shared: shared_status("FeOxDB local store ready."),
    })
}

fn delete_entry_in_store(
    entry_id: &str,
    db: &InventoryDb,
) -> CommandResult<InventoryDeleteMutationResult> {
    let entry = db
        .find_entry(entry_id)?
        .ok_or_else(|| "The selected entry could not be found.".to_string())?;
    db.delete_entry_by_uuid(&entry.entry_uuid)?;

    Ok(InventoryDeleteMutationResult {
        entry_id: entry.id,
        message: "Entry deleted.".to_string(),
        mutation_mode: "local".to_string(),
        shared: shared_status("FeOxDB local store ready."),
    })
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

        let deleted = delete_entry_in_store(&created.entry.entry_uuid, &db).unwrap();
        assert_eq!(deleted.entry_id, "1");
        assert!(db.load_entries().unwrap().is_empty());
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

    fn unique_test_dir(prefix: &str) -> PathBuf {
        env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4().simple()))
    }
}
