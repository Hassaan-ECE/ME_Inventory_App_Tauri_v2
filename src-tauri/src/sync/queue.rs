use uuid::Uuid;

use crate::{
    model::{now_timestamp, CommandResult, InventoryEntry},
    store::InventoryDb,
};

use super::{
    apply::mark_operation_applied,
    conflicts::{
        record_entry_state_for_operation, record_operation_file_conflict, sync_core_error,
    },
    identity::local_identity_without_flush,
    operation_file::{
        canonical_operation_checksum, operation_file_path, read_operation_file_for_identity,
        write_operation_file,
    },
    SharedSyncPaths, SyncClientIdentity, SyncCoreErrorKind, SyncCoreResult, SyncOperationEnvelope,
    SyncOperationPayload, SyncOperationType, SyncTombstoneRecord, BOOTSTRAP_COMPLETE_KEY,
    SYNC_SCHEMA_VERSION,
};

pub(crate) fn build_entry_operation(
    identity: &SyncClientIdentity,
    local_seq: u64,
    operation_type: SyncOperationType,
    entry: InventoryEntry,
    changed_fields: Vec<String>,
    base_version: Option<String>,
) -> SyncCoreResult<SyncOperationEnvelope> {
    let mutation_ts = if entry.updated_at.trim().is_empty() {
        now_timestamp()
    } else {
        entry.updated_at.clone()
    };

    let mut operation = SyncOperationEnvelope {
        schema_version: SYNC_SCHEMA_VERSION,
        op_id: Uuid::new_v4().simple().to_string(),
        client_id: identity.client_id.clone(),
        device_id: identity.device_id.clone(),
        local_seq,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at_utc: now_timestamp(),
        operation_type,
        entity_type: "inventory_entry".to_string(),
        entity_id: entry.entry_uuid.clone(),
        base_version,
        mutation_ts_utc: mutation_ts,
        payload: SyncOperationPayload::entry(entry, changed_fields),
        checksum: String::new(),
    };

    operation.checksum = canonical_operation_checksum(&operation)?;
    Ok(operation)
}

pub(crate) fn build_delete_operation(
    identity: &SyncClientIdentity,
    local_seq: u64,
    entry_uuid: impl Into<String>,
    deleted_at_utc: impl Into<String>,
    base_version: Option<String>,
) -> SyncCoreResult<SyncOperationEnvelope> {
    let entry_uuid = entry_uuid.into();
    let deleted_at_utc = deleted_at_utc.into();
    let mut operation = SyncOperationEnvelope {
        schema_version: SYNC_SCHEMA_VERSION,
        op_id: Uuid::new_v4().simple().to_string(),
        client_id: identity.client_id.clone(),
        device_id: identity.device_id.clone(),
        local_seq,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at_utc: now_timestamp(),
        operation_type: SyncOperationType::InventoryEntryDelete,
        entity_type: "inventory_entry".to_string(),
        entity_id: entry_uuid.clone(),
        base_version,
        mutation_ts_utc: deleted_at_utc.clone(),
        payload: SyncOperationPayload::delete(entry_uuid, deleted_at_utc),
        checksum: String::new(),
    };

    operation.checksum = canonical_operation_checksum(&operation)?;
    Ok(operation)
}

pub(crate) fn queue_entry_operation(
    db: &InventoryDb,
    operation_type: SyncOperationType,
    entry: InventoryEntry,
    changed_fields: Vec<String>,
    base_version: Option<String>,
) -> CommandResult<SyncOperationEnvelope> {
    queue_entry_operation_with_revision(
        db,
        operation_type,
        entry,
        changed_fields,
        base_version,
        true,
    )
}

fn queue_entry_operation_with_revision(
    db: &InventoryDb,
    operation_type: SyncOperationType,
    entry: InventoryEntry,
    changed_fields: Vec<String>,
    base_version: Option<String>,
    bump_revision: bool,
) -> CommandResult<SyncOperationEnvelope> {
    db.set_sync_schema_version(SYNC_SCHEMA_VERSION.into())?;
    let identity = local_identity_without_flush(db)?;
    let local_seq = db.reserve_next_local_seq()?;
    let operation = build_entry_operation(
        &identity,
        local_seq,
        operation_type,
        entry,
        changed_fields,
        base_version,
    )
    .map_err(sync_core_error)?;
    persist_local_operation(db, &operation, bump_revision)?;
    Ok(operation)
}

pub(crate) fn queue_delete_operation(
    db: &InventoryDb,
    entry_uuid: impl Into<String>,
    deleted_at_utc: impl Into<String>,
    base_version: Option<String>,
) -> CommandResult<SyncOperationEnvelope> {
    db.set_sync_schema_version(SYNC_SCHEMA_VERSION.into())?;
    let identity = local_identity_without_flush(db)?;
    let local_seq = db.reserve_next_local_seq()?;
    let operation = build_delete_operation(
        &identity,
        local_seq,
        entry_uuid,
        deleted_at_utc,
        base_version,
    )
    .map_err(sync_core_error)?;
    persist_local_operation(db, &operation, true)?;
    Ok(operation)
}

pub(super) fn bootstrap_existing_entries_once(db: &InventoryDb) -> CommandResult<usize> {
    if db.get_sync_value(BOOTSTRAP_COMPLETE_KEY)?.is_some() {
        return Ok(0);
    }

    let entries = db.load_entries()?;
    let mut queued = 0usize;
    for entry in entries {
        queue_entry_operation_with_revision(
            db,
            SyncOperationType::InventoryEntryCreate,
            entry,
            Vec::new(),
            None,
            false,
        )?;
        queued += 1;
    }

    db.put_sync_value(BOOTSTRAP_COMPLETE_KEY, now_timestamp().as_bytes())?;
    db.flush();
    Ok(queued)
}

pub(super) fn push_pending_local_operations(
    db: &InventoryDb,
    paths: &SharedSyncPaths,
) -> CommandResult<usize> {
    let mut pushed_count = 0usize;
    db.scan_sync_outbox_records::<SyncOperationEnvelope, _>(None, usize::MAX, |_, operation| {
        match write_operation_file(paths, &operation) {
            Ok(_) => pushed_count += 1,
            Err(error) if error.kind == SyncCoreErrorKind::ExistingOperationConflict => {
                record_operation_file_conflict(db, paths, &operation, error.message)?;
            }
            Err(error) => return Err(sync_core_error(error)),
        }
        Ok(true)
    })?;
    Ok(pushed_count)
}

fn persist_local_operation(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
    bump_revision: bool,
) -> CommandResult<()> {
    db.put_sync_outbox_record(operation.local_seq, operation)?;
    mark_operation_applied(db, operation)?;

    if operation.operation_type == SyncOperationType::InventoryEntryDelete {
        let tombstone = SyncTombstoneRecord {
            entry_uuid: operation.entity_id.clone(),
            deleted_at_utc: operation
                .payload
                .deleted_at_utc
                .clone()
                .unwrap_or_else(|| operation.mutation_ts_utc.clone()),
            op_id: operation.op_id.clone(),
            client_id: operation.client_id.clone(),
            local_seq: operation.local_seq,
            base_version: operation.base_version.clone(),
        };
        db.put_sync_tombstone(&operation.entity_id, &tombstone)?;
    } else if db.has_sync_tombstone(&operation.entity_id)? {
        db.delete_sync_tombstone(&operation.entity_id)?;
    }

    record_entry_state_for_operation(db, operation)?;
    if bump_revision {
        db.increment_sync_revision()?;
    }

    Ok(())
}

pub(super) fn count_pending_local_operations(
    db: &InventoryDb,
    paths: Option<&SharedSyncPaths>,
) -> CommandResult<usize> {
    let mut count = 0usize;
    db.scan_sync_outbox_records::<SyncOperationEnvelope, _>(None, usize::MAX, |_, operation| {
        let written = paths
            .map(|paths| operation_file_matches(paths, &operation))
            .unwrap_or(false);
        if !written {
            count += 1;
        }
        Ok(true)
    })?;
    Ok(count)
}

fn operation_file_matches(paths: &SharedSyncPaths, operation: &SyncOperationEnvelope) -> bool {
    let Ok(path) = operation_file_path(paths, &operation.client_id, operation.local_seq) else {
        return false;
    };
    if !path.exists() {
        return false;
    }

    read_operation_file_for_identity(&path, &operation.client_id, operation.local_seq)
        .map(|existing| {
            existing.checksum == operation.checksum && existing.op_id == operation.op_id
        })
        .unwrap_or(false)
}
