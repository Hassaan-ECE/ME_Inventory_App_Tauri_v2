use std::{env, ffi::OsString, fs, path::PathBuf};

use crate::{model::InventorySharedStatus, store::InventoryDb};

use super::{
    queue::count_pending_local_operations, SharedSyncPaths, SyncCoreResult, DEFAULT_SHARED_ROOT,
    SHARED_ROOT_ENV, SHARED_SYNC_INTERVAL_MS,
};

pub(crate) fn resolve_shared_root() -> PathBuf {
    resolve_shared_root_from_env_value(env::var_os(SHARED_ROOT_ENV))
}

pub(crate) fn resolved_shared_sync_paths() -> SharedSyncPaths {
    SharedSyncPaths::from_shared_root(resolve_shared_root())
}

pub(crate) fn resolve_shared_root_from_env_value(value: Option<OsString>) -> PathBuf {
    value
        .and_then(|value| {
            let path = value.to_string_lossy().trim().to_string();
            (!path.is_empty()).then_some(PathBuf::from(path))
        })
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SHARED_ROOT))
}

pub(crate) fn ensure_operation_log_layout(paths: &SharedSyncPaths) -> SyncCoreResult<()> {
    fs::create_dir_all(&paths.ops_dir)?;
    fs::create_dir_all(&paths.snapshots_dir)?;
    fs::create_dir_all(&paths.locks_dir)?;
    fs::create_dir_all(&paths.backups_dir)?;
    Ok(())
}

pub(crate) fn shared_inventory_status(
    db: &InventoryDb,
    message: impl Into<String>,
) -> InventorySharedStatus {
    let paths = resolved_shared_sync_paths();
    let available = paths.shared_root.exists();
    let pending_count =
        count_pending_local_operations(db, available.then_some(&paths)).unwrap_or(0);
    build_shared_status(db, &paths, available, pending_count, 0, message.into())
}

pub(crate) fn startup_inventory_status(message: impl Into<String>) -> InventorySharedStatus {
    let paths = resolved_shared_sync_paths();
    let available = paths.shared_root.exists();
    InventorySharedStatus {
        available,
        can_modify: true,
        enabled: true,
        has_local_only_changes: None,
        message: message.into(),
        mutation_mode: if available { "shared" } else { "local" }.to_string(),
        revision: None,
        last_snapshot_id: None,
        shared_root_path: Some(paths.shared_root.to_string_lossy().into_owned()),
        sync_interval_ms: Some(SHARED_SYNC_INTERVAL_MS),
    }
}

pub(crate) fn queued_local_status(db: &InventoryDb) -> InventorySharedStatus {
    let paths = resolved_shared_sync_paths();
    let available = paths.shared_root.exists();
    let pending_count =
        count_pending_local_operations(db, available.then_some(&paths)).unwrap_or(1);
    build_shared_status(
        db,
        &paths,
        available,
        pending_count.max(1),
        0,
        "Local change saved and queued for shared sync.".to_string(),
    )
}

pub(super) fn build_shared_status(
    db: &InventoryDb,
    paths: &SharedSyncPaths,
    available: bool,
    pending_count: usize,
    corrupt_count: usize,
    message: String,
) -> InventorySharedStatus {
    let has_pending = pending_count > 0;
    let mut message = message;
    if has_pending && !message.contains("Pending local") {
        message = format!("{message} Pending local changes: {pending_count}.");
    }
    if corrupt_count > 0 && !message.contains("corrupt") {
        message = format!("{message} Corrupt remote files ignored: {corrupt_count}.");
    }

    InventorySharedStatus {
        available,
        can_modify: true,
        enabled: true,
        has_local_only_changes: Some(has_pending),
        message,
        mutation_mode: if available && !has_pending {
            "shared".to_string()
        } else {
            "local".to_string()
        },
        revision: db.sync_revision().ok().map(|revision| revision.to_string()),
        last_snapshot_id: db.last_snapshot_id().ok().flatten(),
        shared_root_path: Some(paths.shared_root.to_string_lossy().into_owned()),
        sync_interval_ms: Some(SHARED_SYNC_INTERVAL_MS),
    }
}
