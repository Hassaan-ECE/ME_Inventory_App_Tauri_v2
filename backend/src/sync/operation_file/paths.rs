use std::path::PathBuf;

use crate::model::db_error;

use super::super::{
    SharedSyncPaths, SyncCoreError, SyncCoreErrorKind, SyncCoreResult, LOCAL_SEQ_WIDTH,
    MAX_LOCAL_SEQ, OP_FILE_SUFFIX, OP_TEMP_MARKER,
};

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
pub(crate) fn operation_file_name(local_seq: u64) -> String {
    format!(
        "{:0width$}{OP_FILE_SUFFIX}",
        local_seq,
        width = LOCAL_SEQ_WIDTH
    )
}
pub(super) fn validate_local_seq(local_seq: u64) -> SyncCoreResult<()> {
    if (1..=MAX_LOCAL_SEQ).contains(&local_seq) {
        Ok(())
    } else {
        Err(SyncCoreError::new(
            SyncCoreErrorKind::InvalidPathSegment,
            format!("local_seq must be between 1 and {MAX_LOCAL_SEQ}."),
        ))
    }
}
pub(crate) fn validate_path_segment(segment: &str) -> SyncCoreResult<()> {
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
pub(crate) fn is_temp_operation_file_name(file_name: &str) -> bool {
    file_name.contains(OP_TEMP_MARKER) || file_name.ends_with(".tmp")
}
pub(crate) fn parse_operation_file_name(file_name: &str) -> Result<u64, String> {
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
