use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    model::{db_error, now_timestamp, CommandResult, InventoryEntry},
    store::{InventoryDb, SyncKeyspace},
};

use super::{
    auth,
    operation_file::{parse_operation_file_name, sha256_hex},
    queue::count_pending_local_operations,
    SharedSyncPaths, SyncEntryState, SyncTombstoneRecord, BOOTSTRAP_COMPLETE_KEY, CHECKSUM_PREFIX,
    OP_FILE_SUFFIX, SYNC_SCHEMA_VERSION,
};

const SNAPSHOT_SCHEMA_VERSION: u16 = 1;
const SNAPSHOT_FILE_SUFFIX: &str = ".snapshot.json";
const SNAPSHOT_LOCK_FILE: &str = "snapshot.lock";
const SNAPSHOT_KEEP_COUNT: usize = 3;
const SNAPSHOT_OP_COMPACTION_THRESHOLD: usize = 1_000;
const SNAPSHOT_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const MANIFEST_BACKUP_PREFIX: &str = "manifest";
pub(crate) const SNAPSHOT_APPLY_PENDING_KEY: &str = "meta:snapshot_apply_pending";

#[derive(Debug, Clone, Default)]
pub(crate) struct SnapshotApplyReport {
    pub entries_changed: bool,
    pub corrupt_count: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SnapshotPublishReport {
    pub compacted_operations: usize,
    pub corrupt_count: usize,
    pub snapshot_published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SnapshotWatermark {
    pub client_id: String,
    pub local_seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SharedInventorySnapshot {
    pub schema_version: u16,
    pub sync_schema_version: u16,
    pub snapshot_id: String,
    pub app_version: String,
    pub source_client_id: String,
    pub created_at_utc: String,
    pub entries: Vec<InventoryEntry>,
    pub tombstones: Vec<SyncTombstoneRecord>,
    pub entry_states: Vec<SyncEntryState>,
    pub watermarks: Vec<SnapshotWatermark>,
    pub checksum: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SharedInventoryManifest {
    pub schema_version: u16,
    pub sync_schema_version: u16,
    pub snapshot_id: String,
    pub snapshot_file: String,
    pub snapshot_checksum: String,
    pub app_version: String,
    pub source_client_id: String,
    pub created_at_utc: String,
    pub entry_count: usize,
    pub tombstone_count: usize,
    pub watermarks: Vec<SnapshotWatermark>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
}

pub(crate) fn apply_latest_snapshot_if_safe(
    db: &InventoryDb,
    paths: &SharedSyncPaths,
    pending_count: usize,
) -> CommandResult<SnapshotApplyReport> {
    if pending_count > 0 {
        return Ok(SnapshotApplyReport::default());
    }

    let manifest = match read_manifest(paths) {
        Ok(Some(manifest)) => manifest,
        Ok(None) => return Ok(SnapshotApplyReport::default()),
        Err(error) => {
            return Ok(SnapshotApplyReport {
                entries_changed: false,
                corrupt_count: corruption_count_from_error(error),
            });
        }
    };

    if db.last_snapshot_id()?.as_deref() == Some(manifest.snapshot_id.as_str()) {
        return Ok(SnapshotApplyReport::default());
    }
    if db.has_entries()? && !manifest_covers_local_watermarks(db, &manifest)? {
        return Ok(SnapshotApplyReport::default());
    }

    let snapshot = match read_verified_snapshot(paths, &manifest) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return Ok(SnapshotApplyReport {
                entries_changed: false,
                corrupt_count: corruption_count_from_error(error),
            });
        }
    };

    db.put_sync_value(SNAPSHOT_APPLY_PENDING_KEY, snapshot.snapshot_id.as_bytes())?;
    db.flush();

    let entries_changed = replace_from_snapshot(db, &snapshot)?;
    db.put_sync_value(BOOTSTRAP_COMPLETE_KEY, snapshot.created_at_utc.as_bytes())?;
    db.set_last_snapshot_id(&snapshot.snapshot_id)?;
    if entries_changed {
        db.increment_sync_revision()?;
    }
    db.delete_sync_value(SNAPSHOT_APPLY_PENDING_KEY)?;
    db.flush();

    Ok(SnapshotApplyReport {
        entries_changed,
        corrupt_count: 0,
    })
}

pub(crate) fn maybe_publish_snapshot(
    db: &InventoryDb,
    paths: &SharedSyncPaths,
) -> CommandResult<SnapshotPublishReport> {
    if count_pending_local_operations(db, Some(paths))? > 0 {
        return Ok(SnapshotPublishReport::default());
    }

    let op_count = count_operation_files(paths)?;
    let manifest = match read_manifest(paths) {
        Ok(manifest) => manifest,
        Err(_) => {
            let _ = backup_existing_file(paths, &paths.manifest_path, MANIFEST_BACKUP_PREFIX);
            None
        }
    };

    let should_publish = manifest
        .as_ref()
        .map(|manifest| {
            op_count >= SNAPSHOT_OP_COMPACTION_THRESHOLD
                || (op_count > 0 && manifest_is_old(manifest))
        })
        .unwrap_or(true);
    if !should_publish {
        return Ok(SnapshotPublishReport::default());
    }

    let Some(_lock) = SnapshotLock::try_acquire(paths)? else {
        return Ok(SnapshotPublishReport::default());
    };

    let snapshot = build_snapshot(db)?;
    let snapshot_file = format!("{}{}", snapshot.snapshot_id, SNAPSHOT_FILE_SUFFIX);
    let snapshot_path = paths.snapshots_dir.join(&snapshot_file);
    write_new_json_file(&snapshot_path, &snapshot)?;
    let verified_snapshot = read_verified_snapshot(
        paths,
        &manifest_for_snapshot(&snapshot, snapshot_file.clone()),
    )?;

    let mut manifest = manifest_for_snapshot(&verified_snapshot, snapshot_file);
    sign_manifest_for_configured_trust(&mut manifest)?;
    write_manifest(paths, &manifest)?;
    db.set_last_snapshot_id(&manifest.snapshot_id)?;

    let watermarks = watermarks_to_map(&manifest.watermarks);
    let compacted_operations = compact_covered_operations(paths, &watermarks)?;
    prune_old_snapshots(paths, &manifest.snapshot_file)?;
    db.flush();

    Ok(SnapshotPublishReport {
        compacted_operations,
        corrupt_count: 0,
        snapshot_published: true,
    })
}

fn replace_from_snapshot(
    db: &InventoryDb,
    snapshot: &SharedInventorySnapshot,
) -> CommandResult<bool> {
    db.set_sync_schema_version(snapshot.sync_schema_version.into())?;
    let entries_changed = db.replace_entries_snapshot(&snapshot.entries)?;

    db.clear_sync_keyspace(SyncKeyspace::Tombstone)?;
    for tombstone in &snapshot.tombstones {
        db.put_sync_tombstone(&tombstone.entry_uuid, tombstone)?;
    }

    db.clear_sync_keyspace(SyncKeyspace::EntryState)?;
    for state in &snapshot.entry_states {
        db.put_sync_entry_state(&state.entry_uuid, state)?;
    }

    db.clear_sync_keyspace(SyncKeyspace::Watermark)?;
    for watermark in &snapshot.watermarks {
        if watermark.local_seq > 0 {
            db.set_sync_watermark(&watermark.client_id, watermark.local_seq)?;
        }
    }

    Ok(entries_changed)
}

fn build_snapshot(db: &InventoryDb) -> CommandResult<SharedInventorySnapshot> {
    let mut entries = db.load_entries()?;
    entries.sort_by(|left, right| left.entry_uuid.cmp(&right.entry_uuid));

    let mut tombstones = Vec::new();
    db.scan_sync_tombstones::<SyncTombstoneRecord, _>(usize::MAX, |_, tombstone| {
        tombstones.push(tombstone);
        Ok(true)
    })?;
    tombstones.sort_by(|left, right| left.entry_uuid.cmp(&right.entry_uuid));

    let mut entry_states = Vec::new();
    db.scan_sync_entry_states::<SyncEntryState, _>(usize::MAX, |_, state| {
        entry_states.push(state);
        Ok(true)
    })?;
    entry_states.sort_by(|left, right| left.entry_uuid.cmp(&right.entry_uuid));

    let mut watermarks = collect_watermarks(db)?;
    watermarks.sort_by(|left, right| left.client_id.cmp(&right.client_id));

    let source_client_id = db.get_or_create_client_id()?;
    let mut snapshot = SharedInventorySnapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        sync_schema_version: SYNC_SCHEMA_VERSION,
        snapshot_id: format!("snapshot-{}", Uuid::new_v4().simple()),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        source_client_id,
        created_at_utc: now_timestamp(),
        entries,
        tombstones,
        entry_states,
        watermarks,
        checksum: String::new(),
        auth: None,
    };
    snapshot.checksum = snapshot_checksum(&snapshot)?;
    sign_snapshot_for_configured_trust(&mut snapshot)?;
    Ok(snapshot)
}

fn collect_watermarks(db: &InventoryDb) -> CommandResult<Vec<SnapshotWatermark>> {
    let mut watermarks = Vec::new();
    db.scan_sync_watermarks(usize::MAX, |client_id, local_seq| {
        watermarks.push(SnapshotWatermark {
            client_id,
            local_seq,
        });
        Ok(true)
    })?;
    Ok(watermarks)
}

fn manifest_for_snapshot(
    snapshot: &SharedInventorySnapshot,
    snapshot_file: String,
) -> SharedInventoryManifest {
    SharedInventoryManifest {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        sync_schema_version: snapshot.sync_schema_version,
        snapshot_id: snapshot.snapshot_id.clone(),
        snapshot_file,
        snapshot_checksum: snapshot.checksum.clone(),
        app_version: snapshot.app_version.clone(),
        source_client_id: snapshot.source_client_id.clone(),
        created_at_utc: snapshot.created_at_utc.clone(),
        entry_count: snapshot.entries.len(),
        tombstone_count: snapshot.tombstones.len(),
        watermarks: snapshot.watermarks.clone(),
        auth: None,
    }
}

fn manifest_covers_local_watermarks(
    db: &InventoryDb,
    manifest: &SharedInventoryManifest,
) -> CommandResult<bool> {
    let manifest_watermarks = watermarks_to_map(&manifest.watermarks);
    let mut covers = true;
    db.scan_sync_watermarks(usize::MAX, |client_id, local_seq| {
        if manifest_watermarks
            .get(&client_id)
            .copied()
            .unwrap_or_default()
            < local_seq
        {
            covers = false;
            return Ok(false);
        }
        Ok(true)
    })?;
    Ok(covers)
}

fn read_manifest(paths: &SharedSyncPaths) -> CommandResult<Option<SharedInventoryManifest>> {
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

fn read_verified_snapshot(
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

fn snapshot_checksum(snapshot: &SharedInventorySnapshot) -> CommandResult<String> {
    let mut canonical = snapshot.clone();
    canonical.checksum.clear();
    canonical.auth = None;
    let bytes = serde_json::to_vec(&canonical).map_err(db_error)?;
    Ok(format!("{CHECKSUM_PREFIX}{}", sha256_hex(&bytes)))
}

fn sign_snapshot_for_configured_trust(snapshot: &mut SharedInventorySnapshot) -> CommandResult<()> {
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

fn sign_manifest_for_configured_trust(manifest: &mut SharedInventoryManifest) -> CommandResult<()> {
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

fn write_new_json_file<T: Serialize>(path: &Path, value: &T) -> CommandResult<()> {
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

fn write_manifest(
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

fn compact_covered_operations(
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

fn count_operation_files(paths: &SharedSyncPaths) -> CommandResult<usize> {
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

fn prune_old_snapshots(paths: &SharedSyncPaths, current_snapshot_file: &str) -> CommandResult<()> {
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

fn watermarks_to_map(watermarks: &[SnapshotWatermark]) -> HashMap<String, u64> {
    watermarks
        .iter()
        .map(|watermark| (watermark.client_id.clone(), watermark.local_seq))
        .collect()
}

fn manifest_is_old(manifest: &SharedInventoryManifest) -> bool {
    let Ok(created_at) = DateTime::parse_from_rfc3339(&manifest.created_at_utc) else {
        return true;
    };
    let age = Utc::now().signed_duration_since(created_at.with_timezone(&Utc));
    age.to_std()
        .map(|age| age >= SNAPSHOT_MAX_AGE)
        .unwrap_or(true)
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

fn corruption_count_from_error(_error: String) -> usize {
    1
}

fn backup_existing_file(
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

struct SnapshotLock {
    path: PathBuf,
}

impl SnapshotLock {
    fn try_acquire(paths: &SharedSyncPaths) -> CommandResult<Option<Self>> {
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
