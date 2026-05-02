use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::model::InventoryEntry;

use super::super::{SyncEntryState, SyncTombstoneRecord};

pub(super) const SNAPSHOT_SCHEMA_VERSION: u16 = 1;
pub(super) const SNAPSHOT_FILE_SUFFIX: &str = ".snapshot.json";
pub(super) const SNAPSHOT_LOCK_FILE: &str = "snapshot.lock";
pub(super) const SNAPSHOT_KEEP_COUNT: usize = 3;
pub(super) const SNAPSHOT_OP_COMPACTION_THRESHOLD: usize = 1_000;
pub(super) const SNAPSHOT_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
pub(super) const MANIFEST_BACKUP_PREFIX: &str = "manifest";
pub(crate) const SNAPSHOT_APPLY_PENDING_KEY: &str = "meta:snapshot_apply_pending";

#[derive(Debug, Clone, Default)]
pub(crate) struct SnapshotApplyReport {
    pub entries_changed: bool,
    pub corrupt_count: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SnapshotPublishReport {
    // Kept for diagnostics even though the current status message only uses corrupt_count.
    #[allow(dead_code)]
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
