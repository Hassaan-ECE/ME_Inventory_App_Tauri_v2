use std::cmp::Ordering;

use chrono::{DateTime, Utc};

use super::types::{SyncOperationEnvelope, SyncOperationType};

pub(super) fn validate_operation_timestamps(
    operation: &SyncOperationEnvelope,
) -> Result<(), String> {
    parse_sync_timestamp(&operation.created_at_utc, "created_at_utc")?;
    parse_sync_timestamp(&operation.mutation_ts_utc, "mutation_ts_utc")?;

    if operation.operation_type == SyncOperationType::InventoryEntryDelete {
        let deleted_at_utc = operation
            .payload
            .deleted_at_utc
            .as_deref()
            .unwrap_or_default();
        parse_sync_timestamp(deleted_at_utc, "payload.deleted_at_utc")?;
    }

    Ok(())
}

pub(super) fn compare_timestamp_text(left: &str, right: &str) -> Ordering {
    match (
        parse_sync_timestamp(left, "left"),
        parse_sync_timestamp(right, "right"),
    ) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}

pub(super) fn max_timestamp_text(left: &str, right: &str) -> String {
    if compare_timestamp_text(right, left).is_gt() {
        right.to_string()
    } else {
        left.to_string()
    }
}

fn parse_sync_timestamp(value: &str, field_name: &str) -> Result<DateTime<Utc>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "{field_name} must be a non-empty RFC3339 UTC timestamp."
        ));
    }
    if trimmed != value {
        return Err(format!(
            "{field_name} must not include surrounding whitespace."
        ));
    }

    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| format!("{field_name} must be a valid RFC3339 UTC timestamp."))?;
    if parsed.offset().local_minus_utc() != 0 {
        return Err(format!("{field_name} must use UTC offset Z or +00:00."));
    }

    Ok(parsed.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_comparison_uses_parsed_time_not_raw_text() {
        assert!(compare_timestamp_text("2026-04-26T13:00:00.001Z", "2026-04-26T13:00:00Z").is_gt());
    }

    #[test]
    fn timestamp_validation_requires_utc_rfc3339() {
        assert!(parse_sync_timestamp("2026-04-26T13:00:00Z", "test").is_ok());
        assert!(parse_sync_timestamp("2026-04-26T13:00:00.000+00:00", "test").is_ok());
        assert!(parse_sync_timestamp("2026-04-26T08:00:00.000-05:00", "test").is_err());
        assert!(parse_sync_timestamp("not-a-date", "test").is_err());
    }
}
