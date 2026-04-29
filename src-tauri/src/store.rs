use feoxdb::FeoxStore;
use std::{fs, path::PathBuf, sync::Arc};
use tauri::Manager;

#[path = "store/codec.rs"]
mod codec;
#[path = "store/entries.rs"]
mod entries;
#[path = "store/keys.rs"]
mod keys;
#[path = "store/metadata.rs"]
mod metadata;
#[path = "store/sync_state.rs"]
mod sync_state;
#[cfg(test)]
#[path = "store/tests.rs"]
mod tests;

#[allow(unused_imports)]
pub(crate) use sync_state::{SyncKeyspace, SyncMetadata};

const INITIAL_DB_SIZE: u64 = 64 * 1024 * 1024;
#[cfg(test)]
const TEST_DB_SIZE: u64 = 2 * 1024 * 1024;

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
}
