use serde::{de::DeserializeOwned, Serialize};

use super::{codec::decode_u64_value, keys, InventoryDb};
use crate::model::{db_error, CommandResult};

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct SyncMetadata {
    pub schema_version: Option<u32>,
    pub sync_schema_version: Option<u32>,
    pub client_id: Option<String>,
    pub device_id: Option<String>,
    pub next_local_seq: u64,
    pub last_snapshot_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum SyncKeyspace {
    Outbox,
    Applied,
    ClientSeq,
    Watermark,
    Tombstone,
    EntryState,
    Conflict,
    CorruptRemote,
}

impl SyncKeyspace {
    fn prefix(self) -> &'static str {
        match self {
            Self::Outbox => keys::SYNC_OUTBOX_PREFIX,
            Self::Applied => keys::SYNC_APPLIED_PREFIX,
            Self::ClientSeq => keys::SYNC_CLIENT_SEQ_PREFIX,
            Self::Watermark => keys::SYNC_WATERMARK_PREFIX,
            Self::Tombstone => keys::SYNC_TOMBSTONE_PREFIX,
            Self::EntryState => keys::SYNC_ENTRY_STATE_PREFIX,
            Self::Conflict => keys::SYNC_CONFLICT_PREFIX,
            Self::CorruptRemote => keys::SYNC_CORRUPT_REMOTE_PREFIX,
        }
    }
}

#[allow(dead_code)]
impl InventoryDb {
    pub(crate) fn sync_watermark(&self, client_id: &str) -> CommandResult<Option<u64>> {
        let key = keys::sync_watermark_key(client_id)?;
        self.get_u64(key.as_bytes(), "sync_watermark")
    }

    pub(crate) fn set_sync_watermark(&self, client_id: &str, local_seq: u64) -> CommandResult<()> {
        keys::validate_local_seq(local_seq)?;
        let key = keys::sync_watermark_key(client_id)?;
        self.put_bytes(key.as_bytes(), local_seq.to_string().as_bytes())
    }

    pub(crate) fn clear_sync_watermark(&self, client_id: &str) -> CommandResult<()> {
        let key = keys::sync_watermark_key(client_id)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn scan_sync_watermarks<F>(&self, limit: usize, mut visit: F) -> CommandResult<()>
    where
        F: FnMut(String, u64) -> CommandResult<bool>,
    {
        self.scan_sync_prefix_from(
            keys::SYNC_WATERMARK_PREFIX,
            keys::SYNC_WATERMARK_PREFIX.as_bytes().to_vec(),
            limit,
            |key, value| {
                let client_id =
                    keys::parse_segment_from_key(keys::SYNC_WATERMARK_PREFIX, key, "client_id")?;
                let local_seq = decode_u64_value(value, "sync_watermark")?;
                visit(client_id, local_seq)
            },
        )
    }

    pub(crate) fn put_sync_outbox_record<T: Serialize>(
        &self,
        local_seq: u64,
        record: &T,
    ) -> CommandResult<bool> {
        let key = keys::sync_outbox_key(local_seq)?;
        self.put_json(key.as_bytes(), record)
    }

    pub(crate) fn sync_outbox_record<T: DeserializeOwned>(
        &self,
        local_seq: u64,
    ) -> CommandResult<Option<T>> {
        let key = keys::sync_outbox_key(local_seq)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn delete_sync_outbox_record(&self, local_seq: u64) -> CommandResult<()> {
        let key = keys::sync_outbox_key(local_seq)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn scan_sync_outbox_records<T, F>(
        &self,
        start_after_local_seq: Option<u64>,
        limit: usize,
        mut visit: F,
    ) -> CommandResult<()>
    where
        T: DeserializeOwned,
        F: FnMut(u64, T) -> CommandResult<bool>,
    {
        let start_key = match start_after_local_seq {
            Some(local_seq) => {
                keys::next_entry_range_start(keys::sync_outbox_key(local_seq)?.into_bytes())
            }
            None => keys::SYNC_OUTBOX_PREFIX.as_bytes().to_vec(),
        };

        self.scan_sync_prefix_from(keys::SYNC_OUTBOX_PREFIX, start_key, limit, |key, value| {
            let local_seq = keys::parse_local_seq_from_key(keys::SYNC_OUTBOX_PREFIX, key)?;
            let record = serde_json::from_slice(value).map_err(db_error)?;
            visit(local_seq, record)
        })
    }

    pub(crate) fn put_sync_applied_marker<T: Serialize>(
        &self,
        op_id: &str,
        marker: &T,
    ) -> CommandResult<bool> {
        let key = keys::sync_applied_key(op_id)?;
        self.put_json(key.as_bytes(), marker)
    }

    pub(crate) fn sync_applied_marker<T: DeserializeOwned>(
        &self,
        op_id: &str,
    ) -> CommandResult<Option<T>> {
        let key = keys::sync_applied_key(op_id)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn has_sync_applied_marker(&self, op_id: &str) -> CommandResult<bool> {
        let key = keys::sync_applied_key(op_id)?;
        Ok(self.store.contains_key(key.as_bytes()))
    }

    pub(crate) fn delete_sync_applied_marker(&self, op_id: &str) -> CommandResult<()> {
        let key = keys::sync_applied_key(op_id)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn put_sync_client_seq_marker<T: Serialize>(
        &self,
        client_id: &str,
        local_seq: u64,
        marker: &T,
    ) -> CommandResult<bool> {
        let key = keys::sync_client_seq_key(client_id, local_seq)?;
        self.put_json(key.as_bytes(), marker)
    }

    pub(crate) fn sync_client_seq_marker<T: DeserializeOwned>(
        &self,
        client_id: &str,
        local_seq: u64,
    ) -> CommandResult<Option<T>> {
        let key = keys::sync_client_seq_key(client_id, local_seq)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn delete_sync_client_seq_marker(
        &self,
        client_id: &str,
        local_seq: u64,
    ) -> CommandResult<()> {
        let key = keys::sync_client_seq_key(client_id, local_seq)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn put_sync_tombstone<T: Serialize>(
        &self,
        entry_uuid: &str,
        tombstone: &T,
    ) -> CommandResult<bool> {
        let key = keys::sync_tombstone_key(entry_uuid)?;
        self.put_json(key.as_bytes(), tombstone)
    }

    pub(crate) fn sync_tombstone<T: DeserializeOwned>(
        &self,
        entry_uuid: &str,
    ) -> CommandResult<Option<T>> {
        let key = keys::sync_tombstone_key(entry_uuid)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn has_sync_tombstone(&self, entry_uuid: &str) -> CommandResult<bool> {
        let key = keys::sync_tombstone_key(entry_uuid)?;
        Ok(self.store.contains_key(key.as_bytes()))
    }

    pub(crate) fn delete_sync_tombstone(&self, entry_uuid: &str) -> CommandResult<()> {
        let key = keys::sync_tombstone_key(entry_uuid)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn scan_sync_tombstones<T, F>(&self, limit: usize, mut visit: F) -> CommandResult<()>
    where
        T: DeserializeOwned,
        F: FnMut(String, T) -> CommandResult<bool>,
    {
        self.scan_sync_prefix_from(
            keys::SYNC_TOMBSTONE_PREFIX,
            keys::SYNC_TOMBSTONE_PREFIX.as_bytes().to_vec(),
            limit,
            |key, value| {
                let entry_uuid =
                    keys::parse_segment_from_key(keys::SYNC_TOMBSTONE_PREFIX, key, "entry_uuid")?;
                let tombstone = serde_json::from_slice(value).map_err(db_error)?;
                visit(entry_uuid, tombstone)
            },
        )
    }

    pub(crate) fn put_sync_entry_state<T: Serialize>(
        &self,
        entry_uuid: &str,
        state: &T,
    ) -> CommandResult<bool> {
        let key = keys::sync_entry_state_key(entry_uuid)?;
        self.put_json(key.as_bytes(), state)
    }

    pub(crate) fn sync_entry_state<T: DeserializeOwned>(
        &self,
        entry_uuid: &str,
    ) -> CommandResult<Option<T>> {
        let key = keys::sync_entry_state_key(entry_uuid)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn delete_sync_entry_state(&self, entry_uuid: &str) -> CommandResult<()> {
        let key = keys::sync_entry_state_key(entry_uuid)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn scan_sync_entry_states<T, F>(
        &self,
        limit: usize,
        mut visit: F,
    ) -> CommandResult<()>
    where
        T: DeserializeOwned,
        F: FnMut(String, T) -> CommandResult<bool>,
    {
        self.scan_sync_prefix_from(
            keys::SYNC_ENTRY_STATE_PREFIX,
            keys::SYNC_ENTRY_STATE_PREFIX.as_bytes().to_vec(),
            limit,
            |key, value| {
                let entry_uuid =
                    keys::parse_segment_from_key(keys::SYNC_ENTRY_STATE_PREFIX, key, "entry_uuid")?;
                let state = serde_json::from_slice(value).map_err(db_error)?;
                visit(entry_uuid, state)
            },
        )
    }

    pub(crate) fn put_sync_conflict_record<T: Serialize>(
        &self,
        conflict_id: &str,
        record: &T,
    ) -> CommandResult<bool> {
        let key = keys::sync_conflict_key(conflict_id)?;
        self.put_json(key.as_bytes(), record)
    }

    pub(crate) fn sync_conflict_record<T: DeserializeOwned>(
        &self,
        conflict_id: &str,
    ) -> CommandResult<Option<T>> {
        let key = keys::sync_conflict_key(conflict_id)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn delete_sync_conflict_record(&self, conflict_id: &str) -> CommandResult<()> {
        let key = keys::sync_conflict_key(conflict_id)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn scan_sync_conflict_records<T, F>(
        &self,
        limit: usize,
        mut visit: F,
    ) -> CommandResult<()>
    where
        T: DeserializeOwned,
        F: FnMut(String, T) -> CommandResult<bool>,
    {
        self.scan_sync_prefix_from(
            keys::SYNC_CONFLICT_PREFIX,
            keys::SYNC_CONFLICT_PREFIX.as_bytes().to_vec(),
            limit,
            |key, value| {
                let conflict_id =
                    keys::parse_segment_from_key(keys::SYNC_CONFLICT_PREFIX, key, "conflict_id")?;
                let record = serde_json::from_slice(value).map_err(db_error)?;
                visit(conflict_id, record)
            },
        )
    }

    pub(crate) fn put_sync_corrupt_record<T: Serialize>(
        &self,
        record_id: &str,
        record: &T,
    ) -> CommandResult<bool> {
        let key = keys::sync_corrupt_remote_key(record_id)?;
        self.put_json(key.as_bytes(), record)
    }

    pub(crate) fn sync_corrupt_record<T: DeserializeOwned>(
        &self,
        record_id: &str,
    ) -> CommandResult<Option<T>> {
        let key = keys::sync_corrupt_remote_key(record_id)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn delete_sync_corrupt_record(&self, record_id: &str) -> CommandResult<()> {
        let key = keys::sync_corrupt_remote_key(record_id)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn scan_sync_corrupt_records<T, F>(
        &self,
        limit: usize,
        mut visit: F,
    ) -> CommandResult<()>
    where
        T: DeserializeOwned,
        F: FnMut(String, T) -> CommandResult<bool>,
    {
        self.scan_sync_prefix_from(
            keys::SYNC_CORRUPT_REMOTE_PREFIX,
            keys::SYNC_CORRUPT_REMOTE_PREFIX.as_bytes().to_vec(),
            limit,
            |key, value| {
                let record_id = keys::parse_segment_from_key(
                    keys::SYNC_CORRUPT_REMOTE_PREFIX,
                    key,
                    "record_id",
                )?;
                let record = serde_json::from_slice(value).map_err(db_error)?;
                visit(record_id, record)
            },
        )
    }

    pub(crate) fn scan_sync_range<F>(
        &self,
        keyspace: SyncKeyspace,
        limit: usize,
        visit: F,
    ) -> CommandResult<()>
    where
        F: FnMut(&[u8], &[u8]) -> CommandResult<bool>,
    {
        let prefix = keyspace.prefix();
        self.scan_sync_prefix_from(prefix, prefix.as_bytes().to_vec(), limit, visit)
    }

    pub(crate) fn clear_sync_keyspace(&self, keyspace: SyncKeyspace) -> CommandResult<()> {
        let mut keys = Vec::new();
        self.scan_sync_range(keyspace, usize::MAX, |key, _| {
            keys.push(key.to_vec());
            Ok(true)
        })?;

        for key in keys {
            self.delete_key(&key)?;
        }

        Ok(())
    }

    pub(crate) fn get_sync_value(&self, key: &str) -> CommandResult<Option<Vec<u8>>> {
        keys::validate_sync_key(key)?;
        let key = key.as_bytes();
        if !self.store.contains_key(key) {
            return Ok(None);
        }

        self.store.get(key).map(Some).map_err(db_error)
    }

    pub(crate) fn put_sync_value(&self, key: &str, value: &[u8]) -> CommandResult<()> {
        keys::validate_sync_key(key)?;
        self.put_bytes(key.as_bytes(), value)
    }

    pub(crate) fn delete_sync_value(&self, key: &str) -> CommandResult<()> {
        keys::validate_sync_key(key)?;
        self.delete_key(key.as_bytes())
    }
}
