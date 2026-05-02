use tauri::{AppHandle, Emitter, State};

use super::mutations::{
    create_entry_in_store, delete_entry_in_store, set_archived_entry_in_store,
    toggle_verified_entry_in_store, update_entry_in_store,
};
use crate::{
    model::{
        CommandResult, InventoryDeleteMutationResult, InventoryEntryEditContext,
        InventoryEntryInput, InventoryEntryMutationResult, InventoryQueryInput,
        InventoryQueryResult, InventorySharedStatus, InventorySyncResult,
    },
    query::{get_inventory_counts, query_entries},
    shared_sync::SharedSyncCoordinator,
    shared_watcher::{self, SharedSyncWatcher},
    store::InventoryDb,
    sync,
};

#[tauri::command]
pub(crate) fn load_inventory(
    coordinator: State<'_, SharedSyncCoordinator>,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventorySyncResult> {
    load_inventory_from_store_with_status(&db, coordinator.background_status()?)
}

#[tauri::command]
pub(crate) fn query_inventory(
    input: InventoryQueryInput,
    coordinator: State<'_, SharedSyncCoordinator>,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryQueryResult> {
    query_inventory_from_store_with_status(input, &db, coordinator.background_status()?)
}

#[tauri::command]
pub(crate) async fn sync_inventory(
    app: AppHandle,
    coordinator: State<'_, SharedSyncCoordinator>,
    watcher: State<'_, SharedSyncWatcher>,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventorySyncResult> {
    let coordinator = coordinator.inner().clone();
    let db = db.inner().clone();
    let (result, entries, db_path) = tauri::async_runtime::spawn_blocking(move || {
        let result = coordinator.run_exclusive("shared sync", || sync::run_shared_sync(&db))?;
        let _ = coordinator.set_background_status(result.shared.clone());
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
    app: AppHandle,
    input: InventoryEntryInput,
    coordinator: State<'_, SharedSyncCoordinator>,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryEntryMutationResult> {
    let coordinator = coordinator.inner().clone();
    let result =
        coordinator.run_exclusive("inventory create", || create_entry_in_store(input, &db))?;
    schedule_shared_publish(app, db.inner().clone(), coordinator);
    Ok(result)
}

#[tauri::command]
pub(crate) fn update_entry(
    app: AppHandle,
    entry_id: String,
    input: InventoryEntryInput,
    edit_context: Option<InventoryEntryEditContext>,
    coordinator: State<'_, SharedSyncCoordinator>,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryEntryMutationResult> {
    let coordinator = coordinator.inner().clone();
    let result = coordinator.run_exclusive("inventory update", || {
        update_entry_in_store(&entry_id, input, edit_context, &db)
    })?;
    schedule_shared_publish(app, db.inner().clone(), coordinator);
    Ok(result)
}

#[tauri::command]
pub(crate) fn toggle_verified_entry(
    app: AppHandle,
    entry_id: String,
    next_verified: bool,
    coordinator: State<'_, SharedSyncCoordinator>,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryEntryMutationResult> {
    let coordinator = coordinator.inner().clone();
    let result = coordinator.run_exclusive("inventory verify", || {
        toggle_verified_entry_in_store(&entry_id, next_verified, &db)
    })?;
    schedule_shared_publish(app, db.inner().clone(), coordinator);
    Ok(result)
}

#[tauri::command]
pub(crate) fn set_archived_entry(
    app: AppHandle,
    entry_id: String,
    archived: bool,
    coordinator: State<'_, SharedSyncCoordinator>,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryEntryMutationResult> {
    let coordinator = coordinator.inner().clone();
    let result = coordinator.run_exclusive("inventory archive", || {
        set_archived_entry_in_store(&entry_id, archived, &db)
    })?;
    schedule_shared_publish(app, db.inner().clone(), coordinator);
    Ok(result)
}

#[tauri::command]
pub(crate) fn delete_entry(
    app: AppHandle,
    entry_id: String,
    coordinator: State<'_, SharedSyncCoordinator>,
    db: State<'_, InventoryDb>,
) -> CommandResult<InventoryDeleteMutationResult> {
    let coordinator = coordinator.inner().clone();
    let result =
        coordinator.run_exclusive("inventory delete", || delete_entry_in_store(&entry_id, &db))?;
    schedule_shared_publish(app, db.inner().clone(), coordinator);
    Ok(result)
}

#[cfg(test)]
fn load_inventory_from_store(db: &InventoryDb) -> CommandResult<InventorySyncResult> {
    load_inventory_from_store_with_status(db, None)
}

fn load_inventory_from_store_with_status(
    db: &InventoryDb,
    latest_background_status: Option<InventorySharedStatus>,
) -> CommandResult<InventorySyncResult> {
    let shared = latest_background_status.unwrap_or_else(|| {
        let message = sync::last_local_recovery_message(db)
            .unwrap_or_else(|| "FeOxDB local store ready. Shared sync starting.".to_string());
        sync::startup_inventory_status(message)
    });
    let entries = db.load_entries()?;

    Ok(InventorySyncResult {
        db_path: db.db_path_string(),
        entries,
        entries_changed: Some(true),
        shared,
    })
}

fn schedule_shared_publish(app: AppHandle, db: InventoryDb, coordinator: SharedSyncCoordinator) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        let status = match coordinator.run_exclusive("shared publish", || {
            sync::publish_pending_local_changes(&db)
        }) {
            Ok(result) => result.shared,
            Err(error) => sync::shared_inventory_status(
                &db,
                &format!("Background shared publish failed: {error}"),
            ),
        };
        let _ = coordinator.set_background_status(status);
        db.flush();
        let _ = app.emit(shared_watcher::SHARED_INVENTORY_CHANGED_EVENT, ());
    });
}

fn query_inventory_from_store_with_status(
    input: InventoryQueryInput,
    db: &InventoryDb,
    latest_background_status: Option<InventorySharedStatus>,
) -> CommandResult<InventoryQueryResult> {
    let all_entries = db.load_entries()?;
    let counts = get_inventory_counts(&all_entries);
    let (entries, total_filtered) = query_entries(&all_entries, input);
    let shared = latest_background_status
        .unwrap_or_else(|| sync::shared_inventory_status(db, "FeOxDB local store ready."));

    Ok(InventoryQueryResult {
        counts,
        db_path: db.db_path_string(),
        entries,
        shared,
        total_filtered,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{create_entry_from_input, InventoryEntryInput};
    use std::{env, fs, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn load_inventory_returns_local_entries_without_shared_sync_bootstrap() {
        let db = test_db();
        let entry = create_entry_from_input(1, test_input("Startup local"));
        db.put_entry(&entry).unwrap();
        db.set_next_entry_id(2).unwrap();
        db.flush();

        let loaded = load_inventory_from_store(&db).unwrap();

        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].description, "Startup local");
        assert_eq!(outbox_count(&db), 0);
        assert!(loaded.shared.message.contains("Shared sync starting"));
    }

    #[test]
    fn load_and_query_surface_latest_background_publish_status() {
        let db = test_db();
        let status = InventorySharedStatus {
            available: false,
            can_modify: true,
            enabled: true,
            has_local_only_changes: Some(true),
            last_snapshot_id: None,
            message: "Background shared publish failed: permission denied".to_string(),
            mutation_mode: "local".to_string(),
            revision: Some("7".to_string()),
            shared_root_path: Some("S:\\ME".to_string()),
            sync_interval_ms: Some(500),
        };

        let loaded = load_inventory_from_store_with_status(&db, Some(status.clone())).unwrap();
        let queried = query_inventory_from_store_with_status(
            InventoryQueryInput::default(),
            &db,
            Some(status),
        )
        .unwrap();

        assert_eq!(
            loaded.shared.message,
            "Background shared publish failed: permission denied"
        );
        assert_eq!(
            queried.shared.message,
            "Background shared publish failed: permission denied"
        );
        assert_eq!(queried.shared.has_local_only_changes, Some(true));
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
        InventoryDb::open_at(root.join("inventory.feox")).unwrap()
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

    fn unique_test_dir(prefix: &str) -> PathBuf {
        env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4().simple()))
    }
}
