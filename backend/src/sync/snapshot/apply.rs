use crate::{
    model::CommandResult,
    store::{InventoryDb, SyncKeyspace},
};

use super::super::{SharedSyncPaths, BOOTSTRAP_COMPLETE_KEY};
use super::{
    io::{corruption_count_from_error, read_manifest, read_verified_snapshot, watermarks_to_map},
    types::{
        SharedInventoryManifest, SharedInventorySnapshot, SnapshotApplyReport,
        SNAPSHOT_APPLY_PENDING_KEY,
    },
};

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
