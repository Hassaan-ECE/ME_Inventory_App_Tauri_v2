#![allow(dead_code, unused_imports)]

#[path = "sync/apply.rs"]
mod apply;
#[path = "sync/conflicts.rs"]
mod conflicts;
#[path = "sync/identity.rs"]
mod identity;
#[path = "sync/operation_file.rs"]
mod operation_file;
#[path = "sync/queue.rs"]
mod queue;
#[path = "sync/scanning.rs"]
mod scanning;
#[path = "sync/shared_paths.rs"]
mod shared_paths;
#[path = "sync/types.rs"]
mod types;

pub(crate) use self::apply::{run_shared_sync, run_shared_sync_with_root};
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
pub(crate) use self::scanning::scan_operation_files;
pub(crate) use self::shared_paths::{
    ensure_operation_log_layout, queued_local_status, resolve_shared_root,
    resolve_shared_root_from_env_value, resolved_shared_sync_paths, shared_inventory_status,
};
pub(crate) use self::types::{
    CorruptRemoteFile, CorruptRemoteReason, OperationScanReport, SharedSyncPaths,
    SharedSyncRunResult, SyncAppliedMarker, SyncClientIdentity, SyncConflictReason,
    SyncConflictRecord, SyncCoreError, SyncCoreErrorKind, SyncCoreResult, SyncEntryState,
    SyncOperationEnvelope, SyncOperationPayload, SyncOperationType, SyncTombstoneRecord,
    DEFAULT_SHARED_ROOT, SHARED_ROOT_ENV, SYNC_SCHEMA_VERSION,
};
use self::types::{
    BOOTSTRAP_COMPLETE_KEY, CHECKSUM_PREFIX, LOCAL_SEQ_WIDTH, MAX_LOCAL_SEQ, OP_FILE_SUFFIX,
    OP_TEMP_MARKER, SHARED_SYNC_INTERVAL_MS,
};
