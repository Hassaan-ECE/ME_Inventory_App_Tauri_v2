use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{
    model::{
        db_error, now_timestamp, numeric_id, CommandResult, InventoryEntry, InventorySharedStatus,
    },
    store::InventoryDb,
};

pub(crate) const SHARED_ROOT_ENV: &str = "ME_LAB_SHARED_ROOT";
pub(crate) const DEFAULT_SHARED_ROOT: &str =
    r"S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME";
pub(crate) const SYNC_SCHEMA_VERSION: u16 = 1;

const OP_FILE_SUFFIX: &str = ".op.json";
const OP_TEMP_MARKER: &str = ".op.json.tmp-";
const LOCAL_SEQ_WIDTH: usize = 12;
const MAX_LOCAL_SEQ: u64 = 999_999_999_999;
const CHECKSUM_PREFIX: &str = "sha256:";
const BOOTSTRAP_COMPLETE_KEY: &str = "meta:sync_bootstrap_complete";
const SHARED_SYNC_INTERVAL_MS: u64 = 10_000;

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
    fn new(kind: SyncCoreErrorKind, message: impl Into<String>) -> Self {
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

pub(crate) fn resolve_shared_root() -> PathBuf {
    resolve_shared_root_from_env_value(env::var_os(SHARED_ROOT_ENV))
}

pub(crate) fn resolved_shared_sync_paths() -> SharedSyncPaths {
    SharedSyncPaths::from_shared_root(resolve_shared_root())
}

pub(crate) fn resolve_shared_root_from_env_value(value: Option<OsString>) -> PathBuf {
    value
        .and_then(|value| {
            let path = value.to_string_lossy().trim().to_string();
            (!path.is_empty()).then_some(PathBuf::from(path))
        })
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SHARED_ROOT))
}

pub(crate) fn ensure_operation_log_layout(paths: &SharedSyncPaths) -> SyncCoreResult<()> {
    fs::create_dir_all(&paths.ops_dir)?;
    fs::create_dir_all(&paths.snapshots_dir)?;
    fs::create_dir_all(&paths.locks_dir)?;
    fs::create_dir_all(&paths.backups_dir)?;
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn get_or_create_client_identity(db: &InventoryDb) -> CommandResult<SyncClientIdentity> {
    db.set_sync_schema_version(SYNC_SCHEMA_VERSION.into())?;
    let client_id = db.get_or_create_client_id()?;
    validate_path_segment(&client_id).map_err(|error| error.message)?;
    let device_id = db.get_or_create_device_id()?;

    db.flush();
    Ok(SyncClientIdentity {
        client_id,
        device_id,
    })
}

#[allow(dead_code)]
pub(crate) fn peek_next_local_sequence(db: &InventoryDb) -> CommandResult<u64> {
    db.next_local_seq()
}

#[allow(dead_code)]
pub(crate) fn allocate_local_sequence(db: &InventoryDb) -> CommandResult<u64> {
    let next_seq = db.reserve_next_local_seq()?;
    db.flush();
    Ok(next_seq)
}

pub(crate) fn build_entry_operation(
    identity: &SyncClientIdentity,
    local_seq: u64,
    operation_type: SyncOperationType,
    entry: InventoryEntry,
    changed_fields: Vec<String>,
    base_version: Option<String>,
) -> SyncCoreResult<SyncOperationEnvelope> {
    let mutation_ts = if entry.updated_at.trim().is_empty() {
        now_timestamp()
    } else {
        entry.updated_at.clone()
    };

    let mut operation = SyncOperationEnvelope {
        schema_version: SYNC_SCHEMA_VERSION,
        op_id: Uuid::new_v4().simple().to_string(),
        client_id: identity.client_id.clone(),
        device_id: identity.device_id.clone(),
        local_seq,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at_utc: now_timestamp(),
        operation_type,
        entity_type: "inventory_entry".to_string(),
        entity_id: entry.entry_uuid.clone(),
        base_version,
        mutation_ts_utc: mutation_ts,
        payload: SyncOperationPayload::entry(entry, changed_fields),
        checksum: String::new(),
    };

    operation.checksum = canonical_operation_checksum(&operation)?;
    Ok(operation)
}

pub(crate) fn build_delete_operation(
    identity: &SyncClientIdentity,
    local_seq: u64,
    entry_uuid: impl Into<String>,
    deleted_at_utc: impl Into<String>,
    base_version: Option<String>,
) -> SyncCoreResult<SyncOperationEnvelope> {
    let entry_uuid = entry_uuid.into();
    let deleted_at_utc = deleted_at_utc.into();
    let mut operation = SyncOperationEnvelope {
        schema_version: SYNC_SCHEMA_VERSION,
        op_id: Uuid::new_v4().simple().to_string(),
        client_id: identity.client_id.clone(),
        device_id: identity.device_id.clone(),
        local_seq,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at_utc: now_timestamp(),
        operation_type: SyncOperationType::InventoryEntryDelete,
        entity_type: "inventory_entry".to_string(),
        entity_id: entry_uuid.clone(),
        base_version,
        mutation_ts_utc: deleted_at_utc.clone(),
        payload: SyncOperationPayload::delete(entry_uuid, deleted_at_utc),
        checksum: String::new(),
    };

    operation.checksum = canonical_operation_checksum(&operation)?;
    Ok(operation)
}

pub(crate) fn canonical_operation_checksum(
    operation: &SyncOperationEnvelope,
) -> SyncCoreResult<String> {
    let bytes = canonical_json_bytes_without_checksum(operation)?;
    Ok(format!("{CHECKSUM_PREFIX}{}", sha256_hex(&bytes)))
}

pub(crate) fn canonical_operation_json(
    operation: &SyncOperationEnvelope,
) -> SyncCoreResult<Vec<u8>> {
    canonical_json_bytes(operation)
}

pub(crate) fn operation_file_path(
    paths: &SharedSyncPaths,
    client_id: &str,
    local_seq: u64,
) -> SyncCoreResult<PathBuf> {
    validate_path_segment(client_id)?;
    validate_local_seq(local_seq)?;
    Ok(paths
        .ops_dir
        .join(client_id)
        .join(operation_file_name(local_seq)))
}

pub(crate) fn write_operation_file(
    paths: &SharedSyncPaths,
    operation: &SyncOperationEnvelope,
) -> SyncCoreResult<PathBuf> {
    validate_operation_for_write(operation)?;

    let expected_checksum = canonical_operation_checksum(operation)?;
    if operation.checksum != expected_checksum {
        return Err(SyncCoreError::new(
            SyncCoreErrorKind::ChecksumMismatch,
            "Operation checksum does not match its canonical JSON payload.",
        ));
    }

    let final_path = operation_file_path(paths, &operation.client_id, operation.local_seq)?;
    if final_path.exists() {
        return validate_existing_operation_file(&final_path, operation);
    }

    let parent = final_path.parent().ok_or_else(|| {
        SyncCoreError::new(
            SyncCoreErrorKind::InvalidPathSegment,
            "Operation file path does not have a parent directory.",
        )
    })?;
    fs::create_dir_all(parent)?;

    let temp_path = parent.join(format!(
        "{}.tmp-{}-{}",
        operation_file_name(operation.local_seq),
        process::id(),
        Uuid::new_v4().simple()
    ));
    let bytes = canonical_operation_json(operation)?;

    let write_result = (|| -> SyncCoreResult<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, &final_path)?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    write_result.map(|_| final_path)
}

#[allow(dead_code)]
pub(crate) fn read_operation_file(path: &Path) -> Result<SyncOperationEnvelope, CorruptRemoteFile> {
    let file_name = file_name_string(path).ok_or_else(|| {
        corrupt_without_content(
            path,
            CorruptRemoteReason::InvalidFileName,
            "Operation file path has no file name.",
        )
    })?;
    let expected_seq = parse_operation_file_name(&file_name).map_err(|detail| {
        corrupt_without_content(path, CorruptRemoteReason::InvalidFileName, detail)
    })?;
    let expected_client_id = path
        .parent()
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| {
            corrupt_without_content(
                path,
                CorruptRemoteReason::InvalidFileName,
                "Operation file path has no client directory.",
            )
        })?;

    read_operation_file_for_identity(path, &expected_client_id, expected_seq)
}

pub(crate) fn read_operation_file_for_identity(
    path: &Path,
    expected_client_id: &str,
    expected_seq: u64,
) -> Result<SyncOperationEnvelope, CorruptRemoteFile> {
    let bytes = fs::read(path).map_err(|error| {
        corrupt_without_content(path, CorruptRemoteReason::Io, error.to_string())
    })?;

    let operation: SyncOperationEnvelope = serde_json::from_slice(&bytes).map_err(|error| {
        corrupt_with_content(
            path,
            CorruptRemoteReason::MalformedJson,
            error.to_string(),
            &bytes,
        )
    })?;

    if operation.schema_version != SYNC_SCHEMA_VERSION {
        return Err(corrupt_with_content(
            path,
            CorruptRemoteReason::UnsupportedSchemaVersion,
            format!(
                "Unsupported sync schema version {}.",
                operation.schema_version
            ),
            &bytes,
        ));
    }

    if operation.client_id != expected_client_id {
        return Err(corrupt_with_content(
            path,
            CorruptRemoteReason::ClientIdMismatch,
            format!(
                "Operation client_id '{}' does not match folder '{}'.",
                operation.client_id, expected_client_id
            ),
            &bytes,
        ));
    }

    if operation.local_seq != expected_seq {
        return Err(corrupt_with_content(
            path,
            CorruptRemoteReason::LocalSeqMismatch,
            format!(
                "Operation local_seq {} does not match file sequence {}.",
                operation.local_seq, expected_seq
            ),
            &bytes,
        ));
    }

    if operation.entity_type != "inventory_entry" || operation.entity_id.trim().is_empty() {
        return Err(corrupt_with_content(
            path,
            CorruptRemoteReason::InvalidEnvelope,
            "Operation envelope has an invalid entity reference.",
            &bytes,
        ));
    }

    if let Err(detail) = validate_operation_payload_identity(&operation) {
        return Err(corrupt_with_content(
            path,
            CorruptRemoteReason::InvalidEnvelope,
            detail,
            &bytes,
        ));
    }

    let expected_checksum = canonical_operation_checksum(&operation).map_err(|error| {
        corrupt_with_content(
            path,
            CorruptRemoteReason::InvalidEnvelope,
            error.to_string(),
            &bytes,
        )
    })?;
    if operation.checksum != expected_checksum {
        return Err(corrupt_with_content(
            path,
            CorruptRemoteReason::InvalidChecksum,
            "Operation checksum does not match canonical JSON without checksum.",
            &bytes,
        ));
    }

    Ok(operation)
}

pub(crate) fn scan_operation_files(paths: &SharedSyncPaths) -> SyncCoreResult<OperationScanReport> {
    let mut report = OperationScanReport::default();
    if !paths.ops_dir.exists() {
        return Ok(report);
    }

    let mut seen_sequences: HashMap<(String, u64), String> = HashMap::new();

    for client_dir in fs::read_dir(&paths.ops_dir)? {
        let client_dir = client_dir?;
        if !client_dir.file_type()?.is_dir() {
            report.ignored_unknown_files += 1;
            continue;
        }

        let client_path = client_dir.path();
        let expected_client_id = client_dir.file_name().to_string_lossy().into_owned();

        for file in fs::read_dir(&client_path)? {
            let file = file?;
            if !file.file_type()?.is_file() {
                report.ignored_unknown_files += 1;
                continue;
            }

            let path = file.path();
            let file_name = file.file_name().to_string_lossy().into_owned();
            if is_temp_operation_file_name(&file_name) {
                report.ignored_temp_files += 1;
                continue;
            }

            if !file_name.ends_with(OP_FILE_SUFFIX) {
                report.ignored_unknown_files += 1;
                continue;
            }

            let expected_seq = match parse_operation_file_name(&file_name) {
                Ok(seq) => seq,
                Err(detail) => {
                    report.corrupt.push(corrupt_without_content(
                        &path,
                        CorruptRemoteReason::InvalidFileName,
                        detail,
                    ));
                    continue;
                }
            };

            match read_operation_file_for_identity(&path, &expected_client_id, expected_seq) {
                Ok(operation) => {
                    let sequence_key = (operation.client_id.clone(), operation.local_seq);
                    match seen_sequences.get(&sequence_key) {
                        Some(checksum) if checksum != &operation.checksum => {
                            report.corrupt.push(corrupt_without_content(
                                &path,
                                CorruptRemoteReason::DuplicateSequenceDifferentChecksum,
                                "Duplicate client_id and local_seq has different content.",
                            ));
                        }
                        Some(_) => {}
                        None => {
                            seen_sequences.insert(sequence_key, operation.checksum.clone());
                            report.operations.push(operation);
                        }
                    }
                }
                Err(corrupt) => report.corrupt.push(corrupt),
            }
        }
    }

    report.operations.sort_by(|left, right| {
        left.client_id
            .cmp(&right.client_id)
            .then_with(|| left.local_seq.cmp(&right.local_seq))
            .then_with(|| left.op_id.cmp(&right.op_id))
    });

    Ok(report)
}

pub(crate) fn record_corrupt_remote_file(
    db: &InventoryDb,
    corrupt: &CorruptRemoteFile,
) -> CommandResult<()> {
    put_corrupt_remote_file(db, corrupt)?;
    db.flush();
    Ok(())
}

pub(crate) fn record_corrupt_remote_files(
    db: &InventoryDb,
    corrupt_files: &[CorruptRemoteFile],
) -> CommandResult<usize> {
    for corrupt in corrupt_files {
        put_corrupt_remote_file(db, corrupt)?;
    }

    if !corrupt_files.is_empty() {
        db.flush();
    }

    Ok(corrupt_files.len())
}

pub(crate) fn shared_inventory_status(
    db: &InventoryDb,
    message: impl Into<String>,
) -> InventorySharedStatus {
    let paths = resolved_shared_sync_paths();
    let available = paths.shared_root.exists();
    let pending_count =
        count_pending_local_operations(db, available.then_some(&paths)).unwrap_or(0);
    build_shared_status(db, &paths, available, pending_count, 0, message.into())
}

pub(crate) fn queued_local_status(db: &InventoryDb) -> InventorySharedStatus {
    let paths = resolved_shared_sync_paths();
    let available = paths.shared_root.exists();
    let pending_count =
        count_pending_local_operations(db, available.then_some(&paths)).unwrap_or(1);
    build_shared_status(
        db,
        &paths,
        available,
        pending_count.max(1),
        0,
        "Local change saved and queued for shared sync.".to_string(),
    )
}

pub(crate) fn queue_entry_operation(
    db: &InventoryDb,
    operation_type: SyncOperationType,
    entry: InventoryEntry,
    changed_fields: Vec<String>,
    base_version: Option<String>,
) -> CommandResult<SyncOperationEnvelope> {
    queue_entry_operation_with_revision(
        db,
        operation_type,
        entry,
        changed_fields,
        base_version,
        true,
    )
}

fn queue_entry_operation_with_revision(
    db: &InventoryDb,
    operation_type: SyncOperationType,
    entry: InventoryEntry,
    changed_fields: Vec<String>,
    base_version: Option<String>,
    bump_revision: bool,
) -> CommandResult<SyncOperationEnvelope> {
    db.set_sync_schema_version(SYNC_SCHEMA_VERSION.into())?;
    let identity = local_identity_without_flush(db)?;
    let local_seq = db.reserve_next_local_seq()?;
    let operation = build_entry_operation(
        &identity,
        local_seq,
        operation_type,
        entry,
        changed_fields,
        base_version,
    )
    .map_err(sync_core_error)?;
    persist_local_operation(db, &operation, bump_revision)?;
    Ok(operation)
}

pub(crate) fn queue_delete_operation(
    db: &InventoryDb,
    entry_uuid: impl Into<String>,
    deleted_at_utc: impl Into<String>,
    base_version: Option<String>,
) -> CommandResult<SyncOperationEnvelope> {
    db.set_sync_schema_version(SYNC_SCHEMA_VERSION.into())?;
    let identity = local_identity_without_flush(db)?;
    let local_seq = db.reserve_next_local_seq()?;
    let operation = build_delete_operation(
        &identity,
        local_seq,
        entry_uuid,
        deleted_at_utc,
        base_version,
    )
    .map_err(sync_core_error)?;
    persist_local_operation(db, &operation, true)?;
    Ok(operation)
}

pub(crate) fn run_shared_sync(db: &InventoryDb) -> CommandResult<SharedSyncRunResult> {
    let root = resolve_shared_root();
    run_shared_sync_with_root(db, root)
}

pub(crate) fn run_shared_sync_with_root(
    db: &InventoryDb,
    shared_root: impl Into<PathBuf>,
) -> CommandResult<SharedSyncRunResult> {
    let paths = SharedSyncPaths::from_shared_root(shared_root);
    bootstrap_existing_entries_once(db)?;

    if !paths.shared_root.exists() {
        let pending_count = count_pending_local_operations(db, None)?;
        return Ok(SharedSyncRunResult {
            entries_changed: false,
            shared: build_shared_status(
                db,
                &paths,
                false,
                pending_count,
                0,
                "Shared workspace unavailable. Saving changes locally.".to_string(),
            ),
        });
    }

    if let Err(error) = ensure_operation_log_layout(&paths) {
        let pending_count = count_pending_local_operations(db, None)?;
        return Ok(SharedSyncRunResult {
            entries_changed: false,
            shared: build_shared_status(
                db,
                &paths,
                false,
                pending_count,
                0,
                format!("Shared workspace unavailable. {error}"),
            ),
        });
    }

    let _pushed_count = push_pending_local_operations(db, &paths)?;
    let pull_report = pull_remote_operations(db, &paths)?;
    let pending_count = count_pending_local_operations(db, Some(&paths))?;
    let message = if pull_report.corrupt_count > 0 {
        format!(
            "Shared operation sync ready. Ignored {} corrupt remote file(s).",
            pull_report.corrupt_count
        )
    } else {
        "Shared operation sync ready.".to_string()
    };

    Ok(SharedSyncRunResult {
        entries_changed: pull_report.entries_changed,
        shared: build_shared_status(
            db,
            &paths,
            true,
            pending_count,
            pull_report.corrupt_count,
            message,
        ),
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct PullReport {
    entries_changed: bool,
    corrupt_count: usize,
}

fn bootstrap_existing_entries_once(db: &InventoryDb) -> CommandResult<usize> {
    if db.get_sync_value(BOOTSTRAP_COMPLETE_KEY)?.is_some() {
        return Ok(0);
    }

    let entries = db.load_entries()?;
    let mut queued = 0usize;
    for entry in entries {
        queue_entry_operation_with_revision(
            db,
            SyncOperationType::InventoryEntryCreate,
            entry,
            Vec::new(),
            None,
            false,
        )?;
        queued += 1;
    }

    db.put_sync_value(BOOTSTRAP_COMPLETE_KEY, now_timestamp().as_bytes())?;
    db.flush();
    Ok(queued)
}

fn push_pending_local_operations(
    db: &InventoryDb,
    paths: &SharedSyncPaths,
) -> CommandResult<usize> {
    let mut pushed_count = 0usize;
    db.scan_sync_outbox_records::<SyncOperationEnvelope, _>(None, usize::MAX, |_, operation| {
        match write_operation_file(paths, &operation) {
            Ok(_) => pushed_count += 1,
            Err(error) if error.kind == SyncCoreErrorKind::ExistingOperationConflict => {
                record_operation_file_conflict(db, paths, &operation, error.message)?;
            }
            Err(error) => return Err(sync_core_error(error)),
        }
        Ok(true)
    })?;
    Ok(pushed_count)
}

fn pull_remote_operations(db: &InventoryDb, paths: &SharedSyncPaths) -> CommandResult<PullReport> {
    let scan_report = scan_operation_files(paths).map_err(sync_core_error)?;
    let mut corrupt_count = record_corrupt_remote_files(db, &scan_report.corrupt)?;
    let mut entries_changed = false;
    let mut applied_count = 0usize;

    for operation in scan_report.operations {
        if db.has_sync_applied_marker(&operation.op_id)? {
            continue;
        }

        if let Some(existing_op_id) =
            db.sync_client_seq_marker::<String>(&operation.client_id, operation.local_seq)?
        {
            if existing_op_id != operation.op_id {
                let corrupt = CorruptRemoteFile {
                    path: operation_file_path(paths, &operation.client_id, operation.local_seq)
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_else(|_| {
                            format!("{}:{}", operation.client_id, operation.local_seq)
                        }),
                    reason: CorruptRemoteReason::DuplicateSequenceDifferentChecksum,
                    detail: "Remote operation uses an already-applied client_id/local_seq with a different op_id."
                        .to_string(),
                    detected_at_utc: now_timestamp(),
                    content_sha256: Some(operation.checksum.clone()),
                };
                record_corrupt_remote_file(db, &corrupt)?;
                corrupt_count += 1;
                continue;
            }
        }

        if apply_remote_operation(db, &operation)? {
            entries_changed = true;
        }
        mark_operation_applied(db, &operation)?;
        db.set_sync_watermark(&operation.client_id, operation.local_seq)?;
        applied_count += 1;
    }

    if entries_changed || applied_count > 0 {
        db.flush();
    }

    Ok(PullReport {
        entries_changed,
        corrupt_count,
    })
}

fn apply_remote_operation(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
) -> CommandResult<bool> {
    if let Some(current_state) = current_entry_state(db, &operation.entity_id)? {
        if current_state.last_op_id == operation.op_id {
            return Ok(false);
        }
        if !operation_wins_state(operation, &current_state) {
            record_stale_operation_conflict(db, operation, &current_state)?;
            return Ok(false);
        }
    }

    let entries_changed = match operation.operation_type {
        SyncOperationType::InventoryEntryDelete => apply_remote_delete(db, operation),
        SyncOperationType::InventoryEntryCreate
        | SyncOperationType::InventoryEntryUpdate
        | SyncOperationType::InventoryEntryVerify
        | SyncOperationType::InventoryEntryArchive => apply_remote_upsert(db, operation),
    }?;

    record_entry_state_for_operation(db, operation)?;
    if entries_changed {
        db.increment_sync_revision()?;
    }

    Ok(entries_changed)
}

fn apply_remote_delete(db: &InventoryDb, operation: &SyncOperationEnvelope) -> CommandResult<bool> {
    let entry_uuid = operation
        .payload
        .entry_uuid
        .as_deref()
        .unwrap_or(&operation.entity_id);
    let deleted_at_utc = operation
        .payload
        .deleted_at_utc
        .as_deref()
        .unwrap_or(&operation.mutation_ts_utc);

    let tombstone = SyncTombstoneRecord {
        entry_uuid: entry_uuid.to_string(),
        deleted_at_utc: deleted_at_utc.to_string(),
        op_id: operation.op_id.clone(),
        client_id: operation.client_id.clone(),
        local_seq: operation.local_seq,
        base_version: operation.base_version.clone(),
    };
    db.put_sync_tombstone(entry_uuid, &tombstone)?;

    if let Some(entry) = db.find_entry(entry_uuid)? {
        db.delete_entry(&entry)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn apply_remote_upsert(db: &InventoryDb, operation: &SyncOperationEnvelope) -> CommandResult<bool> {
    let Some(entry) = operation.payload.entry.clone() else {
        return Ok(false);
    };

    if db.has_sync_tombstone(&operation.entity_id)? {
        db.delete_sync_tombstone(&operation.entity_id)?;
    }

    let entry = prepare_incoming_entry(db, entry)?;
    let changed = db
        .find_entry(&entry.entry_uuid)?
        .map(|existing| existing.updated_at != entry.updated_at || existing != entry)
        .unwrap_or(true);
    db.put_entry(&entry)?;
    bump_next_entry_id_after_remote_entry(db, &entry)?;
    Ok(changed)
}

fn current_entry_state(
    db: &InventoryDb,
    entry_uuid: &str,
) -> CommandResult<Option<SyncEntryState>> {
    if let Some(state) = db.sync_entry_state::<SyncEntryState>(entry_uuid)? {
        return Ok(Some(state));
    }

    if let Some(tombstone) = db.sync_tombstone::<SyncTombstoneRecord>(entry_uuid)? {
        return Ok(Some(SyncEntryState {
            entry_uuid: tombstone.entry_uuid,
            last_op_id: tombstone.op_id,
            mutation_ts_utc: tombstone.deleted_at_utc,
            deleted: true,
            source_client_id: tombstone.client_id,
            source_local_seq: tombstone.local_seq,
            operation_type: SyncOperationType::InventoryEntryDelete,
            updated_at_utc: now_timestamp(),
        }));
    }

    let Some(entry) = db.find_entry(entry_uuid)? else {
        return Ok(None);
    };

    Ok(Some(SyncEntryState {
        entry_uuid: entry.entry_uuid,
        last_op_id: String::new(),
        mutation_ts_utc: entry.updated_at,
        deleted: false,
        source_client_id: "local".to_string(),
        source_local_seq: 0,
        operation_type: SyncOperationType::InventoryEntryUpdate,
        updated_at_utc: now_timestamp(),
    }))
}

fn operation_wins_state(operation: &SyncOperationEnvelope, state: &SyncEntryState) -> bool {
    operation.mutation_ts_utc.as_str() > state.mutation_ts_utc.as_str()
        || (operation.mutation_ts_utc == state.mutation_ts_utc
            && operation.op_id.as_str() > state.last_op_id.as_str())
}

fn record_entry_state_for_operation(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
) -> CommandResult<()> {
    let state = SyncEntryState {
        entry_uuid: operation.entity_id.clone(),
        last_op_id: operation.op_id.clone(),
        mutation_ts_utc: operation.mutation_ts_utc.clone(),
        deleted: operation.operation_type == SyncOperationType::InventoryEntryDelete,
        source_client_id: operation.client_id.clone(),
        source_local_seq: operation.local_seq,
        operation_type: operation.operation_type,
        updated_at_utc: now_timestamp(),
    };
    db.put_sync_entry_state(&operation.entity_id, &state)?;
    Ok(())
}

fn record_stale_operation_conflict(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
    current_state: &SyncEntryState,
) -> CommandResult<()> {
    let conflict_id = stale_conflict_id(operation, current_state);
    let record = SyncConflictRecord {
        conflict_id: conflict_id.clone(),
        entry_uuid: operation.entity_id.clone(),
        incoming_op_id: operation.op_id.clone(),
        incoming_client_id: operation.client_id.clone(),
        incoming_local_seq: operation.local_seq,
        incoming_mutation_ts_utc: operation.mutation_ts_utc.clone(),
        current_op_id: current_state.last_op_id.clone(),
        current_client_id: current_state.source_client_id.clone(),
        current_local_seq: current_state.source_local_seq,
        current_mutation_ts_utc: current_state.mutation_ts_utc.clone(),
        reason: SyncConflictReason::StaleIncomingOperation,
        detected_at_utc: now_timestamp(),
    };
    db.put_sync_conflict_record(&conflict_id, &record)?;
    Ok(())
}

fn stale_conflict_id(operation: &SyncOperationEnvelope, current_state: &SyncEntryState) -> String {
    let source = format!(
        "stale:{}:{}:{}:{}",
        operation.entity_id,
        operation.op_id,
        current_state.last_op_id,
        current_state.mutation_ts_utc
    );
    sha256_hex(source.as_bytes())
}

fn prepare_incoming_entry(
    db: &InventoryDb,
    mut entry: InventoryEntry,
) -> CommandResult<InventoryEntry> {
    if let Some(existing) = db.find_entry(&entry.entry_uuid)? {
        entry.id = existing.id;
        entry.database_id = existing.database_id;
        return Ok(entry);
    }

    if entry.id.trim().is_empty() || local_id_belongs_to_different_entry(db, &entry)? {
        let local_id = reserve_unused_entry_id(db)?;
        entry.id = local_id.to_string();
        entry.database_id = Some(local_id);
    }

    Ok(entry)
}

fn local_id_belongs_to_different_entry(
    db: &InventoryDb,
    entry: &InventoryEntry,
) -> CommandResult<bool> {
    if entry.id.trim().is_empty() {
        return Ok(false);
    }

    Ok(db
        .find_entry(&entry.id)?
        .map(|existing| existing.entry_uuid != entry.entry_uuid)
        .unwrap_or(false))
}

fn reserve_unused_entry_id(db: &InventoryDb) -> CommandResult<i64> {
    loop {
        let candidate = db.next_entry_id()?;
        let candidate_text = candidate.to_string();
        if db.find_entry(&candidate_text)?.is_none() {
            db.set_next_entry_id(candidate + 1)?;
            return Ok(candidate);
        }
        db.set_next_entry_id(candidate + 1)?;
    }
}

fn bump_next_entry_id_after_remote_entry(
    db: &InventoryDb,
    entry: &InventoryEntry,
) -> CommandResult<()> {
    let entry_id = numeric_id(&entry.id);
    if entry_id > 0 && entry_id >= db.next_entry_id()? {
        db.set_next_entry_id(entry_id + 1)?;
    }
    Ok(())
}

fn persist_local_operation(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
    bump_revision: bool,
) -> CommandResult<()> {
    db.put_sync_outbox_record(operation.local_seq, operation)?;
    mark_operation_applied(db, operation)?;

    if operation.operation_type == SyncOperationType::InventoryEntryDelete {
        let tombstone = SyncTombstoneRecord {
            entry_uuid: operation.entity_id.clone(),
            deleted_at_utc: operation
                .payload
                .deleted_at_utc
                .clone()
                .unwrap_or_else(|| operation.mutation_ts_utc.clone()),
            op_id: operation.op_id.clone(),
            client_id: operation.client_id.clone(),
            local_seq: operation.local_seq,
            base_version: operation.base_version.clone(),
        };
        db.put_sync_tombstone(&operation.entity_id, &tombstone)?;
    } else if db.has_sync_tombstone(&operation.entity_id)? {
        db.delete_sync_tombstone(&operation.entity_id)?;
    }

    record_entry_state_for_operation(db, operation)?;
    if bump_revision {
        db.increment_sync_revision()?;
    }

    Ok(())
}

fn mark_operation_applied(
    db: &InventoryDb,
    operation: &SyncOperationEnvelope,
) -> CommandResult<()> {
    let marker = SyncAppliedMarker {
        op_id: operation.op_id.clone(),
        client_id: operation.client_id.clone(),
        local_seq: operation.local_seq,
        checksum: operation.checksum.clone(),
        applied_at_utc: now_timestamp(),
    };
    db.put_sync_applied_marker(&operation.op_id, &marker)?;
    db.put_sync_client_seq_marker(&operation.client_id, operation.local_seq, &operation.op_id)?;
    Ok(())
}

fn local_identity_without_flush(db: &InventoryDb) -> CommandResult<SyncClientIdentity> {
    let client_id = db.get_or_create_client_id()?;
    validate_path_segment(&client_id).map_err(sync_core_error)?;
    let device_id = db.get_or_create_device_id()?;
    Ok(SyncClientIdentity {
        client_id,
        device_id,
    })
}

fn count_pending_local_operations(
    db: &InventoryDb,
    paths: Option<&SharedSyncPaths>,
) -> CommandResult<usize> {
    let mut count = 0usize;
    db.scan_sync_outbox_records::<SyncOperationEnvelope, _>(None, usize::MAX, |_, operation| {
        let written = paths
            .map(|paths| operation_file_matches(paths, &operation))
            .unwrap_or(false);
        if !written {
            count += 1;
        }
        Ok(true)
    })?;
    Ok(count)
}

fn operation_file_matches(paths: &SharedSyncPaths, operation: &SyncOperationEnvelope) -> bool {
    let Ok(path) = operation_file_path(paths, &operation.client_id, operation.local_seq) else {
        return false;
    };
    if !path.exists() {
        return false;
    }

    read_operation_file_for_identity(&path, &operation.client_id, operation.local_seq)
        .map(|existing| {
            existing.checksum == operation.checksum && existing.op_id == operation.op_id
        })
        .unwrap_or(false)
}

fn record_operation_file_conflict(
    db: &InventoryDb,
    paths: &SharedSyncPaths,
    operation: &SyncOperationEnvelope,
    detail: String,
) -> CommandResult<()> {
    let path = operation_file_path(paths, &operation.client_id, operation.local_seq)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| format!("{}:{}", operation.client_id, operation.local_seq));
    let corrupt = CorruptRemoteFile {
        path,
        reason: CorruptRemoteReason::DuplicateSequenceDifferentChecksum,
        detail,
        detected_at_utc: now_timestamp(),
        content_sha256: Some(operation.checksum.clone()),
    };
    record_corrupt_remote_file(db, &corrupt)
}

fn build_shared_status(
    db: &InventoryDb,
    paths: &SharedSyncPaths,
    available: bool,
    pending_count: usize,
    corrupt_count: usize,
    message: String,
) -> InventorySharedStatus {
    let has_pending = pending_count > 0;
    let mut message = message;
    if has_pending && !message.contains("Pending local") {
        message = format!("{message} Pending local changes: {pending_count}.");
    }
    if corrupt_count > 0 && !message.contains("corrupt") {
        message = format!("{message} Corrupt remote files ignored: {corrupt_count}.");
    }

    InventorySharedStatus {
        available,
        can_modify: true,
        enabled: true,
        has_local_only_changes: Some(has_pending),
        message,
        mutation_mode: if available && !has_pending {
            "shared".to_string()
        } else {
            "local".to_string()
        },
        revision: db.sync_revision().ok().map(|revision| revision.to_string()),
        shared_db_path: None,
        shared_root_path: Some(paths.shared_root.to_string_lossy().into_owned()),
        sync_interval_ms: Some(SHARED_SYNC_INTERVAL_MS),
    }
}

fn sync_core_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn put_corrupt_remote_file(db: &InventoryDb, corrupt: &CorruptRemoteFile) -> CommandResult<()> {
    db.put_sync_corrupt_record(&corrupt_remote_record_id(corrupt), corrupt)?;
    Ok(())
}

pub(crate) fn corrupt_remote_record_id(corrupt: &CorruptRemoteFile) -> String {
    let source = corrupt
        .content_sha256
        .as_deref()
        .unwrap_or(&corrupt.path)
        .as_bytes();
    sha256_hex(source)
}

pub(crate) fn operation_file_name(local_seq: u64) -> String {
    format!(
        "{:0width$}{OP_FILE_SUFFIX}",
        local_seq,
        width = LOCAL_SEQ_WIDTH
    )
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha256_digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push(nibble_to_hex(byte >> 4));
        hex.push(nibble_to_hex(byte & 0x0f));
    }
    hex
}

fn validate_operation_for_write(operation: &SyncOperationEnvelope) -> SyncCoreResult<()> {
    validate_path_segment(&operation.client_id)?;
    validate_local_seq(operation.local_seq)?;
    if operation.schema_version != SYNC_SCHEMA_VERSION {
        return Err(SyncCoreError::new(
            SyncCoreErrorKind::Json,
            format!(
                "Unsupported sync schema version {}.",
                operation.schema_version
            ),
        ));
    }
    if operation.entity_type != "inventory_entry" || operation.entity_id.trim().is_empty() {
        return Err(SyncCoreError::new(
            SyncCoreErrorKind::Json,
            "Operation envelope has an invalid entity reference.",
        ));
    }
    validate_operation_payload_identity(operation).map_err(|detail| {
        SyncCoreError::new(
            SyncCoreErrorKind::InvalidEnvelope,
            format!("Operation envelope payload does not match entity reference: {detail}"),
        )
    })?;
    Ok(())
}

fn validate_operation_payload_identity(operation: &SyncOperationEnvelope) -> Result<(), String> {
    match operation.operation_type {
        SyncOperationType::InventoryEntryDelete => {
            if operation.payload.entry.is_some() {
                return Err("delete operation must not contain an entry payload".to_string());
            }
            if operation.payload.entry_uuid.as_deref() != Some(operation.entity_id.as_str()) {
                return Err("delete payload entry_uuid must match envelope entity_id".to_string());
            }
            if operation
                .payload
                .deleted_at_utc
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                return Err("delete payload deleted_at_utc is required".to_string());
            }
            if !operation.payload.changed_fields.is_empty() {
                return Err("delete operation must not contain changed_fields".to_string());
            }
        }
        SyncOperationType::InventoryEntryCreate
        | SyncOperationType::InventoryEntryUpdate
        | SyncOperationType::InventoryEntryVerify
        | SyncOperationType::InventoryEntryArchive => {
            let Some(entry) = operation.payload.entry.as_ref() else {
                return Err("upsert operation must contain an entry payload".to_string());
            };
            if entry.entry_uuid != operation.entity_id {
                return Err("entry payload entry_uuid must match envelope entity_id".to_string());
            }
            if operation.payload.entry_uuid.is_some() || operation.payload.deleted_at_utc.is_some()
            {
                return Err("upsert operation must not contain delete payload fields".to_string());
            }
        }
    }

    Ok(())
}

fn validate_existing_operation_file(
    path: &Path,
    operation: &SyncOperationEnvelope,
) -> SyncCoreResult<PathBuf> {
    match read_operation_file_for_identity(path, &operation.client_id, operation.local_seq) {
        Ok(existing) if existing.checksum == operation.checksum => Ok(path.to_path_buf()),
        Ok(_) => Err(SyncCoreError::new(
            SyncCoreErrorKind::ExistingOperationConflict,
            "Existing operation file has the same client_id and local_seq but different content.",
        )),
        Err(corrupt) => Err(SyncCoreError::new(
            SyncCoreErrorKind::ExistingOperationConflict,
            format!(
                "Existing operation file is not a valid immutable operation: {}",
                corrupt.detail
            ),
        )),
    }
}

fn validate_local_seq(local_seq: u64) -> SyncCoreResult<()> {
    if (1..=MAX_LOCAL_SEQ).contains(&local_seq) {
        Ok(())
    } else {
        Err(SyncCoreError::new(
            SyncCoreErrorKind::InvalidPathSegment,
            format!("local_seq must be between 1 and {MAX_LOCAL_SEQ}."),
        ))
    }
}

fn validate_path_segment(segment: &str) -> SyncCoreResult<()> {
    let valid = !segment.trim().is_empty()
        && segment != "."
        && segment != ".."
        && segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');

    if valid {
        Ok(())
    } else {
        Err(SyncCoreError::new(
            SyncCoreErrorKind::InvalidPathSegment,
            "Shared sync path segments may only contain ASCII letters, numbers, '-' or '_'.",
        ))
    }
}

fn canonical_json_bytes<T: Serialize>(value: &T) -> SyncCoreResult<Vec<u8>> {
    let value = serde_json::to_value(value)?;
    let value = canonicalize_json_value(value);
    Ok(serde_json::to_vec(&value)?)
}

fn canonical_json_bytes_without_checksum<T: Serialize>(value: &T) -> SyncCoreResult<Vec<u8>> {
    let mut value = serde_json::to_value(value)?;
    if let Value::Object(object) = &mut value {
        object.remove("checksum");
    }
    let value = canonicalize_json_value(value);
    Ok(serde_json::to_vec(&value)?)
}

fn canonicalize_json_value(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(canonicalize_json_value)
                .collect::<Vec<_>>(),
        ),
        Value::Object(object) => {
            let mut keys = object.keys().cloned().collect::<Vec<_>>();
            keys.sort();

            let mut sorted = Map::new();
            for key in keys {
                if let Some(value) = object.get(&key) {
                    sorted.insert(key, canonicalize_json_value(value.clone()));
                }
            }

            Value::Object(sorted)
        }
        value => value,
    }
}

fn is_temp_operation_file_name(file_name: &str) -> bool {
    file_name.contains(OP_TEMP_MARKER) || file_name.ends_with(".tmp")
}

fn parse_operation_file_name(file_name: &str) -> Result<u64, String> {
    let Some(sequence) = file_name.strip_suffix(OP_FILE_SUFFIX) else {
        return Err(format!(
            "Operation file name must end with '{OP_FILE_SUFFIX}'."
        ));
    };

    if sequence.len() != LOCAL_SEQ_WIDTH || !sequence.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!(
            "Operation file sequence must be exactly {LOCAL_SEQ_WIDTH} digits."
        ));
    }

    sequence.parse::<u64>().map_err(db_error)
}

#[allow(dead_code)]
fn file_name_string(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn corrupt_without_content(
    path: &Path,
    reason: CorruptRemoteReason,
    detail: impl Into<String>,
) -> CorruptRemoteFile {
    CorruptRemoteFile {
        path: path.to_string_lossy().into_owned(),
        reason,
        detail: detail.into(),
        detected_at_utc: now_timestamp(),
        content_sha256: None,
    }
}

fn corrupt_with_content(
    path: &Path,
    reason: CorruptRemoteReason,
    detail: impl Into<String>,
    bytes: &[u8],
) -> CorruptRemoteFile {
    CorruptRemoteFile {
        path: path.to_string_lossy().into_owned(),
        reason,
        detail: detail.into(),
        detected_at_utc: now_timestamp(),
        content_sha256: Some(format!("{CHECKSUM_PREFIX}{}", sha256_hex(bytes))),
    }
}

fn nibble_to_hex(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + (nibble - 10)) as char,
        _ => unreachable!("nibble is masked to four bits"),
    }
}

fn sha256_digest(bytes: &[u8]) -> [u8; 32] {
    const H0: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut hash = H0;
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut message = bytes.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks_exact(64) {
        let mut schedule = [0u32; 64];
        for (index, word) in chunk.chunks_exact(4).take(16).enumerate() {
            schedule[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }

        for index in 16..64 {
            let s0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }

        let mut a = hash[0];
        let mut b = hash[1];
        let mut c = hash[2];
        let mut d = hash[3];
        let mut e = hash[4];
        let mut f = hash[5];
        let mut g = hash[6];
        let mut h = hash[7];

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(schedule[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        hash[0] = hash[0].wrapping_add(a);
        hash[1] = hash[1].wrapping_add(b);
        hash[2] = hash[2].wrapping_add(c);
        hash[3] = hash[3].wrapping_add(d);
        hash[4] = hash[4].wrapping_add(e);
        hash[5] = hash[5].wrapping_add(f);
        hash[6] = hash[6].wrapping_add(g);
        hash[7] = hash[7].wrapping_add(h);
    }

    let mut digest = [0u8; 32];
    for (index, value) in hash.into_iter().enumerate() {
        digest[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }

    digest
}
