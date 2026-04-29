use std::path::PathBuf;

use crate::{
    model::{now_timestamp, numeric_id, CommandResult, InventoryEntry},
    store::InventoryDb,
};

use super::{
    conflicts::{
        current_entry_state, operation_wins_state, record_corrupt_remote_file,
        record_corrupt_remote_files, record_entry_state_for_operation,
        record_stale_operation_conflict, sync_core_error,
    },
    operation_file::operation_file_path,
    queue::{
        bootstrap_existing_entries_once, count_pending_local_operations,
        push_pending_local_operations,
    },
    scanning::scan_operation_files,
    shared_paths::{build_shared_status, ensure_operation_log_layout, resolve_shared_root},
    CorruptRemoteFile, CorruptRemoteReason, SharedSyncPaths, SharedSyncRunResult,
    SyncAppliedMarker, SyncOperationEnvelope, SyncOperationType, SyncTombstoneRecord,
};

pub(crate) fn run_shared_sync(db: &InventoryDb) -> CommandResult<SharedSyncRunResult> {
    let root = resolve_shared_root();
    run_shared_sync_with_root(db, root)
}

pub(crate) fn run_shared_sync_with_root(
    db: &InventoryDb,
    shared_root: impl Into<PathBuf>,
) -> CommandResult<SharedSyncRunResult> {
    let paths = SharedSyncPaths::from_shared_root(shared_root);
    bootstrap_existing_entries_once(db)?;

    if !paths.shared_root.exists() {
        let pending_count = count_pending_local_operations(db, None)?;
        return Ok(SharedSyncRunResult {
            entries_changed: false,
            shared: build_shared_status(
                db,
                &paths,
                false,
                pending_count,
                0,
                "Shared workspace unavailable. Saving changes locally.".to_string(),
            ),
        });
    }

    if let Err(error) = ensure_operation_log_layout(&paths) {
        let pending_count = count_pending_local_operations(db, None)?;
        return Ok(SharedSyncRunResult {
            entries_changed: false,
            shared: build_shared_status(
                db,
                &paths,
                false,
                pending_count,
                0,
                format!("Shared workspace unavailable. {error}"),
            ),
        });
    }

    let _pushed_count = push_pending_local_operations(db, &paths)?;
    let pull_report = pull_remote_operations(db, &paths)?;
    let pending_count = count_pending_local_operations(db, Some(&paths))?;
    let message = if pull_report.corrupt_count > 0 {
        format!(
            "Shared operation sync ready. Ignored {} corrupt remote file(s).",
            pull_report.corrupt_count
        )
    } else {
        "Shared operation sync ready.".to_string()
    };

    Ok(SharedSyncRunResult {
        entries_changed: pull_report.entries_changed,
        shared: build_shared_status(
            db,
            &paths,
            true,
            pending_count,
            pull_report.corrupt_count,
            message,
        ),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct PullReport {
    entries_changed: bool,
    corrupt_count: usize,
}

fn pull_remote_operations(db: &InventoryDb, paths: &SharedSyncPaths) -> CommandResult<PullReport> {
    let scan_report = scan_operation_files(paths).map_err(sync_core_error)?;
    let mut corrupt_count = record_corrupt_remote_files(db, &scan_report.corrupt)?;
    let mut entries_changed = false;
    let mut applied_count = 0usize;

    for operation in scan_report.operations {
        if db.has_sync_applied_marker(&operation.op_id)? {
            continue;
        }

        if let Some(existing_op_id) =
            db.sync_client_seq_marker::<String>(&operation.client_id, operation.local_seq)?
        {
            if existing_op_id != operation.op_id {
                let corrupt = CorruptRemoteFile {
                    path: operation_file_path(paths, &operation.client_id, operation.local_seq)
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_else(|_| {
                            format!("{}:{}", operation.client_id, operation.local_seq)
                        }),
                    reason: CorruptRemoteReason::DuplicateSequenceDifferentChecksum,
                    detail: "Remote operation uses an already-applied client_id/local_seq with a different op_id."
                        .to_string(),
                    detected_at_utc: now_timestamp(),
                    content_sha256: Some(operation.checksum.clone()),
                };
                record_corrupt_remote_file(db, &corrupt)?;
                corrupt_count += 1;
                continue;
            }
        }

        if apply_remote_operation(db, &operation)? {
            entries_changed = true;
        }
        mark_operation_applied(db, &operation)?;
        db.set_sync_watermark(&operation.client_id, operation.local_seq)?;
        applied_count += 1;
    }

    if entries_changed || applied_count > 0 {
        db.flush();
    }

    Ok(PullReport {
        entries_changed,
        corrupt_count,
    })
}

fn apply_remote_operation(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
) -> CommandResult<bool> {
    if let Some(current_state) = current_entry_state(db, &operation.entity_id)? {
        if current_state.last_op_id == operation.op_id {
            return Ok(false);
        }
        if !operation_wins_state(operation, &current_state) {
            record_stale_operation_conflict(db, operation, &current_state)?;
            return Ok(false);
        }
    }

    let entries_changed = match operation.operation_type {
        SyncOperationType::InventoryEntryDelete => apply_remote_delete(db, operation),
        SyncOperationType::InventoryEntryCreate
        | SyncOperationType::InventoryEntryUpdate
        | SyncOperationType::InventoryEntryVerify
        | SyncOperationType::InventoryEntryArchive => apply_remote_upsert(db, operation),
    }?;

    record_entry_state_for_operation(db, operation)?;
    if entries_changed {
        db.increment_sync_revision()?;
    }

    Ok(entries_changed)
}

fn apply_remote_delete(db: &InventoryDb, operation: &SyncOperationEnvelope) -> CommandResult<bool> {
    let entry_uuid = operation
        .payload
        .entry_uuid
        .as_deref()
        .unwrap_or(&operation.entity_id);
    let deleted_at_utc = operation
        .payload
        .deleted_at_utc
        .as_deref()
        .unwrap_or(&operation.mutation_ts_utc);

    let tombstone = SyncTombstoneRecord {
        entry_uuid: entry_uuid.to_string(),
        deleted_at_utc: deleted_at_utc.to_string(),
        op_id: operation.op_id.clone(),
        client_id: operation.client_id.clone(),
        local_seq: operation.local_seq,
        base_version: operation.base_version.clone(),
    };
    db.put_sync_tombstone(entry_uuid, &tombstone)?;

    if let Some(entry) = db.find_entry(entry_uuid)? {
        db.delete_entry(&entry)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn apply_remote_upsert(db: &InventoryDb, operation: &SyncOperationEnvelope) -> CommandResult<bool> {
    let Some(entry) = operation.payload.entry.clone() else {
        return Ok(false);
    };

    if db.has_sync_tombstone(&operation.entity_id)? {
        db.delete_sync_tombstone(&operation.entity_id)?;
    }

    let entry = prepare_incoming_entry(db, entry)?;
    let changed = db
        .find_entry(&entry.entry_uuid)?
        .map(|existing| existing.updated_at != entry.updated_at || existing != entry)
        .unwrap_or(true);
    db.put_entry(&entry)?;
    bump_next_entry_id_after_remote_entry(db, &entry)?;
    Ok(changed)
}

fn prepare_incoming_entry(
    db: &InventoryDb,
    mut entry: InventoryEntry,
) -> CommandResult<InventoryEntry> {
    if let Some(existing) = db.find_entry(&entry.entry_uuid)? {
        entry.id = existing.id;
        entry.database_id = existing.database_id;
        return Ok(entry);
    }

    if entry.id.trim().is_empty() || local_id_belongs_to_different_entry(db, &entry)? {
        let local_id = reserve_unused_entry_id(db)?;
        entry.id = local_id.to_string();
        entry.database_id = Some(local_id);
    }

    Ok(entry)
}

fn local_id_belongs_to_different_entry(
    db: &InventoryDb,
    entry: &InventoryEntry,
) -> CommandResult<bool> {
    if entry.id.trim().is_empty() {
        return Ok(false);
    }

    Ok(db
        .find_entry(&entry.id)?
        .map(|existing| existing.entry_uuid != entry.entry_uuid)
        .unwrap_or(false))
}

fn reserve_unused_entry_id(db: &InventoryDb) -> CommandResult<i64> {
    loop {
        let candidate = db.next_entry_id()?;
        let candidate_text = candidate.to_string();
        if db.find_entry(&candidate_text)?.is_none() {
            db.set_next_entry_id(candidate + 1)?;
            return Ok(candidate);
        }
        db.set_next_entry_id(candidate + 1)?;
    }
}

fn bump_next_entry_id_after_remote_entry(
    db: &InventoryDb,
    entry: &InventoryEntry,
) -> CommandResult<()> {
    let entry_id = numeric_id(&entry.id);
    if entry_id > 0 && entry_id >= db.next_entry_id()? {
        db.set_next_entry_id(entry_id + 1)?;
    }
    Ok(())
}

pub(super) fn mark_operation_applied(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
) -> CommandResult<()> {
    let marker = SyncAppliedMarker {
        op_id: operation.op_id.clone(),
        client_id: operation.client_id.clone(),
        local_seq: operation.local_seq,
        checksum: operation.checksum.clone(),
        applied_at_utc: now_timestamp(),
    };
    db.put_sync_applied_marker(&operation.op_id, &marker)?;
    db.put_sync_client_seq_marker(&operation.client_id, operation.local_seq, &operation.op_id)?;
    Ok(())
}
