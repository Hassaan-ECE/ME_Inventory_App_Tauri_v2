use crate::model::{db_error, CommandResult};

pub(super) const KEY_RANGE_SENTINEL: &str = "\u{10ffff}";
pub(super) const ENTRY_PREFIX: &str = "entry:";
pub(super) const ENTRY_RANGE_END: &str = "entry:\u{10ffff}";
pub(super) const ENTRY_ID_PREFIX: &str = "entry_id:";
pub(super) const ENTRY_SCAN_BATCH_LIMIT: usize = 512;
pub(super) const META_NEXT_ID: &[u8] = b"__meta:next_entry_id";
pub(super) const SYNC_META_PREFIX: &str = "meta:";
pub(super) const SYNC_STATE_PREFIX: &str = "sync:";
#[allow(dead_code)]
pub(super) const META_SCHEMA_VERSION: &[u8] = b"meta:schema_version";
pub(super) const META_SYNC_SCHEMA_VERSION: &[u8] = b"meta:sync_schema_version";
pub(super) const META_CLIENT_ID: &[u8] = b"meta:client_id";
pub(super) const META_DEVICE_ID: &[u8] = b"meta:device_id";
pub(super) const META_NEXT_LOCAL_SEQ: &[u8] = b"meta:next_local_seq";
pub(super) const META_SYNC_REVISION: &[u8] = b"meta:sync_revision";
#[allow(dead_code)]
pub(super) const META_LAST_SNAPSHOT_ID: &[u8] = b"meta:last_snapshot_id";
pub(super) const SYNC_OUTBOX_PREFIX: &str = "sync:outbox:";
pub(super) const SYNC_APPLIED_PREFIX: &str = "sync:applied:";
pub(super) const SYNC_CLIENT_SEQ_PREFIX: &str = "sync:seq:";
pub(super) const SYNC_WATERMARK_PREFIX: &str = "sync:watermark:";
pub(super) const SYNC_TOMBSTONE_PREFIX: &str = "sync:tombstone:";
pub(super) const SYNC_ENTRY_STATE_PREFIX: &str = "sync:entry_state:";
pub(super) const SYNC_CONFLICT_PREFIX: &str = "sync:conflict:";
pub(super) const SYNC_CORRUPT_REMOTE_PREFIX: &str = "sync:corrupt_remote:";
pub(super) const SYNC_SCAN_BATCH_LIMIT: usize = 512;
pub(super) const LOCAL_SEQ_KEY_WIDTH: usize = 12;
pub(super) const MAX_LOCAL_SEQ: u64 = 999_999_999_999;

pub(super) fn entry_key(entry_uuid: &str) -> String {
    format!("{ENTRY_PREFIX}{entry_uuid}")
}

pub(super) fn entry_id_key(entry_id: &str) -> String {
    format!("{ENTRY_ID_PREFIX}{entry_id}")
}

pub(super) fn sync_outbox_key(local_seq: u64) -> CommandResult<String> {
    Ok(format!(
        "{SYNC_OUTBOX_PREFIX}{}",
        format_local_seq(local_seq)?
    ))
}

pub(super) fn sync_applied_key(op_id: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_APPLIED_PREFIX, "op_id", op_id)
}

pub(super) fn sync_client_seq_key(client_id: &str, local_seq: u64) -> CommandResult<String> {
    let client_id = normalized_sync_key_segment("client_id", client_id)?;
    Ok(format!(
        "{SYNC_CLIENT_SEQ_PREFIX}{client_id}:{}",
        format_local_seq(local_seq)?
    ))
}

pub(super) fn sync_watermark_key(client_id: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_WATERMARK_PREFIX, "client_id", client_id)
}

pub(super) fn sync_tombstone_key(entry_uuid: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_TOMBSTONE_PREFIX, "entry_uuid", entry_uuid)
}

pub(super) fn sync_entry_state_key(entry_uuid: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_ENTRY_STATE_PREFIX, "entry_uuid", entry_uuid)
}

pub(super) fn sync_conflict_key(conflict_id: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_CONFLICT_PREFIX, "conflict_id", conflict_id)
}

pub(super) fn sync_corrupt_remote_key(record_id: &str) -> CommandResult<String> {
    sync_segment_key(SYNC_CORRUPT_REMOTE_PREFIX, "record_id", record_id)
}

fn sync_segment_key(prefix: &str, label: &str, value: &str) -> CommandResult<String> {
    let segment = normalized_sync_key_segment(label, value)?;
    Ok(format!("{prefix}{segment}"))
}

pub(super) fn normalized_sync_key_segment(label: &str, value: &str) -> CommandResult<String> {
    let trimmed = value.trim();
    validate_sync_key_segment(label, trimmed)?;
    Ok(trimmed.to_string())
}

pub(super) fn validate_sync_key_segment(label: &str, value: &str) -> CommandResult<()> {
    if value.is_empty() {
        return Err(format!("{label} cannot be empty"));
    }

    if value.contains(':') {
        return Err(format!("{label} cannot contain ':'"));
    }

    if value.chars().any(char::is_control) {
        return Err(format!("{label} cannot contain control characters"));
    }

    Ok(())
}

pub(super) fn validate_local_seq(local_seq: u64) -> CommandResult<()> {
    if local_seq == 0 {
        return Err("local_seq must be greater than zero".to_string());
    }

    if local_seq > MAX_LOCAL_SEQ {
        return Err(format!("local_seq must be at most {MAX_LOCAL_SEQ}"));
    }

    Ok(())
}

pub(super) fn format_local_seq(local_seq: u64) -> CommandResult<String> {
    validate_local_seq(local_seq)?;
    Ok(format!("{local_seq:0width$}", width = LOCAL_SEQ_KEY_WIDTH))
}

pub(super) fn parse_local_seq_from_key(prefix: &str, key: &[u8]) -> CommandResult<u64> {
    let key = std::str::from_utf8(key).map_err(db_error)?;
    let Some(local_seq) = key.strip_prefix(prefix) else {
        return Err(format!("invalid {prefix} key"));
    };

    parse_padded_local_seq(local_seq)
}

fn parse_padded_local_seq(value: &str) -> CommandResult<u64> {
    if value.len() != LOCAL_SEQ_KEY_WIDTH || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("local_seq key is not a padded numeric sequence".to_string());
    }

    let local_seq = value
        .parse::<u64>()
        .map_err(|error| format!("invalid local_seq: {error}"))?;
    validate_local_seq(local_seq)?;
    Ok(local_seq)
}

#[allow(dead_code)]
pub(super) fn parse_segment_from_key(
    prefix: &str,
    key: &[u8],
    label: &str,
) -> CommandResult<String> {
    let key = std::str::from_utf8(key).map_err(db_error)?;
    let Some(segment) = key.strip_prefix(prefix) else {
        return Err(format!("invalid {prefix} key"));
    };

    validate_sync_key_segment(label, segment)?;
    Ok(segment.to_string())
}

pub(super) fn range_end_for_prefix(prefix: &str) -> Vec<u8> {
    let mut range_end = prefix.as_bytes().to_vec();
    range_end.extend_from_slice(KEY_RANGE_SENTINEL.as_bytes());
    range_end
}

pub(super) fn is_numeric_entry_id(entry_id: &str) -> bool {
    entry_id.parse::<i64>().is_ok()
}

pub(super) fn validate_sync_key(key: &str) -> CommandResult<()> {
    if key.starts_with(SYNC_META_PREFIX) || key.starts_with(SYNC_STATE_PREFIX) {
        Ok(())
    } else {
        Err(format!(
            "Sync state keys must start with '{SYNC_META_PREFIX}' or '{SYNC_STATE_PREFIX}'."
        ))
    }
}

pub(super) fn next_entry_range_start(mut key: Vec<u8>) -> Vec<u8> {
    key.push(0);
    key
}
