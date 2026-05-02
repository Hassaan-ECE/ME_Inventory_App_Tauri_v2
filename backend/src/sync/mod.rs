#![allow(dead_code, unused_imports)]

mod apply;
mod auth;
mod conflicts;
mod identity;
mod operation_file;
mod queue;
mod recovery;
mod scanning;
mod shared_paths;
mod snapshot;
mod timestamps;
mod types;

pub(crate) use self::apply::{
    publish_pending_local_changes, run_shared_sync, run_shared_sync_with_root,
};
#[cfg(test)]
pub(crate) use self::auth::set_test_hmac_key;
pub(crate) use self::conflicts::{
    corrupt_remote_record_id, record_corrupt_remote_file, record_corrupt_remote_files,
};
pub(crate) use self::identity::{
    allocate_local_sequence, get_or_create_client_identity, peek_next_local_sequence,
};
pub(crate) use self::operation_file::{
    canonical_operation_checksum, canonical_operation_json, operation_file_name,
    operation_file_path, read_operation_file, read_operation_file_for_identity, sha256_hex,
    write_operation_file,
};
pub(crate) use self::queue::{
    build_delete_operation, build_entry_operation, queue_delete_operation, queue_entry_operation,
};
pub(crate) use self::recovery::{last_local_recovery_message, recover_local_sync_state};
pub(crate) use self::scanning::{scan_operation_files, scan_operation_files_after_watermarks};
pub(crate) use self::shared_paths::{
    ensure_operation_log_layout, queued_local_status, resolve_shared_root,
    resolve_shared_root_from_env_value, resolved_shared_sync_paths, shared_inventory_status,
    startup_inventory_status,
};
pub(crate) use self::snapshot::{
    apply_latest_snapshot_if_safe, maybe_publish_snapshot, SharedInventoryManifest,
    SharedInventorySnapshot, SnapshotApplyReport, SnapshotPublishReport, SnapshotWatermark,
    SNAPSHOT_APPLY_PENDING_KEY,
};
pub(crate) use self::types::{
    CorruptRemoteFile, CorruptRemoteReason, OperationScanReport, SharedSyncPaths,
    SharedSyncRunResult, SyncAppliedMarker, SyncClientIdentity, SyncConflictReason,
    SyncConflictRecord, SyncCoreError, SyncCoreErrorKind, SyncCoreResult, SyncEntryState,
    SyncOperationEnvelope, SyncOperationPayload, SyncOperationType, SyncTombstoneRecord,
    DEFAULT_SHARED_ROOT, SHARED_ROOT_ENV, SHARED_SYNC_INTERVAL_MS, SYNC_SCHEMA_VERSION,
};
use self::types::{
    BOOTSTRAP_COMPLETE_KEY, CHECKSUM_PREFIX, LOCAL_SEQ_WIDTH, MAX_LOCAL_SEQ, OP_FILE_SUFFIX,
    OP_TEMP_MARKER,
};
