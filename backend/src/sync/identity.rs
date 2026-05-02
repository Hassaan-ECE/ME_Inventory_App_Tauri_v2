use crate::{model::CommandResult, store::InventoryDb};

use super::{
    conflicts::sync_core_error, operation_file::validate_path_segment, SyncClientIdentity,
};

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn get_or_create_client_identity(db: &InventoryDb) -> CommandResult<SyncClientIdentity> {
    db.set_sync_schema_version(super::SYNC_SCHEMA_VERSION.into())?;
    let client_id = db.get_or_create_client_id()?;
    validate_path_segment(&client_id).map_err(|error| error.message)?;
    let device_id = db.get_or_create_device_id()?;

    db.flush();
    Ok(SyncClientIdentity {
        client_id,
        device_id,
    })
}

#[allow(dead_code)]
pub(crate) fn peek_next_local_sequence(db: &InventoryDb) -> CommandResult<u64> {
    db.next_local_seq()
}

#[allow(dead_code)]
pub(crate) fn allocate_local_sequence(db: &InventoryDb) -> CommandResult<u64> {
    let next_seq = db.reserve_next_local_seq()?;
    db.flush();
    Ok(next_seq)
}

pub(super) fn local_identity_without_flush(db: &InventoryDb) -> CommandResult<SyncClientIdentity> {
    let client_id = db.get_or_create_client_id()?;
    validate_path_segment(&client_id).map_err(sync_core_error)?;
    let device_id = db.get_or_create_device_id()?;
    Ok(SyncClientIdentity {
        client_id,
        device_id,
    })
}
