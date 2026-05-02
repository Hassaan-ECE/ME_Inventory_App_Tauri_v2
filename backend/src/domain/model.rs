use std::path::Path;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

pub(crate) const MAX_QUERY_LIMIT: usize = 10_000;
const MAX_QUANTITY: f64 = 1_000_000.0;
const STANDARD_TEXT_LIMIT: usize = 512;
const LONG_TEXT_LIMIT: usize = 4_000;
const NOTES_TEXT_LIMIT: usize = 8_000;
const PATH_TEXT_LIMIT: usize = 2_048;
const IMAGE_PATH_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff"];
const LINK_PROTOCOLS: &[&str] = &["http", "https", "mailto"];
const PICTURE_URL_PROTOCOLS: &[&str] = &["http", "https"];
pub(crate) type CommandResult<T> = Result<T, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
pub(crate) struct InventoryEntryEditContext {
    pub base_version: Option<String>,
    pub changed_fields: Vec<String>,
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
    pub last_snapshot_id: Option<String>,
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

    if let Some(qty) = input.qty {
        if !qty.is_finite() || !(0.0..=MAX_QUANTITY).contains(&qty) {
            return Err(format!(
                "Quantity must be a number between 0 and {MAX_QUANTITY}."
            ));
        }
    }

    validate_text_length("Asset number", &input.asset_number, STANDARD_TEXT_LIMIT)?;
    validate_text_length("Serial number", &input.serial_number, STANDARD_TEXT_LIMIT)?;
    validate_text_length("Manufacturer", &input.manufacturer, STANDARD_TEXT_LIMIT)?;
    validate_text_length("Model", &input.model, STANDARD_TEXT_LIMIT)?;
    validate_text_length("Description", &input.description, LONG_TEXT_LIMIT)?;
    validate_text_length("Project name", &input.project_name, STANDARD_TEXT_LIMIT)?;
    validate_text_length("Location", &input.location, STANDARD_TEXT_LIMIT)?;
    validate_text_length("Assigned to", &input.assigned_to, STANDARD_TEXT_LIMIT)?;
    validate_text_length("Links", &input.links, LONG_TEXT_LIMIT)?;
    validate_links(&input.links)?;
    validate_text_length("Notes", &input.notes, NOTES_TEXT_LIMIT)?;
    validate_text_length("Condition", &input.condition, STANDARD_TEXT_LIMIT)?;
    if let Some(picture_path) = &input.picture_path {
        validate_text_length("Picture path", picture_path, PATH_TEXT_LIMIT)?;
        validate_picture_path(picture_path)?;
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

fn validate_text_length(field_name: &str, value: &str, max_chars: usize) -> CommandResult<()> {
    if value.chars().count() > max_chars {
        return Err(format!(
            "{field_name} must be {max_chars} characters or fewer."
        ));
    }

    Ok(())
}

fn validate_links(value: &str) -> CommandResult<()> {
    if value.trim().is_empty() {
        return Ok(());
    }
    if looks_like_windows_path(value) {
        return Err("Links must be http, https, or mailto URLs.".to_string());
    }

    let url = parse_url_or_implicit_https(value)
        .ok_or_else(|| "Links must be http, https, or mailto URLs.".to_string())?;
    if LINK_PROTOCOLS.contains(&url.scheme()) {
        Ok(())
    } else {
        Err("Links must be http, https, or mailto URLs.".to_string())
    }
}

fn validate_picture_path(value: &str) -> CommandResult<()> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(());
    }
    if value.starts_with("data:image/") {
        return Ok(());
    }
    if looks_like_windows_path(value) || Path::new(value).is_absolute() {
        return validate_picture_file_extension(value);
    }
    if let Ok(url) = Url::parse(value) {
        if PICTURE_URL_PROTOCOLS.contains(&url.scheme()) {
            return Ok(());
        }
        return Err(
            "Picture path must be an absolute image path or an http/https image URL.".to_string(),
        );
    }

    Err("Picture path must be an absolute image path or an http/https image URL.".to_string())
}

fn parse_url_or_implicit_https(value: &str) -> Option<Url> {
    Url::parse(value)
        .or_else(|_| Url::parse(&format!("https://{value}")))
        .ok()
}

fn looks_like_windows_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.starts_with(r"\\")
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/'))
}

fn validate_picture_file_extension(value: &str) -> CommandResult<()> {
    let extension = Path::new(value)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if IMAGE_PATH_EXTENSIONS
        .iter()
        .any(|allowed| extension.eq_ignore_ascii_case(allowed))
    {
        Ok(())
    } else {
        Err("Picture path must use a supported image extension.".to_string())
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

    #[test]
    fn validation_rejects_unreasonable_quantities() {
        let mut input = normalize_entry_input(InventoryEntryInput {
            description: "Caliper".to_string(),
            qty: Some(-1.0),
            ..InventoryEntryInput::default()
        });
        assert!(validate_entry_input(&input)
            .unwrap_err()
            .contains("Quantity"));

        input.qty = Some(f64::NAN);
        assert!(validate_entry_input(&input)
            .unwrap_err()
            .contains("Quantity"));

        input.qty = Some(MAX_QUANTITY + 1.0);
        assert!(validate_entry_input(&input)
            .unwrap_err()
            .contains("Quantity"));
    }

    #[test]
    fn validation_rejects_overlong_text_fields() {
        let input = normalize_entry_input(InventoryEntryInput {
            description: "x".repeat(LONG_TEXT_LIMIT + 1),
            manufacturer: "Mitutoyo".to_string(),
            ..InventoryEntryInput::default()
        });

        assert!(validate_entry_input(&input)
            .unwrap_err()
            .contains("Description"));
    }

    #[test]
    fn validation_rejects_unsafe_link_values() {
        let valid = normalize_entry_input(InventoryEntryInput {
            description: "Caliper".to_string(),
            links: "https://example.com/manual".to_string(),
            ..InventoryEntryInput::default()
        });
        assert!(validate_entry_input(&valid).is_ok());

        let invalid = normalize_entry_input(InventoryEntryInput {
            description: "Caliper".to_string(),
            links: "C:\\Pictures\\part.jpg".to_string(),
            ..InventoryEntryInput::default()
        });
        assert!(validate_entry_input(&invalid)
            .unwrap_err()
            .contains("Links"));
    }

    #[test]
    fn validation_rejects_unsafe_picture_paths() {
        let valid = normalize_entry_input(InventoryEntryInput {
            description: "Caliper".to_string(),
            picture_path: Some("C:\\Pictures\\part.png".to_string()),
            ..InventoryEntryInput::default()
        });
        assert!(validate_entry_input(&valid).is_ok());

        let invalid = normalize_entry_input(InventoryEntryInput {
            description: "Caliper".to_string(),
            picture_path: Some("relative\\part.txt".to_string()),
            ..InventoryEntryInput::default()
        });
        assert!(validate_entry_input(&invalid)
            .unwrap_err()
            .contains("Picture path"));
    }
}
