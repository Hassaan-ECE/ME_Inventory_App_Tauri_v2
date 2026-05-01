use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::{InventoryEntry, InventorySharedStatus};

pub(crate) const SHARED_ROOT_ENV: &str = "ME_LAB_SHARED_ROOT";
pub(crate) const DEFAULT_SHARED_ROOT: &str =
    r"S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME";
pub(crate) const SYNC_SCHEMA_VERSION: u16 = 1;

pub(super) const OP_FILE_SUFFIX: &str = ".op.json";
pub(super) const OP_TEMP_MARKER: &str = ".op.json.tmp-";
pub(super) const LOCAL_SEQ_WIDTH: usize = 12;
pub(super) const MAX_LOCAL_SEQ: u64 = 999_999_999_999;
pub(super) const CHECKSUM_PREFIX: &str = "sha256:";
pub(super) const BOOTSTRAP_COMPLETE_KEY: &str = "meta:sync_bootstrap_complete";
pub(crate) const SHARED_SYNC_INTERVAL_MS: u64 = 500;

pub(crate) type SyncCoreResult<T> = Result<T, SyncCoreError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SyncCoreErrorKind {
    ChecksumMismatch,
    ExistingOperationConflict,
    InvalidEnvelope,
    InvalidPathSegment,
    Io,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyncCoreError {
    pub kind: SyncCoreErrorKind,
    pub message: String,
}

impl SyncCoreError {
    pub(super) fn new(kind: SyncCoreErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SyncCoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for SyncCoreError {}

impl From<std::io::Error> for SyncCoreError {
    fn from(error: std::io::Error) -> Self {
        Self::new(SyncCoreErrorKind::Io, error.to_string())
    }
}

impl From<serde_json::Error> for SyncCoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(SyncCoreErrorKind::Json, error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyncClientIdentity {
    pub client_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedSyncPaths {
    pub shared_root: PathBuf,
    pub inventory_root: PathBuf,
    pub manifest_path: PathBuf,
    pub ops_dir: PathBuf,
    pub snapshots_dir: PathBuf,
    pub locks_dir: PathBuf,
    pub backups_dir: PathBuf,
}

impl SharedSyncPaths {
    pub(crate) fn from_shared_root(shared_root: impl Into<PathBuf>) -> Self {
        let shared_root = shared_root.into();
        let inventory_root = shared_root.join("shared").join("inventory");

        Self {
            manifest_path: inventory_root.join("manifest.json"),
            ops_dir: inventory_root.join("ops"),
            snapshots_dir: inventory_root.join("snapshots"),
            locks_dir: inventory_root.join("locks"),
            backups_dir: inventory_root.join("backups"),
            inventory_root,
            shared_root,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum SyncOperationType {
    #[serde(rename = "inventory.entry.create")]
    InventoryEntryCreate,
    #[serde(rename = "inventory.entry.update")]
    InventoryEntryUpdate,
    #[serde(rename = "inventory.entry.verify")]
    InventoryEntryVerify,
    #[serde(rename = "inventory.entry.archive")]
    InventoryEntryArchive,
    #[serde(rename = "inventory.entry.delete")]
    InventoryEntryDelete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct SyncOperationPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<InventoryEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at_utc: Option<String>,
}

impl SyncOperationPayload {
    pub(crate) fn entry(entry: InventoryEntry, changed_fields: Vec<String>) -> Self {
        Self {
            entry: Some(entry),
            changed_fields,
            entry_uuid: None,
            deleted_at_utc: None,
        }
    }

    pub(crate) fn delete(entry_uuid: impl Into<String>, deleted_at_utc: impl Into<String>) -> Self {
        Self {
            entry: None,
            changed_fields: Vec::new(),
            entry_uuid: Some(entry_uuid.into()),
            deleted_at_utc: Some(deleted_at_utc.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct SyncOperationEnvelope {
    pub schema_version: u16,
    pub op_id: String,
    pub client_id: String,
    pub device_id: String,
    pub local_seq: u64,
    pub app_version: String,
    pub created_at_utc: String,
    #[serde(rename = "type")]
    pub operation_type: SyncOperationType,
    pub entity_type: String,
    pub entity_id: String,
    pub base_version: Option<String>,
    pub mutation_ts_utc: String,
    pub payload: SyncOperationPayload,
    #[serde(default)]
    pub checksum: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct OperationScanReport {
    pub operations: Vec<SyncOperationEnvelope>,
    pub corrupt: Vec<CorruptRemoteFile>,
    pub ignored_temp_files: usize,
    pub ignored_unknown_files: usize,
    pub ignored_watermarked_files: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct SharedSyncRunResult {
    pub entries_changed: bool,
    pub shared: InventorySharedStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncAppliedMarker {
    pub op_id: String,
    pub client_id: String,
    pub local_seq: u64,
    pub checksum: String,
    pub applied_at_utc: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncTombstoneRecord {
    pub entry_uuid: String,
    pub deleted_at_utc: String,
    pub op_id: String,
    pub client_id: String,
    pub local_seq: u64,
    pub base_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncEntryState {
    pub entry_uuid: String,
    pub last_op_id: String,
    pub mutation_ts_utc: String,
    pub deleted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_version: Option<String>,
    #[serde(default)]
    pub changed_fields: Vec<String>,
    pub source_client_id: String,
    pub source_local_seq: u64,
    pub operation_type: SyncOperationType,
    pub updated_at_utc: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncConflictRecord {
    pub conflict_id: String,
    pub entry_uuid: String,
    pub incoming_op_id: String,
    pub incoming_client_id: String,
    pub incoming_local_seq: u64,
    pub incoming_mutation_ts_utc: String,
    pub current_op_id: String,
    pub current_client_id: String,
    pub current_local_seq: u64,
    pub current_mutation_ts_utc: String,
    pub reason: SyncConflictReason,
    pub detected_at_utc: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SyncConflictReason {
    StaleIncomingOperation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CorruptRemoteReason {
    ClientIdMismatch,
    DuplicateSequenceDifferentChecksum,
    InvalidChecksum,
    InvalidEnvelope,
    InvalidFileName,
    Io,
    LocalSeqMismatch,
    MalformedJson,
    UnsupportedSchemaVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CorruptRemoteFile {
    pub path: String,
    pub reason: CorruptRemoteReason,
    pub detail: String,
    pub detected_at_utc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_sha256: Option<String>,
}
