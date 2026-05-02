use crate::model::{now_timestamp, InventoryEntry, InventoryEntryInput};

pub(crate) fn entry_base_version(entry: &InventoryEntry) -> Option<String> {
    (!entry.updated_at.is_empty()).then(|| entry.updated_at.clone())
}

pub(crate) fn normalize_changed_entry_fields(fields: Vec<String>) -> Vec<String> {
    let mut normalized = fields
        .into_iter()
        .map(|field| normalize_changed_entry_field(&field))
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

pub(crate) fn update_entry_from_input_fields(
    mut entry: InventoryEntry,
    input: &InventoryEntryInput,
    changed_fields: &[String],
) -> InventoryEntry {
    for field in changed_fields {
        match field.as_str() {
            "asset_number" => entry.asset_number.clone_from(&input.asset_number),
            "serial_number" => entry.serial_number.clone_from(&input.serial_number),
            "qty" => entry.qty = input.qty,
            "manufacturer" => entry.manufacturer.clone_from(&input.manufacturer),
            "model" => entry.model.clone_from(&input.model),
            "description" => entry.description.clone_from(&input.description),
            "project_name" => entry.project_name.clone_from(&input.project_name),
            "location" => entry.location.clone_from(&input.location),
            "assigned_to" => entry.assigned_to.clone_from(&input.assigned_to),
            "links" => entry.links.clone_from(&input.links),
            "notes" => entry.notes.clone_from(&input.notes),
            "lifecycle_status" => entry.lifecycle_status.clone_from(&input.lifecycle_status),
            "working_status" => entry.working_status.clone_from(&input.working_status),
            "condition" => entry.condition.clone_from(&input.condition),
            "verified_in_survey" => entry.verified_in_survey = input.verified_in_survey,
            "archived" => entry.archived = input.archived,
            "picture_path" => entry.picture_path = input.picture_path.clone().unwrap_or_default(),
            _ => {}
        }
    }

    entry.updated_at = now_timestamp();
    entry
}

pub(crate) fn changed_entry_fields(before: &InventoryEntry, after: &InventoryEntry) -> Vec<String> {
    let mut fields = Vec::new();

    if before.asset_number != after.asset_number {
        fields.push("asset_number".to_string());
    }
    if before.serial_number != after.serial_number {
        fields.push("serial_number".to_string());
    }
    if before.qty != after.qty {
        fields.push("qty".to_string());
    }
    if before.manufacturer != after.manufacturer {
        fields.push("manufacturer".to_string());
    }
    if before.model != after.model {
        fields.push("model".to_string());
    }
    if before.description != after.description {
        fields.push("description".to_string());
    }
    if before.project_name != after.project_name {
        fields.push("project_name".to_string());
    }
    if before.location != after.location {
        fields.push("location".to_string());
    }
    if before.assigned_to != after.assigned_to {
        fields.push("assigned_to".to_string());
    }
    if before.links != after.links {
        fields.push("links".to_string());
    }
    if before.notes != after.notes {
        fields.push("notes".to_string());
    }
    if before.lifecycle_status != after.lifecycle_status {
        fields.push("lifecycle_status".to_string());
    }
    if before.working_status != after.working_status {
        fields.push("working_status".to_string());
    }
    if before.condition != after.condition {
        fields.push("condition".to_string());
    }
    if before.verified_in_survey != after.verified_in_survey {
        fields.push("verified_in_survey".to_string());
    }
    if before.archived != after.archived {
        fields.push("archived".to_string());
    }
    if before.picture_path != after.picture_path {
        fields.push("picture_path".to_string());
    }

    fields
}

fn normalize_changed_entry_field(field: &str) -> String {
    match field.trim() {
        "assetNumber" => "asset_number".to_string(),
        "serialNumber" => "serial_number".to_string(),
        "projectName" => "project_name".to_string(),
        "assignedTo" => "assigned_to".to_string(),
        "lifecycleStatus" => "lifecycle_status".to_string(),
        "workingStatus" => "working_status".to_string(),
        "verifiedInSurvey" => "verified_in_survey".to_string(),
        "picturePath" => "picture_path".to_string(),
        "databaseId" | "database_id" | "entryUuid" | "entry_uuid" | "createdAt" | "created_at"
        | "updatedAt" | "updated_at" | "id" => String::new(),
        other => other.to_string(),
    }
}
