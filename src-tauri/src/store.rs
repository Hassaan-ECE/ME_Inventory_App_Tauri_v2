use feoxdb::FeoxStore;
use serde::Serialize;
use std::{cmp::Ordering, fs, path::PathBuf, sync::Arc};
use tauri::Manager;

use crate::model::{db_error, numeric_id, CommandResult, InventoryEntry};

const INITIAL_DB_SIZE: u64 = 64 * 1024 * 1024;
#[cfg(test)]
const TEST_DB_SIZE: u64 = 2 * 1024 * 1024;
const ENTRY_PREFIX: &str = "entry:";
const ENTRY_RANGE_END: &str = "entry:\u{10ffff}";
const META_NEXT_ID: &[u8] = b"__meta:next_entry_id";
const META_LEGACY_IMPORT_PATH: &[u8] = b"__meta:legacy_import_path";

pub(crate) struct InventoryDb {
    store: Arc<FeoxStore>,
    db_path: PathBuf,
    legacy_sqlite_path: Option<PathBuf>,
}

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
        let mut entries: Vec<InventoryEntry> = self
            .store
            .range_query(
                ENTRY_PREFIX.as_bytes(),
                ENTRY_RANGE_END.as_bytes(),
                usize::MAX,
            )
            .map_err(db_error)?
            .into_iter()
            .map(|(_, value)| decode_entry(&value))
            .collect::<CommandResult<Vec<_>>>()?;

        entries.sort_by(compare_default_entries);
        Ok(entries)
    }

    pub(crate) fn find_entry(&self, entry_id: &str) -> CommandResult<Option<InventoryEntry>> {
        Ok(self
            .load_entries()?
            .into_iter()
            .find(|entry| entry.id == entry_id || entry.entry_uuid == entry_id))
    }

    pub(crate) fn put_entry(&self, entry: &InventoryEntry) -> CommandResult<bool> {
        self.put_json(entry_key(&entry.entry_uuid).as_bytes(), entry)
    }

    pub(crate) fn delete_entry_by_uuid(&self, entry_uuid: &str) -> CommandResult<()> {
        self.store
            .delete(entry_key(entry_uuid).as_bytes())
            .map_err(db_error)
    }

    pub(crate) fn next_entry_id(&self) -> CommandResult<i64> {
        if self.store.contains_key(META_NEXT_ID) {
            let bytes = self.store.get(META_NEXT_ID).map_err(db_error)?;
            let text = String::from_utf8(bytes).map_err(db_error)?;
            if let Ok(next_id) = text.parse::<i64>() {
                return Ok(next_id.max(1));
            }
        }

        Ok(self
            .load_entries()?
            .iter()
            .filter_map(|entry| entry.id.parse::<i64>().ok())
            .max()
            .unwrap_or(0)
            + 1)
    }

    pub(crate) fn set_next_entry_id(&self, next_id: i64) -> CommandResult<()> {
        self.put_bytes(META_NEXT_ID, next_id.to_string().as_bytes())
    }

    fn put_json<T: Serialize>(&self, key: &[u8], value: &T) -> CommandResult<bool> {
        let bytes = serde_json::to_vec(value).map_err(db_error)?;
        self.store.insert(key, &bytes).map_err(db_error)
    }

    fn put_bytes(&self, key: &[u8], value: &[u8]) -> CommandResult<()> {
        self.store.insert(key, value).map_err(db_error)?;
        Ok(())
    }
}

fn decode_entry(value: &[u8]) -> CommandResult<InventoryEntry> {
    serde_json::from_slice(value).map_err(db_error)
}

fn entry_key(entry_uuid: &str) -> String {
    format!("{ENTRY_PREFIX}{entry_uuid}")
}

fn compare_default_entries(left: &InventoryEntry, right: &InventoryEntry) -> Ordering {
    right
        .updated_at
        .cmp(&left.updated_at)
        .then_with(|| numeric_id(&right.id).cmp(&numeric_id(&left.id)))
}
