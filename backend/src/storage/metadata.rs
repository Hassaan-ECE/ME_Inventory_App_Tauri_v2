use super::{keys, sync_state::SyncMetadata, InventoryDb};
use crate::model::CommandResult;

// Metadata accessors intentionally cover migration/test state beyond the current runtime path.
#[allow(dead_code)]
impl InventoryDb {
    pub(crate) fn next_entry_id(&self) -> CommandResult<i64> {
        if self.store.contains_key(keys::META_NEXT_ID) {
            let bytes = self
                .store
                .get(keys::META_NEXT_ID)
                .map_err(crate::model::db_error)?;
            let text = String::from_utf8(bytes).map_err(crate::model::db_error)?;
            if let Ok(next_id) = text.parse::<i64>() {
                return Ok(next_id.max(1));
            }
        }

        let mut max_id = 0;
        self.scan_entries(|entry| {
            if let Ok(id) = entry.id.parse::<i64>() {
                max_id = max_id.max(id);
            }
            Ok(true)
        })?;

        Ok(max_id + 1)
    }

    pub(crate) fn set_next_entry_id(&self, next_id: i64) -> CommandResult<()> {
        self.put_bytes(keys::META_NEXT_ID, next_id.to_string().as_bytes())
    }

    pub(crate) fn sync_metadata(&self) -> CommandResult<SyncMetadata> {
        Ok(SyncMetadata {
            schema_version: self.schema_version()?,
            sync_schema_version: self.sync_schema_version()?,
            client_id: self.client_id()?,
            device_id: self.device_id()?,
            next_local_seq: self.next_local_seq()?,
            last_snapshot_id: self.last_snapshot_id()?,
        })
    }

    pub(crate) fn schema_version(&self) -> CommandResult<Option<u32>> {
        self.get_u32(keys::META_SCHEMA_VERSION, "schema_version")
    }

    pub(crate) fn set_schema_version(&self, version: u32) -> CommandResult<()> {
        self.put_u32(keys::META_SCHEMA_VERSION, "schema_version", version)
    }

    pub(crate) fn sync_schema_version(&self) -> CommandResult<Option<u32>> {
        self.get_u32(keys::META_SYNC_SCHEMA_VERSION, "sync_schema_version")
    }

    pub(crate) fn set_sync_schema_version(&self, version: u32) -> CommandResult<()> {
        self.put_u32(
            keys::META_SYNC_SCHEMA_VERSION,
            "sync_schema_version",
            version,
        )
    }

    pub(crate) fn client_id(&self) -> CommandResult<Option<String>> {
        self.get_string(keys::META_CLIENT_ID)
    }

    pub(crate) fn set_client_id(&self, client_id: &str) -> CommandResult<()> {
        let client_id = keys::normalized_sync_key_segment("client_id", client_id)?;
        self.put_bytes(keys::META_CLIENT_ID, client_id.as_bytes())
    }

    pub(crate) fn get_or_create_client_id(&self) -> CommandResult<String> {
        if let Some(client_id) = self.client_id()? {
            return Ok(client_id);
        }

        let client_id = uuid::Uuid::new_v4().simple().to_string();
        self.set_client_id(&client_id)?;
        Ok(client_id)
    }

    pub(crate) fn device_id(&self) -> CommandResult<Option<String>> {
        self.get_string(keys::META_DEVICE_ID)
    }

    pub(crate) fn set_device_id(&self, device_id: &str) -> CommandResult<()> {
        let device_id = keys::normalized_sync_key_segment("device_id", device_id)?;
        self.put_bytes(keys::META_DEVICE_ID, device_id.as_bytes())
    }

    pub(crate) fn get_or_create_device_id(&self) -> CommandResult<String> {
        if let Some(device_id) = self.device_id()? {
            return Ok(device_id);
        }

        let device_id = uuid::Uuid::new_v4().simple().to_string();
        self.set_device_id(&device_id)?;
        Ok(device_id)
    }

    pub(crate) fn next_local_seq(&self) -> CommandResult<u64> {
        match self.get_u64(keys::META_NEXT_LOCAL_SEQ, "next_local_seq")? {
            Some(next_local_seq) => {
                keys::validate_local_seq(next_local_seq)?;
                Ok(next_local_seq)
            }
            None => Ok(1),
        }
    }

    pub(crate) fn set_next_local_seq(&self, next_local_seq: u64) -> CommandResult<()> {
        keys::validate_local_seq(next_local_seq)?;
        self.put_bytes(
            keys::META_NEXT_LOCAL_SEQ,
            next_local_seq.to_string().as_bytes(),
        )
    }

    pub(crate) fn reserve_next_local_seq(&self) -> CommandResult<u64> {
        let local_seq = self.next_local_seq()?;
        let next_local_seq = local_seq
            .checked_add(1)
            .ok_or_else(|| "next_local_seq overflowed".to_string())?;
        self.set_next_local_seq(next_local_seq)?;
        Ok(local_seq)
    }

    pub(crate) fn sync_revision(&self) -> CommandResult<u64> {
        Ok(self
            .get_u64(keys::META_SYNC_REVISION, "sync_revision")?
            .unwrap_or(0))
    }

    pub(crate) fn increment_sync_revision(&self) -> CommandResult<u64> {
        let next_revision = self
            .sync_revision()?
            .checked_add(1)
            .ok_or_else(|| "sync_revision overflowed".to_string())?;
        self.put_bytes(
            keys::META_SYNC_REVISION,
            next_revision.to_string().as_bytes(),
        )?;
        Ok(next_revision)
    }

    pub(crate) fn last_snapshot_id(&self) -> CommandResult<Option<String>> {
        self.get_string(keys::META_LAST_SNAPSHOT_ID)
    }

    pub(crate) fn set_last_snapshot_id(&self, snapshot_id: &str) -> CommandResult<()> {
        let snapshot_id = keys::normalized_sync_key_segment("last_snapshot_id", snapshot_id)?;
        self.put_bytes(keys::META_LAST_SNAPSHOT_ID, snapshot_id.as_bytes())
    }

    pub(crate) fn clear_last_snapshot_id(&self) -> CommandResult<()> {
        self.delete_key(keys::META_LAST_SNAPSHOT_ID)
    }
}
