use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
    time::Duration,
};

use crate::model::{db_error, now_timestamp, CommandResult};

use super::{super::SharedSyncPaths, types::SNAPSHOT_LOCK_FILE};

pub(super) struct SnapshotLock {
    path: PathBuf,
}

impl SnapshotLock {
    pub(super) fn try_acquire(paths: &SharedSyncPaths) -> CommandResult<Option<Self>> {
        fs::create_dir_all(&paths.locks_dir).map_err(db_error)?;
        let path = paths.locks_dir.join(SNAPSHOT_LOCK_FILE);
        remove_stale_lock(&path);

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let _ = writeln!(file, "pid={}", process::id());
                let _ = writeln!(file, "createdAtUtc={}", now_timestamp());
                let _ = file.sync_all();
                Ok(Some(Self { path }))
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
}

impl Drop for SnapshotLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn remove_stale_lock(path: &Path) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    let Ok(modified) = metadata.modified() else {
        return;
    };
    let Ok(age) = modified.elapsed() else {
        return;
    };
    if age > Duration::from_secs(10 * 60) {
        let _ = fs::remove_file(path);
    }
}
