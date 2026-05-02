use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    model::{now_timestamp, CommandResult},
    store::InventoryDb,
};

use super::super::{
    queue::count_pending_local_operations, SharedSyncPaths, SyncEntryState, SyncTombstoneRecord,
    SYNC_SCHEMA_VERSION,
};
use super::{
    io::{
        backup_existing_file, compact_covered_operations, count_operation_files,
        prune_old_snapshots, read_manifest, read_verified_snapshot,
        sign_manifest_for_configured_trust, sign_snapshot_for_configured_trust, snapshot_checksum,
        watermarks_to_map, write_manifest, write_new_json_file,
    },
    lock::SnapshotLock,
    types::{
        SharedInventoryManifest, SharedInventorySnapshot, SnapshotPublishReport, SnapshotWatermark,
        MANIFEST_BACKUP_PREFIX, SNAPSHOT_FILE_SUFFIX, SNAPSHOT_MAX_AGE,
        SNAPSHOT_OP_COMPACTION_THRESHOLD, SNAPSHOT_SCHEMA_VERSION,
    },
};

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
fn manifest_is_old(manifest: &SharedInventoryManifest) -> bool {
    let Ok(created_at) = DateTime::parse_from_rfc3339(&manifest.created_at_utc) else {
        return true;
    };
    let age = Utc::now().signed_duration_since(created_at.with_timezone(&Utc));
    age.to_std()
        .map(|age| age >= SNAPSHOT_MAX_AGE)
        .unwrap_or(true)
}
