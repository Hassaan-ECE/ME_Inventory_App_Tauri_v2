use std::{
    env,
    path::{Path, PathBuf},
};
use tauri::Manager;

use super::{DB_FILENAME, LEGACY_DB_FILENAME, LEGACY_SQLITE_ENV};

pub(super) fn legacy_sqlite_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(path) = env::var(LEGACY_SQLITE_ENV) {
        let path = path.trim();
        if !path.is_empty() {
            candidates.push(PathBuf::from(path));
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(project_root) = manifest_dir.parent() {
        candidates.push(project_root.join("data").join(DB_FILENAME));
        candidates.push(project_root.join("data").join(LEGACY_DB_FILENAME));
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join(DB_FILENAME));
        candidates.push(resource_dir.join(LEGACY_DB_FILENAME));
        candidates.push(resource_dir.join("data").join(DB_FILENAME));
        candidates.push(resource_dir.join("data").join(LEGACY_DB_FILENAME));
    }

    if let Ok(current_dir) = env::current_dir() {
        candidates.push(current_dir.join("data").join(DB_FILENAME));
        candidates.push(current_dir.join("data").join(LEGACY_DB_FILENAME));
    }

    dedupe_paths(candidates)
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut result: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !result.iter().any(|existing| same_path(existing, &path)) {
            result.push(path);
        }
    }
    result
}

fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}
