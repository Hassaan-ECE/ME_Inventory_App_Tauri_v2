use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
};

use uuid::Uuid;

use crate::model::now_timestamp;

mod canonical;
mod paths;
mod validation;

pub(crate) use self::canonical::{
    canonical_operation_checksum, canonical_operation_json, sha256_hex,
};
pub(super) use self::canonical::{sha256_digest_bytes, sign_operation_for_configured_trust};
pub(super) use self::paths::{
    is_temp_operation_file_name, parse_operation_file_name, validate_path_segment,
};
pub(crate) use self::paths::{operation_file_name, operation_file_path};
use self::validation::{
    validate_operation_for_write, validate_operation_payload_identity, verify_operation_auth,
};
use super::{
    timestamps::validate_operation_timestamps, CorruptRemoteFile, CorruptRemoteReason,
    SharedSyncPaths, SyncCoreError, SyncCoreErrorKind, SyncCoreResult, SyncOperationEnvelope,
    CHECKSUM_PREFIX, SYNC_SCHEMA_VERSION,
};

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

// Used by path-included sync integration tests through sync::test_support.
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

    if let Err(detail) = validate_operation_timestamps(&operation) {
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

    if let Err(detail) = verify_operation_auth(&operation) {
        return Err(corrupt_with_content(
            path,
            CorruptRemoteReason::InvalidEnvelope,
            detail,
            &bytes,
        ));
    }

    Ok(operation)
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

// Used by the test-only read_operation_file helper.
#[allow(dead_code)]
fn file_name_string(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

pub(super) fn corrupt_without_content(
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
