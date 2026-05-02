use super::super::{
    auth, timestamps::validate_operation_timestamps, SyncCoreError, SyncCoreErrorKind,
    SyncCoreResult, SyncOperationEnvelope, SyncOperationType, SYNC_SCHEMA_VERSION,
};
use super::{
    canonical::canonical_json_bytes_without_checksum_or_auth,
    paths::{validate_local_seq, validate_path_segment},
};

pub(super) fn validate_operation_for_write(
    operation: &SyncOperationEnvelope,
) -> SyncCoreResult<()> {
    validate_path_segment(&operation.client_id)?;
    validate_local_seq(operation.local_seq)?;
    if operation.schema_version != SYNC_SCHEMA_VERSION {
        return Err(SyncCoreError::new(
            SyncCoreErrorKind::Json,
            format!(
                "Unsupported sync schema version {}.",
                operation.schema_version
            ),
        ));
    }
    if operation.entity_type != "inventory_entry" || operation.entity_id.trim().is_empty() {
        return Err(SyncCoreError::new(
            SyncCoreErrorKind::Json,
            "Operation envelope has an invalid entity reference.",
        ));
    }
    validate_operation_payload_identity(operation).map_err(|detail| {
        SyncCoreError::new(
            SyncCoreErrorKind::InvalidEnvelope,
            format!("Operation envelope payload does not match entity reference: {detail}"),
        )
    })?;
    validate_operation_timestamps(operation).map_err(|detail| {
        SyncCoreError::new(
            SyncCoreErrorKind::InvalidEnvelope,
            format!("Operation envelope has invalid timestamps: {detail}"),
        )
    })?;
    verify_operation_auth(operation).map_err(|detail| {
        SyncCoreError::new(
            SyncCoreErrorKind::InvalidEnvelope,
            format!("Operation envelope has invalid authentication: {detail}"),
        )
    })?;
    Ok(())
}
pub(super) fn verify_operation_auth(operation: &SyncOperationEnvelope) -> Result<(), String> {
    let bytes = canonical_json_bytes_without_checksum_or_auth(operation)
        .map_err(|error| error.to_string())?;
    auth::verify_canonical_bytes("sync.operation.v1", &bytes, operation.auth.as_deref())
}
pub(super) fn validate_operation_payload_identity(
    operation: &SyncOperationEnvelope,
) -> Result<(), String> {
    match operation.operation_type {
        SyncOperationType::InventoryEntryDelete => {
            if operation.payload.entry.is_some() {
                return Err("delete operation must not contain an entry payload".to_string());
            }
            if operation.payload.entry_uuid.as_deref() != Some(operation.entity_id.as_str()) {
                return Err("delete payload entry_uuid must match envelope entity_id".to_string());
            }
            if operation
                .payload
                .deleted_at_utc
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                return Err("delete payload deleted_at_utc is required".to_string());
            }
            if !operation.payload.changed_fields.is_empty() {
                return Err("delete operation must not contain changed_fields".to_string());
            }
        }
        SyncOperationType::InventoryEntryCreate
        | SyncOperationType::InventoryEntryUpdate
        | SyncOperationType::InventoryEntryVerify
        | SyncOperationType::InventoryEntryArchive => {
            let Some(entry) = operation.payload.entry.as_ref() else {
                return Err("upsert operation must contain an entry payload".to_string());
            };
            if entry.entry_uuid != operation.entity_id {
                return Err("entry payload entry_uuid must match envelope entity_id".to_string());
            }
            if operation.payload.entry_uuid.is_some() || operation.payload.deleted_at_utc.is_some()
            {
                return Err("upsert operation must not contain delete payload fields".to_string());
            }
        }
    }

    Ok(())
}
