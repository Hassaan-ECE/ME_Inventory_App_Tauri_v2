use crate::{
    model::{now_timestamp, CommandResult},
    store::InventoryDb,
};

use super::{
    operation_file::{operation_file_path, sha256_hex},
    CorruptRemoteFile, CorruptRemoteReason, SharedSyncPaths, SyncConflictReason,
    SyncConflictRecord, SyncEntryState, SyncOperationEnvelope, SyncOperationType,
    SyncTombstoneRecord,
};

pub(crate) fn record_corrupt_remote_file(
    db: &InventoryDb,
    corrupt: &CorruptRemoteFile,
) -> CommandResult<()> {
    put_corrupt_remote_file(db, corrupt)?;
    db.flush();
    Ok(())
}

pub(crate) fn record_corrupt_remote_files(
    db: &InventoryDb,
    corrupt_files: &[CorruptRemoteFile],
) -> CommandResult<usize> {
    for corrupt in corrupt_files {
        put_corrupt_remote_file(db, corrupt)?;
    }

    if !corrupt_files.is_empty() {
        db.flush();
    }

    Ok(corrupt_files.len())
}

pub(super) fn current_entry_state(
    db: &InventoryDb,
    entry_uuid: &str,
) -> CommandResult<Option<SyncEntryState>> {
    if let Some(state) = db.sync_entry_state::<SyncEntryState>(entry_uuid)? {
        return Ok(Some(state));
    }

    if let Some(tombstone) = db.sync_tombstone::<SyncTombstoneRecord>(entry_uuid)? {
        return Ok(Some(SyncEntryState {
            entry_uuid: tombstone.entry_uuid,
            last_op_id: tombstone.op_id,
            mutation_ts_utc: tombstone.deleted_at_utc,
            deleted: true,
            base_version: tombstone.base_version,
            changed_fields: Vec::new(),
            source_client_id: tombstone.client_id,
            source_local_seq: tombstone.local_seq,
            operation_type: SyncOperationType::InventoryEntryDelete,
            updated_at_utc: now_timestamp(),
        }));
    }

    let Some(entry) = db.find_entry(entry_uuid)? else {
        return Ok(None);
    };

    Ok(Some(SyncEntryState {
        entry_uuid: entry.entry_uuid,
        last_op_id: String::new(),
        mutation_ts_utc: entry.updated_at,
        deleted: false,
        base_version: None,
        changed_fields: Vec::new(),
        source_client_id: "local".to_string(),
        source_local_seq: 0,
        operation_type: SyncOperationType::InventoryEntryUpdate,
        updated_at_utc: now_timestamp(),
    }))
}

pub(super) fn operation_wins_state(
    operation: &SyncOperationEnvelope,
    state: &SyncEntryState,
) -> bool {
    operation.mutation_ts_utc.as_str() > state.mutation_ts_utc.as_str()
        || (operation.mutation_ts_utc == state.mutation_ts_utc
            && operation.op_id.as_str() > state.last_op_id.as_str())
}

pub(super) fn record_entry_state_for_operation(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
) -> CommandResult<()> {
    let state = SyncEntryState {
        entry_uuid: operation.entity_id.clone(),
        last_op_id: operation.op_id.clone(),
        mutation_ts_utc: operation.mutation_ts_utc.clone(),
        deleted: operation.operation_type == SyncOperationType::InventoryEntryDelete,
        base_version: operation.base_version.clone(),
        changed_fields: operation.payload.changed_fields.clone(),
        source_client_id: operation.client_id.clone(),
        source_local_seq: operation.local_seq,
        operation_type: operation.operation_type,
        updated_at_utc: now_timestamp(),
    };
    db.put_sync_entry_state(&operation.entity_id, &state)?;
    Ok(())
}

pub(super) fn record_stale_operation_conflict(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
    current_state: &SyncEntryState,
) -> CommandResult<()> {
    let conflict_id = stale_conflict_id(operation, current_state);
    let record = SyncConflictRecord {
        conflict_id: conflict_id.clone(),
        entry_uuid: operation.entity_id.clone(),
        incoming_op_id: operation.op_id.clone(),
        incoming_client_id: operation.client_id.clone(),
        incoming_local_seq: operation.local_seq,
        incoming_mutation_ts_utc: operation.mutation_ts_utc.clone(),
        current_op_id: current_state.last_op_id.clone(),
        current_client_id: current_state.source_client_id.clone(),
        current_local_seq: current_state.source_local_seq,
        current_mutation_ts_utc: current_state.mutation_ts_utc.clone(),
        reason: SyncConflictReason::StaleIncomingOperation,
        detected_at_utc: now_timestamp(),
    };
    db.put_sync_conflict_record(&conflict_id, &record)?;
    Ok(())
}

fn stale_conflict_id(operation: &SyncOperationEnvelope, current_state: &SyncEntryState) -> String {
    let source = format!(
        "stale:{}:{}:{}:{}",
        operation.entity_id,
        operation.op_id,
        current_state.last_op_id,
        current_state.mutation_ts_utc
    );
    sha256_hex(source.as_bytes())
}

pub(super) fn record_operation_file_conflict(
    db: &InventoryDb,
    paths: &SharedSyncPaths,
    operation: &SyncOperationEnvelope,
    detail: String,
) -> CommandResult<()> {
    let path = operation_file_path(paths, &operation.client_id, operation.local_seq)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| format!("{}:{}", operation.client_id, operation.local_seq));
    let corrupt = CorruptRemoteFile {
        path,
        reason: CorruptRemoteReason::DuplicateSequenceDifferentChecksum,
        detail,
        detected_at_utc: now_timestamp(),
        content_sha256: Some(operation.checksum.clone()),
    };
    record_corrupt_remote_file(db, &corrupt)
}

pub(super) fn sync_core_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn put_corrupt_remote_file(db: &InventoryDb, corrupt: &CorruptRemoteFile) -> CommandResult<()> {
    db.put_sync_corrupt_record(&corrupt_remote_record_id(corrupt), corrupt)?;
    Ok(())
}

pub(crate) fn corrupt_remote_record_id(corrupt: &CorruptRemoteFile) -> String {
    let source = corrupt
        .content_sha256
        .as_deref()
        .unwrap_or(&corrupt.path)
        .as_bytes();
    sha256_hex(source)
}
