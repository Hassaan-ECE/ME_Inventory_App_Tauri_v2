use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
};

use serde::Serialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::model::{db_error, now_timestamp};

use super::{auth, timestamps::validate_operation_timestamps};
use super::{
    CorruptRemoteFile, CorruptRemoteReason, SharedSyncPaths, SyncCoreError, SyncCoreErrorKind,
    SyncCoreResult, SyncOperationEnvelope, SyncOperationType, CHECKSUM_PREFIX, LOCAL_SEQ_WIDTH,
    MAX_LOCAL_SEQ, OP_FILE_SUFFIX, OP_TEMP_MARKER, SYNC_SCHEMA_VERSION,
};

pub(crate) fn canonical_operation_checksum(
    operation: &SyncOperationEnvelope,
) -> SyncCoreResult<String> {
    let bytes = canonical_json_bytes_without_checksum_or_auth(operation)?;
    Ok(format!("{CHECKSUM_PREFIX}{}", sha256_hex(&bytes)))
}

pub(crate) fn canonical_operation_json(
    operation: &SyncOperationEnvelope,
) -> SyncCoreResult<Vec<u8>> {
    canonical_json_bytes(operation)
}

pub(super) fn sign_operation_for_configured_trust(
    operation: &mut SyncOperationEnvelope,
) -> SyncCoreResult<()> {
    operation.auth = None;
    let bytes = canonical_json_bytes_without_checksum_or_auth(operation)?;
    operation.auth = auth::sign_canonical_bytes("sync.operation.v1", &bytes)?;
    Ok(())
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

pub(crate) fn operation_file_name(local_seq: u64) -> String {
    format!(
        "{:0width$}{OP_FILE_SUFFIX}",
        local_seq,
        width = LOCAL_SEQ_WIDTH
    )
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha256_digest_bytes(bytes);
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
    validate_operation_timestamps(operation).map_err(|detail| {
        SyncCoreError::new(
            SyncCoreErrorKind::InvalidEnvelope,
            format!("Operation envelope has invalid timestamps: {detail}"),
        )
    })?;
    verify_operation_auth(operation).map_err(|detail| {
        SyncCoreError::new(
            SyncCoreErrorKind::InvalidEnvelope,
            format!("Operation envelope has invalid authentication: {detail}"),
        )
    })?;
    Ok(())
}

fn verify_operation_auth(operation: &SyncOperationEnvelope) -> Result<(), String> {
    let bytes = canonical_json_bytes_without_checksum_or_auth(operation)
        .map_err(|error| error.to_string())?;
    auth::verify_canonical_bytes("sync.operation.v1", &bytes, operation.auth.as_deref())
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

pub(super) fn validate_path_segment(segment: &str) -> SyncCoreResult<()> {
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

fn canonical_json_bytes_without_checksum_or_auth<T: Serialize>(
    value: &T,
) -> SyncCoreResult<Vec<u8>> {
    let mut value = serde_json::to_value(value)?;
    if let Value::Object(object) = &mut value {
        object.remove("checksum");
        object.remove("auth");
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

pub(super) fn is_temp_operation_file_name(file_name: &str) -> bool {
    file_name.contains(OP_TEMP_MARKER) || file_name.ends_with(".tmp")
}

pub(super) fn parse_operation_file_name(file_name: &str) -> Result<u64, String> {
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

fn nibble_to_hex(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + (nibble - 10)) as char,
        _ => unreachable!("nibble is masked to four bits"),
    }
}

pub(super) fn sha256_digest_bytes(bytes: &[u8]) -> [u8; 32] {
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
