use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) const MAX_QUERY_LIMIT: usize = 100_000;
pub(crate) type CommandResult<T> = Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryEntry {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_id: Option<i64>,
    #[serde(default)]
    pub entry_uuid: String,
    #[serde(default)]
    pub asset_number: String,
    #[serde(default)]
    pub serial_number: String,
    pub qty: Option<f64>,
    #[serde(default)]
    pub manufacturer: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub assigned_to: String,
    #[serde(default)]
    pub links: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "default_lifecycle_status")]
    pub lifecycle_status: String,
    #[serde(default = "default_working_status")]
    pub working_status: String,
    #[serde(default)]
    pub condition: String,
    #[serde(default)]
    pub verified_in_survey: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub manual_entry: bool,
    #[serde(default)]
    pub picture_path: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct InventoryEntryInput {
    pub asset_number: String,
    pub serial_number: String,
    pub qty: Option<f64>,
    pub manufacturer: String,
    pub model: String,
    pub description: String,
    pub project_name: String,
    pub location: String,
    pub assigned_to: String,
    pub links: String,
    pub notes: String,
    pub lifecycle_status: String,
    pub working_status: String,
    pub condition: String,
    pub verified_in_survey: bool,
    pub archived: bool,
    pub picture_path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct FilterState {
    pub asset_number: String,
    pub manufacturer: String,
    pub model: String,
    pub description: String,
    pub location: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct SortState {
    pub column: String,
    pub direction: String,
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            column: "manufacturer".to_string(),
            direction: "asc".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct InventoryQueryInput {
    pub filters: FilterState,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub query: String,
    pub scope: String,
    pub sort: SortState,
}

impl Default for InventoryQueryInput {
    fn default() -> Self {
        Self {
            filters: FilterState::default(),
            limit: Some(MAX_QUERY_LIMIT),
            offset: Some(0),
            query: String::new(),
            scope: "inventory".to_string(),
            sort: SortState::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryCounts {
    pub archive: usize,
    pub inventory: usize,
    pub total: usize,
    pub verified: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventorySharedStatus {
    pub available: bool,
    pub can_modify: bool,
    pub enabled: bool,
    pub has_local_only_changes: Option<bool>,
    pub message: String,
    pub mutation_mode: String,
    pub revision: Option<String>,
    pub shared_db_path: Option<String>,
    pub shared_root_path: Option<String>,
    pub sync_interval_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventorySyncResult {
    pub db_path: String,
    pub entries: Vec<InventoryEntry>,
    pub entries_changed: Option<bool>,
    pub shared: InventorySharedStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryQueryResult {
    pub counts: InventoryCounts,
    pub db_path: String,
    pub entries: Vec<InventoryEntry>,
    pub shared: InventorySharedStatus,
    pub total_filtered: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryEntryMutationResult {
    pub entry: InventoryEntry,
    pub message: String,
    pub mutation_mode: String,
    pub shared: InventorySharedStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryDeleteMutationResult {
    pub entry_id: String,
    pub message: String,
    pub mutation_mode: String,
    pub shared: InventorySharedStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LegacyImportResult {
    pub imported: usize,
    pub source_path: String,
    pub message: String,
}

pub(crate) fn default_lifecycle_status() -> String {
    "active".to_string()
}

pub(crate) fn default_working_status() -> String {
    "unknown".to_string()
}

pub(crate) fn db_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

pub(crate) fn numeric_id(id: &str) -> i64 {
    id.parse::<i64>().unwrap_or(0)
}

pub(crate) fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) fn shared_status(message: impl Into<String>) -> InventorySharedStatus {
    InventorySharedStatus {
        available: true,
        can_modify: true,
        enabled: false,
        has_local_only_changes: Some(false),
        message: message.into(),
        mutation_mode: "local".to_string(),
        revision: None,
        shared_db_path: None,
        shared_root_path: None,
        sync_interval_ms: Some(10_000),
    }
}

pub(crate) fn normalize_entry_input(input: InventoryEntryInput) -> InventoryEntryInput {
    InventoryEntryInput {
        asset_number: input.asset_number.trim().to_string(),
        serial_number: input.serial_number.trim().to_string(),
        qty: input.qty,
        manufacturer: input.manufacturer.trim().to_string(),
        model: input.model.trim().to_string(),
        description: input.description.trim().to_string(),
        project_name: input.project_name.trim().to_string(),
        location: input.location.trim().to_string(),
        assigned_to: input.assigned_to.trim().to_string(),
        links: input.links.trim().to_string(),
        notes: input.notes.trim().to_string(),
        lifecycle_status: normalize_enum(
            input.lifecycle_status,
            &["active", "repair", "scrapped", "missing", "rental"],
            "active",
        ),
        working_status: normalize_enum(
            input.working_status,
            &["unknown", "working", "limited", "not_working"],
            "unknown",
        ),
        condition: input.condition.trim().to_string(),
        verified_in_survey: input.verified_in_survey,
        archived: input.archived,
        picture_path: input.picture_path.map(|path| path.trim().to_string()),
    }
}

pub(crate) fn validate_entry_input(input: &InventoryEntryInput) -> CommandResult<()> {
    let has_identity = !input.asset_number.is_empty()
        || !input.serial_number.is_empty()
        || !input.manufacturer.is_empty()
        || !input.model.is_empty()
        || !input.description.is_empty();

    if !has_identity {
        return Err(
            "Provide at least an asset number, serial number, manufacturer, model, or description before saving."
                .to_string(),
        );
    }

    Ok(())
}

pub(crate) fn create_entry_from_input(id: i64, input: InventoryEntryInput) -> InventoryEntry {
    let timestamp = now_timestamp();
    InventoryEntry {
        id: id.to_string(),
        database_id: Some(id),
        entry_uuid: Uuid::new_v4().simple().to_string(),
        asset_number: input.asset_number,
        serial_number: input.serial_number,
        qty: input.qty,
        manufacturer: input.manufacturer,
        model: input.model,
        description: input.description,
        project_name: input.project_name,
        location: input.location,
        assigned_to: input.assigned_to,
        links: input.links,
        notes: input.notes,
        lifecycle_status: input.lifecycle_status,
        working_status: input.working_status,
        condition: input.condition,
        verified_in_survey: input.verified_in_survey,
        archived: input.archived,
        manual_entry: true,
        picture_path: input.picture_path.unwrap_or_default(),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    }
}

pub(crate) fn update_entry_from_input(
    mut entry: InventoryEntry,
    input: InventoryEntryInput,
) -> InventoryEntry {
    entry.asset_number = input.asset_number;
    entry.serial_number = input.serial_number;
    entry.qty = input.qty;
    entry.manufacturer = input.manufacturer;
    entry.model = input.model;
    entry.description = input.description;
    entry.project_name = input.project_name;
    entry.location = input.location;
    entry.assigned_to = input.assigned_to;
    entry.links = input.links;
    entry.notes = input.notes;
    entry.lifecycle_status = input.lifecycle_status;
    entry.working_status = input.working_status;
    entry.condition = input.condition;
    entry.verified_in_survey = input.verified_in_survey;
    entry.archived = input.archived;
    entry.picture_path = input.picture_path.unwrap_or_default();
    entry.updated_at = now_timestamp();
    entry
}

fn normalize_enum(value: String, allowed: &[&str], fallback: &str) -> String {
    let trimmed = value.trim();
    if allowed.contains(&trimmed) {
        trimmed.to_string()
    } else {
        fallback.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_identity_is_invalid() {
        let input = normalize_entry_input(InventoryEntryInput::default());

        assert!(validate_entry_input(&input).is_err());
    }

    #[test]
    fn normalize_trims_text_and_defaults_enums() {
        let input = normalize_entry_input(InventoryEntryInput {
            manufacturer: "  Mitutoyo ".to_string(),
            lifecycle_status: "bad".to_string(),
            working_status: "also_bad".to_string(),
            picture_path: Some(" C:\\Pictures\\part.jpg ".to_string()),
            ..InventoryEntryInput::default()
        });

        assert_eq!(input.manufacturer, "Mitutoyo");
        assert_eq!(input.lifecycle_status, "active");
        assert_eq!(input.working_status, "unknown");
        assert_eq!(
            input.picture_path.as_deref(),
            Some("C:\\Pictures\\part.jpg")
        );
    }
}
