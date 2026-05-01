use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use tauri::Manager;

use crate::{
    model::{now_timestamp, CommandResult},
    store::InventoryDb,
};

const CLEANUP_MARKER_KEY: &str = "meta:deprecated_db_quarantine_complete";
const OLD_DB_STEMS: &[&str] = &["me_inventory", "me_lab_inventory", "me_lab_shared"];
const DB_EXTENSION: &str = ".db";
const BACKUP_DIR_NAME: &str = "deprecated-db-backups";

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub(crate) struct DeprecatedDbQuarantineReport {
    pub moved_files: Vec<String>,
    pub skipped_errors: Vec<String>,
}

pub(crate) fn quarantine_deprecated_databases_once(
    app: &tauri::AppHandle,
    db: &InventoryDb,
) -> CommandResult<DeprecatedDbQuarantineReport> {
    if db.get_sync_value(CLEANUP_MARKER_KEY)?.is_some() {
        return Ok(DeprecatedDbQuarantineReport::default());
    }

    let report = quarantine_deprecated_databases(app)?;
    db.put_sync_value(CLEANUP_MARKER_KEY, now_timestamp().as_bytes())?;
    db.flush();
    Ok(report)
}

fn quarantine_deprecated_databases(
    app: &tauri::AppHandle,
) -> CommandResult<DeprecatedDbQuarantineReport> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let backup_dir = app_data_dir
        .join(BACKUP_DIR_NAME)
        .join(file_safe_timestamp());
    let mut report = DeprecatedDbQuarantineReport::default();

    for candidate in deprecated_database_candidates(app) {
        if !candidate.is_file() {
            continue;
        }

        let Some(file_name) = candidate.file_name().map(|name| name.to_os_string()) else {
            continue;
        };
        fs::create_dir_all(&backup_dir).map_err(|error| error.to_string())?;
        let target = unique_backup_path(&backup_dir.join(file_name));
        match move_file(&candidate, &target) {
            Ok(()) => report
                .moved_files
                .push(candidate.to_string_lossy().into_owned()),
            Err(error) => report
                .skipped_errors
                .push(format!("{}: {error}", candidate.to_string_lossy())),
        }
    }

    Ok(report)
}

fn deprecated_database_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(path) = app.path().app_data_dir() {
        roots.push(path);
    }
    if let Ok(path) = app.path().app_local_data_dir() {
        roots.push(path);
    }
    if let Ok(path) = app.path().app_cache_dir() {
        roots.push(path);
    }

    let mut candidates = Vec::new();
    for root in roots {
        for relative_dir in ["", "data", "resources\\data"] {
            let dir = if relative_dir.is_empty() {
                root.clone()
            } else {
                root.join(relative_dir)
            };
            for stem in OLD_DB_STEMS {
                candidates.push(dir.join(format!("{stem}{DB_EXTENSION}")));
            }
        }
    }

    dedupe_paths(candidates)
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for path in paths {
        let key = path.to_string_lossy().to_lowercase();
        if seen.insert(key) {
            result.push(path);
        }
    }
    result
}

fn unique_backup_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "database".to_string());
    let extension = path
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();

    for index in 1.. {
        let candidate = parent.join(format!("{stem}-{index}{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("unbounded backup path search should always find a free name")
}

fn move_file(source: &Path, target: &Path) -> std::io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            fs::copy(source, target)?;
            fs::remove_file(source).map_err(|remove_error| {
                std::io::Error::new(
                    remove_error.kind(),
                    format!(
                        "{rename_error}; copied backup but could not remove source: {remove_error}"
                    ),
                )
            })
        }
    }
}

fn file_safe_timestamp() -> String {
    now_timestamp()
        .replace(':', "-")
        .replace('.', "-")
        .replace('Z', "z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs};
    use uuid::Uuid;

    #[test]
    fn unique_backup_path_keeps_existing_backup_files() {
        let root = env::temp_dir().join(format!(
            "me-inventory-db-quarantine-{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        let first = root.join(format!("{}{}", OLD_DB_STEMS[0], DB_EXTENSION));
        fs::write(&first, b"old").unwrap();

        let second = unique_backup_path(&first);

        assert_eq!(
            second.file_name().unwrap().to_string_lossy(),
            "me_inventory-1.db"
        );
    }

    #[test]
    fn move_file_falls_back_to_copy_remove_semantics() {
        let root =
            env::temp_dir().join(format!("me-inventory-db-move-{}", Uuid::new_v4().simple()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join(format!("{}{}", OLD_DB_STEMS[1], DB_EXTENSION));
        let target = root
            .join("backup")
            .join(format!("{}{}", OLD_DB_STEMS[1], DB_EXTENSION));
        fs::write(&source, b"old").unwrap();

        move_file(&source, &target).unwrap();

        assert!(!source.exists());
        assert_eq!(fs::read(&target).unwrap(), b"old");
    }
}
