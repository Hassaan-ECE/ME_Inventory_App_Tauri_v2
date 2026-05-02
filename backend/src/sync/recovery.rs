use serde::{Deserialize, Serialize};

use crate::{
    model::{db_error, now_timestamp, CommandResult},
    store::{InventoryDb, SyncKeyspace},
};

use super::{
    apply::mark_operation_applied,
    conflicts::{current_entry_state, operation_wins_state, record_entry_state_for_operation},
    snapshot::SNAPSHOT_APPLY_PENDING_KEY,
    timestamps::compare_timestamp_text,
    SyncAppliedMarker, SyncEntryState, SyncOperationEnvelope, SyncOperationPayload,
    SyncOperationType, SyncTombstoneRecord,
};

const LOCAL_RECOVERY_STATUS_KEY: &str = "meta:last_local_recovery_status";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalSyncRecoveryReport {
    pub recovered_at_utc: String,
    pub repaired_client_seq_markers: usize,
    pub repaired_entry_states: usize,
    pub repaired_entries: usize,
    #[serde(default)]
    pub repaired_local_sequence_markers: usize,
    pub repaired_outbox_operations: usize,
    pub repaired_tombstones: usize,
    pub snapshot_apply_pending: Option<String>,
}

impl LocalSyncRecoveryReport {
    fn has_repairs(&self) -> bool {
        self.repaired_client_seq_markers > 0
            || self.repaired_entry_states > 0
            || self.repaired_entries > 0
            || self.repaired_local_sequence_markers > 0
            || self.repaired_outbox_operations > 0
            || self.repaired_tombstones > 0
            || self.snapshot_apply_pending.is_some()
    }

    fn message(&self) -> String {
        if !self.has_repairs() {
            return "Local sync state is consistent.".to_string();
        }

        let mut parts = Vec::new();
        if self.repaired_outbox_operations > 0 {
            parts.push(format!(
                "{} outbox operation(s)",
                self.repaired_outbox_operations
            ));
        }
        if self.repaired_client_seq_markers > 0 {
            parts.push(format!(
                "{} applied sequence marker(s)",
                self.repaired_client_seq_markers
            ));
        }
        if self.repaired_local_sequence_markers > 0 {
            parts.push(format!(
                "{} local sequence marker(s)",
                self.repaired_local_sequence_markers
            ));
        }
        if self.repaired_entries > 0 {
            parts.push(format!(
                "{} inventory entry record(s)",
                self.repaired_entries
            ));
        }
        if self.repaired_tombstones > 0 {
            parts.push(format!("{} tombstone record(s)", self.repaired_tombstones));
        }
        if self.repaired_entry_states > 0 {
            parts.push(format!(
                "{} entry-state record(s)",
                self.repaired_entry_states
            ));
        }
        if self.snapshot_apply_pending.is_some() {
            parts.push("one interrupted snapshot apply".to_string());
        }

        format!("Recovered local sync state: {}.", parts.join(", "))
    }
}

pub(crate) fn recover_local_sync_state(db: &InventoryDb) -> CommandResult<LocalSyncRecoveryReport> {
    let mut report = LocalSyncRecoveryReport {
        recovered_at_utc: now_timestamp(),
        ..LocalSyncRecoveryReport::default()
    };

    repair_applied_client_sequence_markers(db, &mut report)?;
    repair_outbox_operations(db, &mut report)?;
    repair_next_local_sequence_marker(db, &mut report)?;
    repair_tombstone_invariants(db, &mut report)?;
    note_pending_snapshot_apply(db, &mut report)?;

    if report.has_repairs() {
        let bytes = serde_json::to_vec(&report).map_err(db_error)?;
        db.put_sync_value(LOCAL_RECOVERY_STATUS_KEY, &bytes)?;
        db.increment_sync_revision()?;
        db.flush();
    }

    Ok(report)
}

pub(crate) fn last_local_recovery_message(db: &InventoryDb) -> Option<String> {
    let bytes = db
        .get_sync_value(LOCAL_RECOVERY_STATUS_KEY)
        .ok()
        .flatten()?;
    let report = serde_json::from_slice::<LocalSyncRecoveryReport>(&bytes).ok()?;
    report.has_repairs().then(|| report.message())
}

fn repair_applied_client_sequence_markers(
    db: &InventoryDb,
    report: &mut LocalSyncRecoveryReport,
) -> CommandResult<()> {
    let mut markers = Vec::new();
    db.scan_sync_range(SyncKeyspace::Applied, usize::MAX, |_, value| {
        markers.push(serde_json::from_slice::<SyncAppliedMarker>(value).map_err(db_error)?);
        Ok(true)
    })?;

    for marker in markers {
        let current = db.sync_client_seq_marker::<String>(&marker.client_id, marker.local_seq)?;
        if current.as_deref() != Some(marker.op_id.as_str()) {
            db.put_sync_client_seq_marker(&marker.client_id, marker.local_seq, &marker.op_id)?;
            report.repaired_client_seq_markers += 1;
        }
    }

    Ok(())
}

fn repair_outbox_operations(
    db: &InventoryDb,
    report: &mut LocalSyncRecoveryReport,
) -> CommandResult<()> {
    let mut operations = Vec::new();
    db.scan_sync_outbox_records::<SyncOperationEnvelope, _>(None, usize::MAX, |_, operation| {
        operations.push(operation);
        Ok(true)
    })?;

    for operation in operations {
        let before = report_total_repairs(report);
        repair_outbox_operation(db, &operation, report)?;
        if report_total_repairs(report) > before {
            report.repaired_outbox_operations += 1;
        }
    }

    Ok(())
}

fn repair_outbox_operation(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
    report: &mut LocalSyncRecoveryReport,
) -> CommandResult<()> {
    let client_seq_marker =
        db.sync_client_seq_marker::<String>(&operation.client_id, operation.local_seq)?;
    if !db.has_sync_applied_marker(&operation.op_id)?
        || client_seq_marker.as_deref() != Some(operation.op_id.as_str())
    {
        mark_operation_applied(db, operation)?;
        report.repaired_client_seq_markers += 1;
    }

    if !outbox_operation_can_repair_entity(db, operation)? {
        return Ok(());
    }

    match operation.operation_type {
        SyncOperationType::InventoryEntryDelete => {
            if let Some(entry) = db.find_entry(&operation.entity_id)? {
                db.delete_entry(&entry)?;
                report.repaired_entries += 1;
            }
            let tombstone = tombstone_for_operation(operation);
            if db
                .sync_tombstone::<SyncTombstoneRecord>(&operation.entity_id)?
                .as_ref()
                .map(|current| current.op_id.as_str())
                != Some(operation.op_id.as_str())
            {
                db.put_sync_tombstone(&operation.entity_id, &tombstone)?;
                report.repaired_tombstones += 1;
            }
        }
        SyncOperationType::InventoryEntryCreate
        | SyncOperationType::InventoryEntryUpdate
        | SyncOperationType::InventoryEntryVerify
        | SyncOperationType::InventoryEntryArchive => {
            if let Some(entry) = operation.payload.entry.as_ref() {
                if db.find_entry(&operation.entity_id)?.as_ref() != Some(entry) {
                    db.put_entry(entry)?;
                    report.repaired_entries += 1;
                }
            }
            if db.has_sync_tombstone(&operation.entity_id)? {
                db.delete_sync_tombstone(&operation.entity_id)?;
                report.repaired_tombstones += 1;
            }
        }
    }

    if db
        .sync_entry_state::<SyncEntryState>(&operation.entity_id)?
        .as_ref()
        .map(|state| state.last_op_id.as_str())
        != Some(operation.op_id.as_str())
    {
        record_entry_state_for_operation(db, operation)?;
        report.repaired_entry_states += 1;
    }

    Ok(())
}

fn repair_next_local_sequence_marker(
    db: &InventoryDb,
    report: &mut LocalSyncRecoveryReport,
) -> CommandResult<()> {
    let local_client_id = db.client_id()?;
    let mut max_local_seq = 0u64;

    db.scan_sync_outbox_records::<SyncOperationEnvelope, _>(None, usize::MAX, |_, operation| {
        max_local_seq = max_local_seq.max(operation.local_seq);
        Ok(true)
    })?;

    if let Some(local_client_id) = local_client_id {
        db.scan_sync_range(SyncKeyspace::Applied, usize::MAX, |_, value| {
            let marker = serde_json::from_slice::<SyncAppliedMarker>(value).map_err(db_error)?;
            if marker.client_id == local_client_id {
                max_local_seq = max_local_seq.max(marker.local_seq);
            }
            Ok(true)
        })?;
    }

    if max_local_seq == 0 {
        return Ok(());
    }

    let next_local_seq = db.next_local_seq()?;
    if next_local_seq <= max_local_seq {
        let repaired_next_seq = max_local_seq
            .checked_add(1)
            .ok_or_else(|| "next_local_seq recovery overflowed".to_string())?;
        db.set_next_local_seq(repaired_next_seq)?;
        report.repaired_local_sequence_markers += 1;
    }

    Ok(())
}

fn outbox_operation_can_repair_entity(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
) -> CommandResult<bool> {
    let Some(current_state) = current_entry_state(db, &operation.entity_id)? else {
        return Ok(true);
    };

    Ok(current_state.last_op_id == operation.op_id
        || operation_wins_state(operation, &current_state))
}

fn repair_tombstone_invariants(
    db: &InventoryDb,
    report: &mut LocalSyncRecoveryReport,
) -> CommandResult<()> {
    let mut tombstones = Vec::new();
    db.scan_sync_tombstones::<SyncTombstoneRecord, _>(usize::MAX, |_, tombstone| {
        tombstones.push(tombstone);
        Ok(true)
    })?;

    for tombstone in tombstones {
        if let Some(state) = db.sync_entry_state::<SyncEntryState>(&tombstone.entry_uuid)? {
            if !state.deleted && state_covers_tombstone(&state, &tombstone) {
                db.delete_sync_tombstone(&tombstone.entry_uuid)?;
                report.repaired_tombstones += 1;
                continue;
            }

            if state.deleted
                && state.last_op_id != tombstone.op_id
                && state_covers_tombstone(&state, &tombstone)
            {
                db.put_sync_tombstone(&state.entry_uuid, &tombstone_for_state(&state))?;
                report.repaired_tombstones += 1;
                continue;
            }
        }

        if let Some(entry) = db.find_entry(&tombstone.entry_uuid)? {
            db.delete_entry(&entry)?;
            report.repaired_entries += 1;
        }
        if db
            .sync_entry_state::<SyncEntryState>(&tombstone.entry_uuid)?
            .is_none()
        {
            let operation = tombstone_as_delete_operation(&tombstone);
            record_entry_state_for_operation(db, &operation)?;
            report.repaired_entry_states += 1;
        }
    }

    Ok(())
}

fn state_covers_tombstone(state: &SyncEntryState, tombstone: &SyncTombstoneRecord) -> bool {
    let timestamp_order = compare_timestamp_text(&state.mutation_ts_utc, &tombstone.deleted_at_utc);
    timestamp_order.is_gt()
        || (timestamp_order.is_eq() && state.last_op_id.as_str() >= tombstone.op_id.as_str())
}

fn tombstone_for_state(state: &SyncEntryState) -> SyncTombstoneRecord {
    SyncTombstoneRecord {
        entry_uuid: state.entry_uuid.clone(),
        deleted_at_utc: state.mutation_ts_utc.clone(),
        op_id: state.last_op_id.clone(),
        client_id: state.source_client_id.clone(),
        local_seq: state.source_local_seq,
        base_version: state.base_version.clone(),
    }
}

fn note_pending_snapshot_apply(
    db: &InventoryDb,
    report: &mut LocalSyncRecoveryReport,
) -> CommandResult<()> {
    let Some(bytes) = db.get_sync_value(SNAPSHOT_APPLY_PENDING_KEY)? else {
        return Ok(());
    };
    let pending_snapshot_id = String::from_utf8(bytes).map_err(db_error)?;
    if db.last_snapshot_id()?.as_deref() == Some(pending_snapshot_id.as_str()) {
        db.delete_sync_value(SNAPSHOT_APPLY_PENDING_KEY)?;
    } else {
        report.snapshot_apply_pending = Some(pending_snapshot_id);
    }

    Ok(())
}

fn tombstone_for_operation(operation: &SyncOperationEnvelope) -> SyncTombstoneRecord {
    SyncTombstoneRecord {
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
    }
}

fn tombstone_as_delete_operation(tombstone: &SyncTombstoneRecord) -> SyncOperationEnvelope {
    SyncOperationEnvelope {
        schema_version: super::SYNC_SCHEMA_VERSION,
        op_id: tombstone.op_id.clone(),
        client_id: tombstone.client_id.clone(),
        device_id: "recovered".to_string(),
        local_seq: tombstone.local_seq,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at_utc: tombstone.deleted_at_utc.clone(),
        operation_type: SyncOperationType::InventoryEntryDelete,
        entity_type: "inventory_entry".to_string(),
        entity_id: tombstone.entry_uuid.clone(),
        base_version: tombstone.base_version.clone(),
        mutation_ts_utc: tombstone.deleted_at_utc.clone(),
        payload: SyncOperationPayload::delete(
            tombstone.entry_uuid.clone(),
            tombstone.deleted_at_utc.clone(),
        ),
        checksum: String::new(),
        auth: None,
    }
}

fn report_total_repairs(report: &LocalSyncRecoveryReport) -> usize {
    report.repaired_client_seq_markers
        + report.repaired_entry_states
        + report.repaired_entries
        + report.repaired_local_sequence_markers
        + report.repaired_tombstones
}
