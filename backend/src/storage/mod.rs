use feoxdb::FeoxStore;
use std::{fs, path::PathBuf, sync::Arc};
use tauri::Manager;

mod codec;
mod entries;
mod keys;
mod metadata;
mod sync_state;
#[cfg(test)]
mod tests;

pub(crate) use sync_state::SyncKeyspace;

const INITIAL_DB_SIZE: u64 = 64 * 1024 * 1024;
#[cfg(test)]
const TEST_DB_SIZE: u64 = 2 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct InventoryDb {
    store: Arc<FeoxStore>,
    db_path: PathBuf,
}

impl InventoryDb {
    pub(crate) fn open(app: &tauri::AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = app.path().app_data_dir()?;
        fs::create_dir_all(&data_dir)?;

        Self::open_with_size(data_dir.join("inventory.feox"), INITIAL_DB_SIZE)
    }

    #[cfg(test)]
    pub(crate) fn open_at(db_path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        Self::open_with_size(db_path, TEST_DB_SIZE)
    }

    #[cfg(test)]
    // Used by path-included performance integration tests, not by the library test crate.
    #[allow(dead_code)]
    pub(crate) fn open_at_with_size(
        db_path: PathBuf,
        file_size: u64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::open_with_size(db_path, file_size)
    }

    fn open_with_size(
        db_path: PathBuf,
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
        })
    }

    pub(crate) fn flush(&self) {
        self.store.flush_all();
    }

    pub(crate) fn db_path_string(&self) -> String {
        self.db_path.to_string_lossy().into_owned()
    }
}
