use feoxdb::FeoxStore;
use serde::{de::DeserializeOwned, Serialize};
use std::{cmp::Ordering, fs, path::PathBuf, sync::Arc};
use tauri::Manager;

use crate::model::{db_error, numeric_id, CommandResult, InventoryEntry};

const INITIAL_DB_SIZE: u64 = 64 * 1024 * 1024;
#[cfg(test)]
const TEST_DB_SIZE: u64 = 2 * 1024 * 1024;
const KEY_RANGE_SENTINEL: &str = "\u{10ffff}";
const ENTRY_PREFIX: &str = "entry:";
const ENTRY_RANGE_END: &str = "entry:\u{10ffff}";
const ENTRY_ID_PREFIX: &str = "entry_id:";
const ENTRY_SCAN_BATCH_LIMIT: usize = 512;
const META_NEXT_ID: &[u8] = b"__meta:next_entry_id";
const META_LEGACY_IMPORT_PATH: &[u8] = b"__meta:legacy_import_path";
const SYNC_META_PREFIX: &str = "meta:";
const SYNC_STATE_PREFIX: &str = "sync:";
#[allow(dead_code)]
const META_SCHEMA_VERSION: &[u8] = b"meta:schema_version";
const META_SYNC_SCHEMA_VERSION: &[u8] = b"meta:sync_schema_version";
const META_CLIENT_ID: &[u8] = b"meta:client_id";
const META_DEVICE_ID: &[u8] = b"meta:device_id";
const META_NEXT_LOCAL_SEQ: &[u8] = b"meta:next_local_seq";
const META_SYNC_REVISION: &[u8] = b"meta:sync_revision";
#[allow(dead_code)]
const META_LAST_SNAPSHOT_ID: &[u8] = b"meta:last_snapshot_id";
const SYNC_OUTBOX_PREFIX: &str = "sync:outbox:";
const SYNC_APPLIED_PREFIX: &str = "sync:applied:";
const SYNC_CLIENT_SEQ_PREFIX: &str = "sync:seq:";
const SYNC_WATERMARK_PREFIX: &str = "sync:watermark:";
const SYNC_TOMBSTONE_PREFIX: &str = "sync:tombstone:";
const SYNC_ENTRY_STATE_PREFIX: &str = "sync:entry_state:";
const SYNC_CONFLICT_PREFIX: &str = "sync:conflict:";
const SYNC_CORRUPT_REMOTE_PREFIX: &str = "sync:corrupt_remote:";
const SYNC_SCAN_BATCH_LIMIT: usize = 512;
const LOCAL_SEQ_KEY_WIDTH: usize = 12;
const MAX_LOCAL_SEQ: u64 = 999_999_999_999;

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
            Self::Outbox => SYNC_OUTBOX_PREFIX,
            Self::Applied => SYNC_APPLIED_PREFIX,
            Self::ClientSeq => SYNC_CLIENT_SEQ_PREFIX,
            Self::Watermark => SYNC_WATERMARK_PREFIX,
            Self::Tombstone => SYNC_TOMBSTONE_PREFIX,
            Self::EntryState => SYNC_ENTRY_STATE_PREFIX,
            Self::Conflict => SYNC_CONFLICT_PREFIX,
            Self::CorruptRemote => SYNC_CORRUPT_REMOTE_PREFIX,
        }
    }
}

#[derive(Clone)]
pub(crate) struct InventoryDb {
    store: Arc<FeoxStore>,
    db_path: PathBuf,
    legacy_sqlite_path: Option<PathBuf>,
}

#[allow(dead_code)]
impl InventoryDb {
    pub(crate) fn open(
        app: &tauri::AppHandle,
        legacy_sqlite_path: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = app.path().app_data_dir()?;
        fs::create_dir_all(&data_dir)?;

        Self::open_with_size(
            data_dir.join("inventory.feox"),
            legacy_sqlite_path,
            INITIAL_DB_SIZE,
        )
    }

    #[cfg(test)]
    pub(crate) fn open_at(
        db_path: PathBuf,
        legacy_sqlite_path: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::open_with_size(db_path, legacy_sqlite_path, TEST_DB_SIZE)
    }

    #[cfg(test)]
    pub(crate) fn open_at_with_size(
        db_path: PathBuf,
        legacy_sqlite_path: Option<PathBuf>,
        file_size: u64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::open_with_size(db_path, legacy_sqlite_path, file_size)
    }

    fn open_with_size(
        db_path: PathBuf,
        legacy_sqlite_path: Option<PathBuf>,
        file_size: u64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let store = FeoxStore::builder()
            .device_path(db_path.to_string_lossy().into_owned())
            .file_size(file_size)
            .build()?;

        Ok(Self {
            store: Arc::new(store),
            db_path,
            legacy_sqlite_path,
        })
    }

    pub(crate) fn flush(&self) {
        self.store.flush_all();
    }

    pub(crate) fn db_path_string(&self) -> String {
        self.db_path.to_string_lossy().into_owned()
    }

    pub(crate) fn legacy_sqlite_path(&self) -> Option<&PathBuf> {
        self.legacy_sqlite_path.as_ref()
    }

    pub(crate) fn has_legacy_import_marker(&self) -> bool {
        self.store.contains_key(META_LEGACY_IMPORT_PATH)
    }

    pub(crate) fn mark_legacy_imported(&self, source_path: &str) -> CommandResult<()> {
        self.put_bytes(META_LEGACY_IMPORT_PATH, source_path.as_bytes())
    }

    pub(crate) fn has_entries(&self) -> CommandResult<bool> {
        Ok(!self
            .store
            .range_query(ENTRY_PREFIX.as_bytes(), ENTRY_RANGE_END.as_bytes(), 1)
            .map_err(db_error)?
            .is_empty())
    }

    pub(crate) fn load_entries(&self) -> CommandResult<Vec<InventoryEntry>> {
        let mut entries = Vec::new();
        self.scan_entries(|entry| {
            entries.push(entry);
            Ok(true)
        })?;

        entries.sort_by(compare_default_entries);
        Ok(entries)
    }

    pub(crate) fn find_entry(&self, entry_id: &str) -> CommandResult<Option<InventoryEntry>> {
        let entry_id = entry_id.trim();
        if entry_id.is_empty() {
            return Ok(None);
        }

        if entry_id.starts_with(ENTRY_PREFIX) {
            if let Some(entry) = self.get_entry_by_key(entry_id.as_bytes())? {
                return Ok(Some(entry));
            }
        }

        if is_numeric_entry_id(entry_id) {
            if let Some(entry) = self.find_entry_by_id(entry_id)? {
                return Ok(Some(entry));
            }
        }

        if let Some(entry) = self.get_entry_by_uuid(entry_id)? {
            return Ok(Some(entry));
        }

        if is_numeric_entry_id(entry_id) {
            Ok(None)
        } else {
            self.find_entry_by_id(entry_id)
        }
    }

    pub(crate) fn put_entry(&self, entry: &InventoryEntry) -> CommandResult<bool> {
        let inserted = self.put_json(entry_key(&entry.entry_uuid).as_bytes(), entry)?;
        self.put_entry_id_index(entry)?;
        Ok(inserted)
    }

    pub(crate) fn delete_entry(&self, entry: &InventoryEntry) -> CommandResult<()> {
        self.store
            .delete(entry_key(&entry.entry_uuid).as_bytes())
            .map_err(db_error)?;
        self.delete_entry_id_index(&entry.id)?;
        Ok(())
    }

    pub(crate) fn next_entry_id(&self) -> CommandResult<i64> {
        if self.store.contains_key(META_NEXT_ID) {
            let bytes = self.store.get(META_NEXT_ID).map_err(db_error)?;
            let text = String::from_utf8(bytes).map_err(db_error)?;
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
        self.put_bytes(META_NEXT_ID, next_id.to_string().as_bytes())
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
        self.get_u32(META_SCHEMA_VERSION, "schema_version")
    }

    pub(crate) fn set_schema_version(&self, version: u32) -> CommandResult<()> {
        self.put_u32(META_SCHEMA_VERSION, "schema_version", version)
    }

    pub(crate) fn sync_schema_version(&self) -> CommandResult<Option<u32>> {
        self.get_u32(META_SYNC_SCHEMA_VERSION, "sync_schema_version")
    }

    pub(crate) fn set_sync_schema_version(&self, version: u32) -> CommandResult<()> {
        self.put_u32(META_SYNC_SCHEMA_VERSION, "sync_schema_version", version)
    }

    pub(crate) fn client_id(&self) -> CommandResult<Option<String>> {
        self.get_string(META_CLIENT_ID)
    }

    pub(crate) fn set_client_id(&self, client_id: &str) -> CommandResult<()> {
        let client_id = normalized_sync_key_segment("client_id", client_id)?;
        self.put_bytes(META_CLIENT_ID, client_id.as_bytes())
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
        self.get_string(META_DEVICE_ID)
    }

    pub(crate) fn set_device_id(&self, device_id: &str) -> CommandResult<()> {
        let device_id = normalized_sync_key_segment("device_id", device_id)?;
        self.put_bytes(META_DEVICE_ID, device_id.as_bytes())
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
        match self.get_u64(META_NEXT_LOCAL_SEQ, "next_local_seq")? {
            Some(next_local_seq) => {
                validate_local_seq(next_local_seq)?;
                Ok(next_local_seq)
            }
            None => Ok(1),
        }
    }

    pub(crate) fn set_next_local_seq(&self, next_local_seq: u64) -> CommandResult<()> {
        validate_local_seq(next_local_seq)?;
        self.put_bytes(META_NEXT_LOCAL_SEQ, next_local_seq.to_string().as_bytes())
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
            .get_u64(META_SYNC_REVISION, "sync_revision")?
            .unwrap_or(0))
    }

    pub(crate) fn increment_sync_revision(&self) -> CommandResult<u64> {
        let next_revision = self
            .sync_revision()?
            .checked_add(1)
            .ok_or_else(|| "sync_revision overflowed".to_string())?;
        self.put_bytes(META_SYNC_REVISION, next_revision.to_string().as_bytes())?;
        Ok(next_revision)
    }

    pub(crate) fn last_snapshot_id(&self) -> CommandResult<Option<String>> {
        self.get_string(META_LAST_SNAPSHOT_ID)
    }

    pub(crate) fn set_last_snapshot_id(&self, snapshot_id: &str) -> CommandResult<()> {
        let snapshot_id = normalized_sync_key_segment("last_snapshot_id", snapshot_id)?;
        self.put_bytes(META_LAST_SNAPSHOT_ID, snapshot_id.as_bytes())
    }

    pub(crate) fn clear_last_snapshot_id(&self) -> CommandResult<()> {
        self.delete_key(META_LAST_SNAPSHOT_ID)
    }

    pub(crate) fn sync_watermark(&self, client_id: &str) -> CommandResult<Option<u64>> {
        let key = sync_watermark_key(client_id)?;
        self.get_u64(key.as_bytes(), "sync_watermark")
    }

    pub(crate) fn set_sync_watermark(&self, client_id: &str, local_seq: u64) -> CommandResult<()> {
        validate_local_seq(local_seq)?;
        let key = sync_watermark_key(client_id)?;
        self.put_bytes(key.as_bytes(), local_seq.to_string().as_bytes())
    }

    pub(crate) fn clear_sync_watermark(&self, client_id: &str) -> CommandResult<()> {
        let key = sync_watermark_key(client_id)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn scan_sync_watermarks<F>(&self, limit: usize, mut visit: F) -> CommandResult<()>
    where
        F: FnMut(String, u64) -> CommandResult<bool>,
    {
        self.scan_sync_prefix_from(
            SYNC_WATERMARK_PREFIX,
            SYNC_WATERMARK_PREFIX.as_bytes().to_vec(),
            limit,
            |key, value| {
                let client_id = parse_segment_from_key(SYNC_WATERMARK_PREFIX, key, "client_id")?;
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
        let key = sync_outbox_key(local_seq)?;
        self.put_json(key.as_bytes(), record)
    }

    pub(crate) fn sync_outbox_record<T: DeserializeOwned>(
        &self,
        local_seq: u64,
    ) -> CommandResult<Option<T>> {
        let key = sync_outbox_key(local_seq)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn delete_sync_outbox_record(&self, local_seq: u64) -> CommandResult<()> {
        let key = sync_outbox_key(local_seq)?;
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
            Some(local_seq) => next_entry_range_start(sync_outbox_key(local_seq)?.into_bytes()),
            None => SYNC_OUTBOX_PREFIX.as_bytes().to_vec(),
        };

        self.scan_sync_prefix_from(SYNC_OUTBOX_PREFIX, start_key, limit, |key, value| {
            let local_seq = parse_local_seq_from_key(SYNC_OUTBOX_PREFIX, key)?;
            let record = serde_json::from_slice(value).map_err(db_error)?;
            visit(local_seq, record)
        })
    }

    pub(crate) fn put_sync_applied_marker<T: Serialize>(
        &self,
        op_id: &str,
        marker: &T,
    ) -> CommandResult<bool> {
        let key = sync_applied_key(op_id)?;
        self.put_json(key.as_bytes(), marker)
    }

    pub(crate) fn sync_applied_marker<T: DeserializeOwned>(
        &self,
        op_id: &str,
    ) -> CommandResult<Option<T>> {
        let key = sync_applied_key(op_id)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn has_sync_applied_marker(&self, op_id: &str) -> CommandResult<bool> {
        let key = sync_applied_key(op_id)?;
        Ok(self.store.contains_key(key.as_bytes()))
    }

    pub(crate) fn delete_sync_applied_marker(&self, op_id: &str) -> CommandResult<()> {
        let key = sync_applied_key(op_id)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn put_sync_client_seq_marker<T: Serialize>(
        &self,
        client_id: &str,
        local_seq: u64,
        marker: &T,
    ) -> CommandResult<bool> {
        let key = sync_client_seq_key(client_id, local_seq)?;
        self.put_json(key.as_bytes(), marker)
    }

    pub(crate) fn sync_client_seq_marker<T: DeserializeOwned>(
        &self,
        client_id: &str,
        local_seq: u64,
    ) -> CommandResult<Option<T>> {
        let key = sync_client_seq_key(client_id, local_seq)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn delete_sync_client_seq_marker(
        &self,
        client_id: &str,
        local_seq: u64,
    ) -> CommandResult<()> {
        let key = sync_client_seq_key(client_id, local_seq)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn put_sync_tombstone<T: Serialize>(
        &self,
        entry_uuid: &str,
        tombstone: &T,
    ) -> CommandResult<bool> {
        let key = sync_tombstone_key(entry_uuid)?;
        self.put_json(key.as_bytes(), tombstone)
    }

    pub(crate) fn sync_tombstone<T: DeserializeOwned>(
        &self,
        entry_uuid: &str,
    ) -> CommandResult<Option<T>> {
        let key = sync_tombstone_key(entry_uuid)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn has_sync_tombstone(&self, entry_uuid: &str) -> CommandResult<bool> {
        let key = sync_tombstone_key(entry_uuid)?;
        Ok(self.store.contains_key(key.as_bytes()))
    }

    pub(crate) fn delete_sync_tombstone(&self, entry_uuid: &str) -> CommandResult<()> {
        let key = sync_tombstone_key(entry_uuid)?;
        self.delete_key(key.as_bytes())
    }

    pub(crate) fn scan_sync_tombstones<T, F>(&self, limit: usize, mut visit: F) -> CommandResult<()>
    where
        T: DeserializeOwned,
        F: FnMut(String, T) -> CommandResult<bool>,
    {
        self.scan_sync_prefix_from(
            SYNC_TOMBSTONE_PREFIX,
            SYNC_TOMBSTONE_PREFIX.as_bytes().to_vec(),
            limit,
            |key, value| {
                let entry_uuid = parse_segment_from_key(SYNC_TOMBSTONE_PREFIX, key, "entry_uuid")?;
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
        let key = sync_entry_state_key(entry_uuid)?;
        self.put_json(key.as_bytes(), state)
    }

    pub(crate) fn sync_entry_state<T: DeserializeOwned>(
        &self,
        entry_uuid: &str,
    ) -> CommandResult<Option<T>> {
        let key = sync_entry_state_key(entry_uuid)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn delete_sync_entry_state(&self, entry_uuid: &str) -> CommandResult<()> {
        let key = sync_entry_state_key(entry_uuid)?;
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
            SYNC_ENTRY_STATE_PREFIX,
            SYNC_ENTRY_STATE_PREFIX.as_bytes().to_vec(),
            limit,
            |key, value| {
                let entry_uuid =
                    parse_segment_from_key(SYNC_ENTRY_STATE_PREFIX, key, "entry_uuid")?;
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
        let key = sync_conflict_key(conflict_id)?;
        self.put_json(key.as_bytes(), record)
    }

    pub(crate) fn sync_conflict_record<T: DeserializeOwned>(
        &self,
        conflict_id: &str,
    ) -> CommandResult<Option<T>> {
        let key = sync_conflict_key(conflict_id)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn delete_sync_conflict_record(&self, conflict_id: &str) -> CommandResult<()> {
        let key = sync_conflict_key(conflict_id)?;
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
            SYNC_CONFLICT_PREFIX,
            SYNC_CONFLICT_PREFIX.as_bytes().to_vec(),
            limit,
            |key, value| {
                let conflict_id = parse_segment_from_key(SYNC_CONFLICT_PREFIX, key, "conflict_id")?;
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
        let key = sync_corrupt_remote_key(record_id)?;
        self.put_json(key.as_bytes(), record)
    }

    pub(crate) fn sync_corrupt_record<T: DeserializeOwned>(
        &self,
        record_id: &str,
    ) -> CommandResult<Option<T>> {
        let key = sync_corrupt_remote_key(record_id)?;
        self.get_json(key.as_bytes())
    }

    pub(crate) fn delete_sync_corrupt_record(&self, record_id: &str) -> CommandResult<()> {
        let key = sync_corrupt_remote_key(record_id)?;
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
            SYNC_CORRUPT_REMOTE_PREFIX,
            SYNC_CORRUPT_REMOTE_PREFIX.as_bytes().to_vec(),
            limit,
            |key, value| {
                let record_id =
                    parse_segment_from_key(SYNC_CORRUPT_REMOTE_PREFIX, key, "record_id")?;
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

    pub(crate) fn get_sync_value(&self, key: &str) -> CommandResult<Option<Vec<u8>>> {
        validate_sync_key(key)?;
        let key = key.as_bytes();
        if !self.store.contains_key(key) {
            return Ok(None);
        }

        self.store.get(key).map(Some).map_err(db_error)
    }

    pub(crate) fn put_sync_value(&self, key: &str, value: &[u8]) -> CommandResult<()> {
        validate_sync_key(key)?;
        self.put_bytes(key.as_bytes(), value)
    }

    fn put_json<T: Serialize>(&self, key: &[u8], value: &T) -> CommandResult<bool> {
        let bytes = serde_json::to_vec(value).map_err(db_error)?;
        self.store.insert(key, &bytes).map_err(db_error)
    }

    fn put_bytes(&self, key: &[u8], value: &[u8]) -> CommandResult<()> {
        self.store.insert(key, value).map_err(db_error)?;
        Ok(())
    }

    fn get_json<T: DeserializeOwned>(&self, key: &[u8]) -> CommandResult<Option<T>> {
        if !self.store.contains_key(key) {
            return Ok(None);
        }

        let value = self.store.get(key).map_err(db_error)?;
        serde_json::from_slice(&value).map(Some).map_err(db_error)
    }

    fn get_string(&self, key: &[u8]) -> CommandResult<Option<String>> {
        if !self.store.contains_key(key) {
            return Ok(None);
        }

        let value = self.store.get(key).map_err(db_error)?;
        String::from_utf8(value).map(Some).map_err(db_error)
    }

    fn get_u32(&self, key: &[u8], label: &str) -> CommandResult<Option<u32>> {
        self.get_string(key)?
            .map(|value| {
                value
                    .parse::<u32>()
                    .map_err(|error| format!("invalid {label}: {error}"))
            })
            .transpose()
    }

    fn put_u32(&self, key: &[u8], label: &str, value: u32) -> CommandResult<()> {
        if value == 0 {
            return Err(format!("{label} must be greater than zero"));
        }

        self.put_bytes(key, value.to_string().as_bytes())
    }

    fn get_u64(&self, key: &[u8], label: &str) -> CommandResult<Option<u64>> {
        self.get_string(key)?
            .map(|value| {
                value
                    .parse::<u64>()
                    .map_err(|error| format!("invalid {label}: {error}"))
            })
            .transpose()
    }

    fn delete_key(&self, key: &[u8]) -> CommandResult<()> {
        if self.store.contains_key(key) {
            self.store.delete(key).map_err(db_error)?;
        }

        Ok(())
    }

    fn scan_sync_prefix_from<F>(
        &self,
        prefix: &str,
        mut start_key: Vec<u8>,
        limit: usize,
        mut visit: F,
    ) -> CommandResult<()>
    where
        F: FnMut(&[u8], &[u8]) -> CommandResult<bool>,
    {
        if limit == 0 {
            return Ok(());
        }

        let range_end = range_end_for_prefix(prefix);
        let prefix_bytes = prefix.as_bytes();
        let mut remaining = limit;

        loop {
            let batch_limit = remaining.min(SYNC_SCAN_BATCH_LIMIT);
            let batch = self
                .store
                .range_query(&start_key, &range_end, batch_limit)
                .map_err(db_error)?;
            if batch.is_empty() {
                return Ok(());
            }

            let is_last_batch = batch.len() < batch_limit;
            let last_key = batch.last().map(|(key, _)| key.clone());

            for (key, value) in batch {
                if !key.starts_with(prefix_bytes) {
                    return Ok(());
                }

                if !visit(&key, &value)? {
                    return Ok(());
                }

                remaining -= 1;
                if remaining == 0 {
                    return Ok(());
                }
            }

            if is_last_batch {
                return Ok(());
            }

            if let Some(last_key) = last_key {
                start_key = next_entry_range_start(last_key);
            }
        }
    }

    fn get_entry_by_uuid(&self, entry_uuid: &str) -> CommandResult<Option<InventoryEntry>> {
        self.get_entry_by_key(entry_key(entry_uuid).as_bytes())
    }

    fn get_entry_by_key(&self, key: &[u8]) -> CommandResult<Option<InventoryEntry>> {
        if !self.store.contains_key(key) {
            return Ok(None);
        }

        let value = self.store.get(key).map_err(db_error)?;
        decode_entry(&value).map(Some)
    }

    fn find_entry_by_id(&self, entry_id: &str) -> CommandResult<Option<InventoryEntry>> {
        if let Some(entry) = self.find_entry_by_id_index(entry_id)? {
            return Ok(Some(entry));
        }

        let mut found = None;
        self.scan_entries(|entry| {
            if entry.id == entry_id {
                self.put_entry_id_index(&entry)?;
                found = Some(entry);
                Ok(false)
            } else {
                Ok(true)
            }
        })?;

        Ok(found)
    }

    fn find_entry_by_id_index(&self, entry_id: &str) -> CommandResult<Option<InventoryEntry>> {
        let key = entry_id_key(entry_id);
        if !self.store.contains_key(key.as_bytes()) {
            return Ok(None);
        }

        let uuid_bytes = self.store.get(key.as_bytes()).map_err(db_error)?;
        let entry_uuid = String::from_utf8(uuid_bytes).map_err(db_error)?;
        let Some(entry) = self.get_entry_by_uuid(&entry_uuid)? else {
            return Ok(None);
        };

        Ok((entry.id == entry_id).then_some(entry))
    }

    fn put_entry_id_index(&self, entry: &InventoryEntry) -> CommandResult<()> {
        if entry.id.is_empty() {
            return Ok(());
        }

        self.put_bytes(
            entry_id_key(&entry.id).as_bytes(),
            entry.entry_uuid.as_bytes(),
        )
    }

    fn delete_entry_id_index(&self, entry_id: &str) -> CommandResult<()> {
        if entry_id.is_empty() {
            return Ok(());
        }

        let key = entry_id_key(entry_id);
        if self.store.contains_key(key.as_bytes()) {
            self.store.delete(key.as_bytes()).map_err(db_error)?;
        }

        Ok(())
    }

    fn scan_entries<F>(&self, mut visit: F) -> CommandResult<()>
    where
        F: FnMut(InventoryEntry) -> CommandResult<bool>,
    {
        let mut start_key = ENTRY_PREFIX.as_bytes().to_vec();

        loop {
            let batch = self
                .store
                .range_query(
                    &start_key,
                    ENTRY_RANGE_END.as_bytes(),
                    ENTRY_SCAN_BATCH_LIMIT,
                )
                .map_err(db_error)?;
            if batch.is_empty() {
                return Ok(());
            }

            let is_last_batch = batch.len() < ENTRY_SCAN_BATCH_LIMIT;
            let last_key = batch.last().map(|(key, _)| key.clone());

            for (_, value) in batch {
                if !visit(decode_entry(&value)?)? {
                    return Ok(());
                }
            }

            if is_last_batch {
                return Ok(());
            }

            if let Some(last_key) = last_key {
                start_key = next_entry_range_start(last_key);
            }
        }
    }
}

fn decode_entry(value: &[u8]) -> CommandResult<InventoryEntry> {
    serde_json::from_slice(value).map_err(db_error)
}

fn entry_key(entry_uuid: &str) -> String {
    format!("{ENTRY_PREFIX}{entry_uuid}")
}

fn entry_id_key(entry_id: &str) -> String {
    format!("{ENTRY_ID_PREFIX}{entry_id}")
}

fn sync_outbox_key(local_seq: u64) -> CommandResult<String> {
    Ok(format!(
        "{SYNC_OUTBOX_PREFIX}{}",
        format_local_seq(local_seq)?
    ))
}

fn sync_applied_key(op_id: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_APPLIED_PREFIX, "op_id", op_id)
}

fn sync_client_seq_key(client_id: &str, local_seq: u64) -> CommandResult<String> {
    let client_id = normalized_sync_key_segment("client_id", client_id)?;
    Ok(format!(
        "{SYNC_CLIENT_SEQ_PREFIX}{client_id}:{}",
        format_local_seq(local_seq)?
    ))
}

fn sync_watermark_key(client_id: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_WATERMARK_PREFIX, "client_id", client_id)
}

fn sync_tombstone_key(entry_uuid: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_TOMBSTONE_PREFIX, "entry_uuid", entry_uuid)
}

fn sync_entry_state_key(entry_uuid: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_ENTRY_STATE_PREFIX, "entry_uuid", entry_uuid)
}

fn sync_conflict_key(conflict_id: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_CONFLICT_PREFIX, "conflict_id", conflict_id)
}

fn sync_corrupt_remote_key(record_id: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_CORRUPT_REMOTE_PREFIX, "record_id", record_id)
}

fn sync_segment_key(prefix: &str, label: &str, value: &str) -> CommandResult<String> {
    let segment = normalized_sync_key_segment(label, value)?;
    Ok(format!("{prefix}{segment}"))
}

fn normalized_sync_key_segment(label: &str, value: &str) -> CommandResult<String> {
    let trimmed = value.trim();
    validate_sync_key_segment(label, trimmed)?;
    Ok(trimmed.to_string())
}

fn validate_sync_key_segment(label: &str, value: &str) -> CommandResult<()> {
    if value.is_empty() {
        return Err(format!("{label} cannot be empty"));
    }

    if value.contains(':') {
        return Err(format!("{label} cannot contain ':'"));
    }

    if value.chars().any(char::is_control) {
        return Err(format!("{label} cannot contain control characters"));
    }

    Ok(())
}

fn validate_local_seq(local_seq: u64) -> CommandResult<()> {
    if local_seq == 0 {
        return Err("local_seq must be greater than zero".to_string());
    }

    if local_seq > MAX_LOCAL_SEQ {
        return Err(format!("local_seq must be at most {MAX_LOCAL_SEQ}"));
    }

    Ok(())
}

fn format_local_seq(local_seq: u64) -> CommandResult<String> {
    validate_local_seq(local_seq)?;
    Ok(format!("{local_seq:0width$}", width = LOCAL_SEQ_KEY_WIDTH))
}

fn parse_local_seq_from_key(prefix: &str, key: &[u8]) -> CommandResult<u64> {
    let key = std::str::from_utf8(key).map_err(db_error)?;
    let Some(local_seq) = key.strip_prefix(prefix) else {
        return Err(format!("invalid {prefix} key"));
    };

    parse_padded_local_seq(local_seq)
}

fn parse_padded_local_seq(value: &str) -> CommandResult<u64> {
    if value.len() != LOCAL_SEQ_KEY_WIDTH || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("local_seq key is not a padded numeric sequence".to_string());
    }

    let local_seq = value
        .parse::<u64>()
        .map_err(|error| format!("invalid local_seq: {error}"))?;
    validate_local_seq(local_seq)?;
    Ok(local_seq)
}

#[allow(dead_code)]
fn parse_segment_from_key(prefix: &str, key: &[u8], label: &str) -> CommandResult<String> {
    let key = std::str::from_utf8(key).map_err(db_error)?;
    let Some(segment) = key.strip_prefix(prefix) else {
        return Err(format!("invalid {prefix} key"));
    };

    validate_sync_key_segment(label, segment)?;
    Ok(segment.to_string())
}

#[allow(dead_code)]
fn decode_u64_value(value: &[u8], label: &str) -> CommandResult<u64> {
    let value = String::from_utf8(value.to_vec()).map_err(db_error)?;
    let local_seq = value
        .parse::<u64>()
        .map_err(|error| format!("invalid {label}: {error}"))?;
    validate_local_seq(local_seq)?;
    Ok(local_seq)
}

fn range_end_for_prefix(prefix: &str) -> Vec<u8> {
    let mut range_end = prefix.as_bytes().to_vec();
    range_end.extend_from_slice(KEY_RANGE_SENTINEL.as_bytes());
    range_end
}

fn is_numeric_entry_id(entry_id: &str) -> bool {
    entry_id.parse::<i64>().is_ok()
}

fn validate_sync_key(key: &str) -> CommandResult<()> {
    if key.starts_with(SYNC_META_PREFIX) || key.starts_with(SYNC_STATE_PREFIX) {
        Ok(())
    } else {
        Err(format!(
            "Sync state keys must start with '{SYNC_META_PREFIX}' or '{SYNC_STATE_PREFIX}'."
        ))
    }
}

fn next_entry_range_start(mut key: Vec<u8>) -> Vec<u8> {
    key.push(0);
    key
}

fn compare_default_entries(left: &InventoryEntry, right: &InventoryEntry) -> Ordering {
    right
        .updated_at
        .cmp(&left.updated_at)
        .then_with(|| numeric_id(&right.id).cmp(&numeric_id(&left.id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs, path::PathBuf};
    use uuid::Uuid;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct TestSyncRecord {
        label: String,
        local_seq: u64,
    }

    #[test]
    fn find_entry_supports_key_uuid_and_legacy_numeric_id() {
        let db = test_db();
        let entry = test_entry("42", "legacy-uuid-42");

        db.put_json(entry_key(&entry.entry_uuid).as_bytes(), &entry)
            .unwrap();

        assert_eq!(
            db.find_entry(&entry_key(&entry.entry_uuid))
                .unwrap()
                .unwrap()
                .id,
            "42"
        );
        assert_eq!(db.find_entry(&entry.entry_uuid).unwrap().unwrap().id, "42");
        assert_eq!(
            db.find_entry("42").unwrap().unwrap().entry_uuid,
            "legacy-uuid-42"
        );
        assert!(db.store.contains_key(entry_id_key("42").as_bytes()));
    }

    #[test]
    fn delete_entry_removes_entry_and_id_index() {
        let db = test_db();
        let entry = test_entry("7", "uuid-7");

        db.put_entry(&entry).unwrap();
        assert!(db
            .store
            .contains_key(entry_key(&entry.entry_uuid).as_bytes()));
        assert!(db.store.contains_key(entry_id_key(&entry.id).as_bytes()));

        db.delete_entry(&entry).unwrap();

        assert!(!db
            .store
            .contains_key(entry_key(&entry.entry_uuid).as_bytes()));
        assert!(!db.store.contains_key(entry_id_key(&entry.id).as_bytes()));
    }

    #[test]
    fn sync_metadata_identity_and_local_seq_are_stable() {
        let db = test_db();

        let metadata = db.sync_metadata().unwrap();
        assert_eq!(metadata.next_local_seq, 1);
        assert_eq!(metadata.schema_version, None);
        assert_eq!(metadata.sync_schema_version, None);
        assert_eq!(metadata.client_id, None);
        assert_eq!(metadata.device_id, None);
        assert_eq!(metadata.last_snapshot_id, None);

        db.set_schema_version(1).unwrap();
        db.set_sync_schema_version(1).unwrap();
        db.set_last_snapshot_id("snapshot-000001").unwrap();

        let client_id = db.get_or_create_client_id().unwrap();
        assert_eq!(db.get_or_create_client_id().unwrap(), client_id);
        let device_id = db.get_or_create_device_id().unwrap();
        assert_eq!(db.get_or_create_device_id().unwrap(), device_id);

        assert!(db.set_next_local_seq(0).is_err());
        assert_eq!(db.reserve_next_local_seq().unwrap(), 1);
        assert_eq!(db.reserve_next_local_seq().unwrap(), 2);
        assert_eq!(db.next_local_seq().unwrap(), 3);

        let metadata = db.sync_metadata().unwrap();
        assert_eq!(metadata.schema_version, Some(1));
        assert_eq!(metadata.sync_schema_version, Some(1));
        assert_eq!(metadata.client_id.as_deref(), Some(client_id.as_str()));
        assert_eq!(metadata.device_id.as_deref(), Some(device_id.as_str()));
        assert_eq!(metadata.next_local_seq, 3);
        assert_eq!(
            metadata.last_snapshot_id.as_deref(),
            Some("snapshot-000001")
        );

        db.clear_last_snapshot_id().unwrap();
        assert_eq!(db.last_snapshot_id().unwrap(), None);
    }

    #[test]
    fn sync_records_round_trip_and_scan_in_key_order() {
        let db = test_db();

        db.put_sync_outbox_record(2, &sync_record("second", 2))
            .unwrap();
        db.put_sync_outbox_record(1, &sync_record("first", 1))
            .unwrap();

        assert_eq!(
            db.sync_outbox_record::<TestSyncRecord>(1).unwrap(),
            Some(sync_record("first", 1))
        );

        let mut outbox = Vec::new();
        db.scan_sync_outbox_records::<TestSyncRecord, _>(None, 10, |local_seq, record| {
            outbox.push((local_seq, record.label));
            Ok(true)
        })
        .unwrap();
        assert_eq!(
            outbox,
            vec![(1, "first".to_string()), (2, "second".to_string())]
        );

        let mut outbox_after_one = Vec::new();
        db.scan_sync_outbox_records::<TestSyncRecord, _>(Some(1), 10, |local_seq, record| {
            outbox_after_one.push((local_seq, record.label));
            Ok(true)
        })
        .unwrap();
        assert_eq!(outbox_after_one, vec![(2, "second".to_string())]);

        db.delete_sync_outbox_record(1).unwrap();
        assert_eq!(db.sync_outbox_record::<TestSyncRecord>(1).unwrap(), None);

        db.put_sync_applied_marker("op-1", &sync_record("applied", 1))
            .unwrap();
        assert!(db.has_sync_applied_marker("op-1").unwrap());
        assert_eq!(
            db.sync_applied_marker::<TestSyncRecord>("op-1")
                .unwrap()
                .unwrap()
                .label,
            "applied"
        );
        db.delete_sync_applied_marker("op-1").unwrap();
        assert!(!db.has_sync_applied_marker("op-1").unwrap());

        db.put_sync_client_seq_marker("client-1", 7, &sync_record("client-seq", 7))
            .unwrap();
        assert_eq!(
            db.sync_client_seq_marker::<TestSyncRecord>("client-1", 7)
                .unwrap()
                .unwrap()
                .label,
            "client-seq"
        );
        db.delete_sync_client_seq_marker("client-1", 7).unwrap();
        assert_eq!(
            db.sync_client_seq_marker::<TestSyncRecord>("client-1", 7)
                .unwrap(),
            None
        );

        db.set_sync_watermark("client-1", 9).unwrap();
        assert_eq!(db.sync_watermark("client-1").unwrap(), Some(9));
        let mut watermarks = Vec::new();
        db.scan_sync_watermarks(10, |client_id, local_seq| {
            watermarks.push((client_id, local_seq));
            Ok(true)
        })
        .unwrap();
        assert_eq!(watermarks, vec![("client-1".to_string(), 9)]);
        db.clear_sync_watermark("client-1").unwrap();
        assert_eq!(db.sync_watermark("client-1").unwrap(), None);

        db.put_sync_tombstone("entry-1", &sync_record("deleted", 8))
            .unwrap();
        assert!(db.has_sync_tombstone("entry-1").unwrap());
        assert_eq!(
            db.sync_tombstone::<TestSyncRecord>("entry-1")
                .unwrap()
                .unwrap()
                .label,
            "deleted"
        );
        let mut tombstones = Vec::new();
        db.scan_sync_tombstones::<TestSyncRecord, _>(10, |entry_uuid, record| {
            tombstones.push((entry_uuid, record.label));
            Ok(true)
        })
        .unwrap();
        assert_eq!(
            tombstones,
            vec![("entry-1".to_string(), "deleted".to_string())]
        );
        db.delete_sync_tombstone("entry-1").unwrap();
        assert!(!db.has_sync_tombstone("entry-1").unwrap());

        db.put_sync_corrupt_record("hash-1", &sync_record("bad-json", 10))
            .unwrap();
        assert_eq!(
            db.sync_corrupt_record::<TestSyncRecord>("hash-1")
                .unwrap()
                .unwrap()
                .label,
            "bad-json"
        );

        let mut corrupt_records = Vec::new();
        db.scan_sync_corrupt_records::<TestSyncRecord, _>(10, |record_id, record| {
            corrupt_records.push((record_id, record.label));
            Ok(true)
        })
        .unwrap();
        assert_eq!(
            corrupt_records,
            vec![("hash-1".to_string(), "bad-json".to_string())]
        );

        let mut raw_outbox_keys = Vec::new();
        db.scan_sync_range(SyncKeyspace::Outbox, 10, |key, _| {
            raw_outbox_keys.push(String::from_utf8(key.to_vec()).unwrap());
            Ok(true)
        })
        .unwrap();
        assert_eq!(raw_outbox_keys, vec!["sync:outbox:000000000002"]);

        db.delete_sync_corrupt_record("hash-1").unwrap();
        assert_eq!(
            db.sync_corrupt_record::<TestSyncRecord>("hash-1").unwrap(),
            None
        );

        for keyspace in [
            SyncKeyspace::Applied,
            SyncKeyspace::ClientSeq,
            SyncKeyspace::Watermark,
            SyncKeyspace::Tombstone,
            SyncKeyspace::Conflict,
            SyncKeyspace::CorruptRemote,
        ] {
            db.scan_sync_range(keyspace, 10, |_, _| Ok(true)).unwrap();
        }
    }

    #[test]
    fn sync_keys_do_not_appear_in_entry_scans() {
        let db = test_db();

        db.set_schema_version(1).unwrap();
        db.set_sync_schema_version(1).unwrap();
        db.set_client_id("client-1").unwrap();
        db.set_device_id("device-1").unwrap();
        db.set_next_local_seq(2).unwrap();
        db.set_sync_watermark("client-1", 1).unwrap();
        db.put_sync_outbox_record(1, &sync_record("pending", 1))
            .unwrap();
        db.put_sync_applied_marker("op-1", &sync_record("applied", 1))
            .unwrap();
        db.put_sync_tombstone("entry-1", &sync_record("deleted", 1))
            .unwrap();
        db.put_sync_corrupt_record("hash-1", &sync_record("corrupt", 1))
            .unwrap();
        db.put_sync_value("meta:test_raw", b"meta").unwrap();
        db.put_sync_value("sync:test_raw", b"sync").unwrap();

        assert!(!db.has_entries().unwrap());
        assert!(db.load_entries().unwrap().is_empty());
        assert_eq!(
            db.get_sync_value("meta:test_raw").unwrap().as_deref(),
            Some(&b"meta"[..])
        );
        assert!(db.put_sync_value("entry:test_raw", b"bad").is_err());

        let entry = test_entry("11", "uuid-11");
        db.put_entry(&entry).unwrap();

        let entries = db.load_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_uuid, "uuid-11");
    }

    fn sync_record(label: &str, local_seq: u64) -> TestSyncRecord {
        TestSyncRecord {
            label: label.to_string(),
            local_seq,
        }
    }

    fn test_entry(id: &str, entry_uuid: &str) -> InventoryEntry {
        InventoryEntry {
            id: id.to_string(),
            database_id: id.parse::<i64>().ok(),
            entry_uuid: entry_uuid.to_string(),
            asset_number: format!("ME-{id}"),
            serial_number: String::new(),
            qty: Some(1.0),
            manufacturer: "Mitutoyo".to_string(),
            model: String::new(),
            description: "Caliper".to_string(),
            project_name: String::new(),
            location: "Lab".to_string(),
            assigned_to: String::new(),
            links: String::new(),
            notes: String::new(),
            lifecycle_status: "active".to_string(),
            working_status: "unknown".to_string(),
            condition: String::new(),
            verified_in_survey: false,
            archived: false,
            manual_entry: false,
            picture_path: String::new(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn test_db() -> InventoryDb {
        let root = unique_test_dir("store");
        fs::create_dir_all(&root).unwrap();
        InventoryDb::open_at(root.join("inventory.feox"), None).unwrap()
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4().simple()))
    }
}
