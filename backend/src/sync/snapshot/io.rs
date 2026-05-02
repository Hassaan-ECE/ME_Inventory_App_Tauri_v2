use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
    time::SystemTime,
};

use serde::Serialize;
use uuid::Uuid;

use crate::model::{db_error, now_timestamp, CommandResult};

use super::super::{
    auth,
    operation_file::{parse_operation_file_name, sha256_hex},
    SharedSyncPaths, CHECKSUM_PREFIX, OP_FILE_SUFFIX, SYNC_SCHEMA_VERSION,
};
use super::types::{
    SharedInventoryManifest, SharedInventorySnapshot, SnapshotWatermark, SNAPSHOT_FILE_SUFFIX,
    SNAPSHOT_KEEP_COUNT, SNAPSHOT_SCHEMA_VERSION,
};

pub(super) fn read_manifest(
    paths: &SharedSyncPaths,
) -> CommandResult<Option<SharedInventoryManifest>> {
    if !paths.manifest_path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&paths.manifest_path).map_err(db_error)?;
    let manifest: SharedInventoryManifest = serde_json::from_slice(&bytes).map_err(db_error)?;
    validate_manifest(&manifest)?;
    verify_manifest_auth(&manifest)?;
    Ok(Some(manifest))
}
fn validate_manifest(manifest: &SharedInventoryManifest) -> CommandResult<()> {
    if manifest.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported snapshot manifest schema version {}.",
            manifest.schema_version
        ));
    }
    if manifest.sync_schema_version != SYNC_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported snapshot sync schema version {}.",
            manifest.sync_schema_version
        ));
    }
    if !is_safe_snapshot_file_name(&manifest.snapshot_file) {
        return Err("Snapshot manifest points outside the snapshots folder.".to_string());
    }
    if !manifest.snapshot_checksum.starts_with(CHECKSUM_PREFIX) {
        return Err("Snapshot manifest has an invalid checksum.".to_string());
    }
    Ok(())
}
pub(super) fn read_verified_snapshot(
    paths: &SharedSyncPaths,
    manifest: &SharedInventoryManifest,
) -> CommandResult<SharedInventorySnapshot> {
    validate_manifest(manifest)?;
    let snapshot_path = paths.snapshots_dir.join(&manifest.snapshot_file);
    let bytes = fs::read(&snapshot_path).map_err(db_error)?;
    let snapshot: SharedInventorySnapshot = serde_json::from_slice(&bytes).map_err(db_error)?;

    if snapshot.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported snapshot schema version {}.",
            snapshot.schema_version
        ));
    }
    if snapshot.snapshot_id != manifest.snapshot_id {
        return Err("Snapshot id does not match manifest.".to_string());
    }
    if snapshot.entries.len() != manifest.entry_count {
        return Err("Snapshot entry count does not match manifest.".to_string());
    }
    if snapshot.tombstones.len() != manifest.tombstone_count {
        return Err("Snapshot tombstone count does not match manifest.".to_string());
    }
    let expected_checksum = snapshot_checksum(&snapshot)?;
    if snapshot.checksum != manifest.snapshot_checksum || snapshot.checksum != expected_checksum {
        return Err("Snapshot checksum does not match manifest.".to_string());
    }
    verify_snapshot_auth(&snapshot)?;

    Ok(snapshot)
}
pub(super) fn snapshot_checksum(snapshot: &SharedInventorySnapshot) -> CommandResult<String> {
    let mut canonical = snapshot.clone();
    canonical.checksum.clear();
    canonical.auth = None;
    let bytes = serde_json::to_vec(&canonical).map_err(db_error)?;
    Ok(format!("{CHECKSUM_PREFIX}{}", sha256_hex(&bytes)))
}
pub(super) fn sign_snapshot_for_configured_trust(
    snapshot: &mut SharedInventorySnapshot,
) -> CommandResult<()> {
    snapshot.auth = None;
    let bytes = snapshot_auth_bytes(snapshot)?;
    snapshot.auth = auth::sign_canonical_bytes("sync.snapshot.v1", &bytes)
        .map_err(|error| error.to_string())?;
    Ok(())
}
fn verify_snapshot_auth(snapshot: &SharedInventorySnapshot) -> CommandResult<()> {
    let bytes = snapshot_auth_bytes(snapshot)?;
    auth::verify_canonical_bytes("sync.snapshot.v1", &bytes, snapshot.auth.as_deref())
}
fn snapshot_auth_bytes(snapshot: &SharedInventorySnapshot) -> CommandResult<Vec<u8>> {
    let mut canonical = snapshot.clone();
    canonical.checksum.clear();
    canonical.auth = None;
    serde_json::to_vec(&canonical).map_err(db_error)
}
pub(super) fn sign_manifest_for_configured_trust(
    manifest: &mut SharedInventoryManifest,
) -> CommandResult<()> {
    manifest.auth = None;
    let bytes = manifest_auth_bytes(manifest)?;
    manifest.auth = auth::sign_canonical_bytes("sync.manifest.v1", &bytes)
        .map_err(|error| error.to_string())?;
    Ok(())
}
fn verify_manifest_auth(manifest: &SharedInventoryManifest) -> CommandResult<()> {
    let bytes = manifest_auth_bytes(manifest)?;
    auth::verify_canonical_bytes("sync.manifest.v1", &bytes, manifest.auth.as_deref())
}
fn manifest_auth_bytes(manifest: &SharedInventoryManifest) -> CommandResult<Vec<u8>> {
    let mut canonical = manifest.clone();
    canonical.auth = None;
    serde_json::to_vec(&canonical).map_err(db_error)
}
pub(super) fn write_new_json_file<T: Serialize>(path: &Path, value: &T) -> CommandResult<()> {
    if path.exists() {
        return Err(format!("Refusing to overwrite {}", path.to_string_lossy()));
    }

    let parent = path
        .parent()
        .ok_or_else(|| "Snapshot path has no parent directory.".to_string())?;
    fs::create_dir_all(parent).map_err(db_error)?;
    let temp_path = parent.join(format!(
        "{}.tmp-{}-{}",
        path.file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default(),
        process::id(),
        Uuid::new_v4().simple()
    ));
    write_json_to_temp_then_rename(&temp_path, path, value, false)
}
pub(super) fn write_manifest(
    paths: &SharedSyncPaths,
    manifest: &SharedInventoryManifest,
) -> CommandResult<()> {
    let parent = paths
        .manifest_path
        .parent()
        .ok_or_else(|| "Manifest path has no parent directory.".to_string())?;
    fs::create_dir_all(parent).map_err(db_error)?;
    let temp_path = parent.join(format!(
        "manifest.json.tmp-{}-{}",
        process::id(),
        Uuid::new_v4().simple()
    ));
    write_json_to_temp_then_rename(&temp_path, &paths.manifest_path, manifest, true)
}
fn write_json_to_temp_then_rename<T: Serialize>(
    temp_path: &Path,
    final_path: &Path,
    value: &T,
    replace_existing: bool,
) -> CommandResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(db_error)?;
    let write_result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temp_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        if replace_existing && final_path.exists() {
            fs::remove_file(final_path)?;
        }
        fs::rename(temp_path, final_path)?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(temp_path);
    }

    write_result.map_err(db_error)
}
pub(super) fn compact_covered_operations(
    paths: &SharedSyncPaths,
    watermarks: &HashMap<String, u64>,
) -> CommandResult<usize> {
    let mut removed = 0usize;
    if !paths.ops_dir.exists() {
        return Ok(removed);
    }

    for client_dir in fs::read_dir(&paths.ops_dir).map_err(db_error)? {
        let client_dir = client_dir.map_err(db_error)?;
        if !client_dir.file_type().map_err(db_error)?.is_dir() {
            continue;
        }
        let client_id = client_dir.file_name().to_string_lossy().into_owned();
        let Some(watermark) = watermarks.get(&client_id).copied() else {
            continue;
        };

        for file in fs::read_dir(client_dir.path()).map_err(db_error)? {
            let file = file.map_err(db_error)?;
            if !file.file_type().map_err(db_error)?.is_file() {
                continue;
            }
            let file_name = file.file_name().to_string_lossy().into_owned();
            if !file_name.ends_with(OP_FILE_SUFFIX) {
                continue;
            }
            let Ok(local_seq) = parse_operation_file_name(&file_name) else {
                continue;
            };
            if local_seq <= watermark && fs::remove_file(file.path()).is_ok() {
                removed += 1;
            }
        }

        let _ = fs::remove_dir(client_dir.path());
    }

    Ok(removed)
}
pub(super) fn count_operation_files(paths: &SharedSyncPaths) -> CommandResult<usize> {
    let mut count = 0usize;
    if !paths.ops_dir.exists() {
        return Ok(count);
    }

    for client_dir in fs::read_dir(&paths.ops_dir).map_err(db_error)? {
        let client_dir = client_dir.map_err(db_error)?;
        if !client_dir.file_type().map_err(db_error)?.is_dir() {
            continue;
        }
        for file in fs::read_dir(client_dir.path()).map_err(db_error)? {
            let file = file.map_err(db_error)?;
            if file.file_type().map_err(db_error)?.is_file()
                && file.file_name().to_string_lossy().ends_with(OP_FILE_SUFFIX)
            {
                count += 1;
            }
        }
    }

    Ok(count)
}
pub(super) fn prune_old_snapshots(
    paths: &SharedSyncPaths,
    current_snapshot_file: &str,
) -> CommandResult<()> {
    if !paths.snapshots_dir.exists() {
        return Ok(());
    }

    let mut snapshots = Vec::new();
    for entry in fs::read_dir(&paths.snapshots_dir).map_err(db_error)? {
        let entry = entry.map_err(db_error)?;
        if !entry.file_type().map_err(db_error)?.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name.ends_with(SNAPSHOT_FILE_SUFFIX) {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            snapshots.push((file_name, entry.path(), modified));
        }
    }

    snapshots.sort_by(|left, right| right.2.cmp(&left.2).then_with(|| right.0.cmp(&left.0)));
    for (index, (file_name, path, _)) in snapshots.into_iter().enumerate() {
        if file_name == current_snapshot_file || index < SNAPSHOT_KEEP_COUNT {
            continue;
        }
        let _ = fs::remove_file(path);
    }

    Ok(())
}
pub(super) fn watermarks_to_map(watermarks: &[SnapshotWatermark]) -> HashMap<String, u64> {
    watermarks
        .iter()
        .map(|watermark| (watermark.client_id.clone(), watermark.local_seq))
        .collect()
}
fn is_safe_snapshot_file_name(file_name: &str) -> bool {
    !file_name.trim().is_empty()
        && file_name.ends_with(SNAPSHOT_FILE_SUFFIX)
        && !file_name.contains('/')
        && !file_name.contains('\\')
        && Path::new(file_name)
            .file_name()
            .and_then(|name| name.to_str())
            == Some(file_name)
}
pub(super) fn corruption_count_from_error(_error: String) -> usize {
    1
}
pub(super) fn backup_existing_file(
    paths: &SharedSyncPaths,
    source: &Path,
    prefix: &str,
) -> std::io::Result<Option<PathBuf>> {
    if !source.is_file() {
        return Ok(None);
    }
    fs::create_dir_all(&paths.backups_dir)?;
    let backup_path = paths.backups_dir.join(format!(
        "{}-{}-{}.json",
        prefix,
        now_timestamp()
            .replace(':', "-")
            .replace('.', "-")
            .replace('Z', "z"),
        Uuid::new_v4().simple()
    ));
    fs::rename(source, &backup_path)?;
    Ok(Some(backup_path))
}
