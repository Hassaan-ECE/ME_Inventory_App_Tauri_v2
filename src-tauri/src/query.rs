use std::cmp::Ordering;

use crate::model::{
    numeric_id, FilterState, InventoryCounts, InventoryEntry, InventoryQueryInput, SortState,
    MAX_QUERY_LIMIT,
};

pub(crate) fn query_entries(
    entries: &[InventoryEntry],
    input: InventoryQueryInput,
) -> (Vec<InventoryEntry>, usize) {
    let query = normalize_query_input(input);
    let mut filtered: Vec<InventoryEntry> = entries
        .iter()
        .filter(|entry| entry_matches_query(entry, &query))
        .cloned()
        .collect();

    filtered.sort_by(|left, right| compare_query_entries(left, right, &query.sort));

    let total_filtered = filtered.len();
    let offset = query.offset.unwrap_or(0).min(total_filtered);
    let limit = query.limit.unwrap_or(MAX_QUERY_LIMIT).min(MAX_QUERY_LIMIT);
    let page = filtered.into_iter().skip(offset).take(limit).collect();

    (page, total_filtered)
}

pub(crate) fn get_inventory_counts(entries: &[InventoryEntry]) -> InventoryCounts {
    let archive = entries.iter().filter(|entry| entry.archived).count();
    let verified = entries
        .iter()
        .filter(|entry| entry.verified_in_survey)
        .count();

    InventoryCounts {
        archive,
        inventory: entries.len() - archive,
        total: entries.len(),
        verified,
    }
}

fn normalize_query_input(input: InventoryQueryInput) -> InventoryQueryInput {
    InventoryQueryInput {
        filters: FilterState {
            asset_number: input.filters.asset_number.trim().to_lowercase(),
            manufacturer: input.filters.manufacturer.trim().to_lowercase(),
            model: input.filters.model.trim().to_lowercase(),
            description: input.filters.description.trim().to_lowercase(),
            location: input.filters.location.trim().to_lowercase(),
        },
        limit: Some(input.limit.unwrap_or(MAX_QUERY_LIMIT).min(MAX_QUERY_LIMIT)),
        offset: Some(input.offset.unwrap_or(0)),
        query: input.query.trim().to_lowercase(),
        scope: if input.scope == "archive" {
            "archive".to_string()
        } else {
            "inventory".to_string()
        },
        sort: SortState {
            column: normalize_sort_column(&input.sort.column).to_string(),
            direction: if input.sort.direction == "desc" {
                "desc".to_string()
            } else {
                "asc".to_string()
            },
        },
    }
}

fn normalize_sort_column(column: &str) -> &str {
    match column {
        "assetNumber" | "qty" | "manufacturer" | "model" | "description" | "projectName"
        | "location" | "links" | "verified" => column,
        _ => "manufacturer",
    }
}

fn entry_matches_query(entry: &InventoryEntry, query: &InventoryQueryInput) -> bool {
    if query.scope == "archive" && !entry.archived {
        return false;
    }
    if query.scope != "archive" && entry.archived {
        return false;
    }

    let filters = &query.filters;
    if !contains_text(&entry.asset_number, &filters.asset_number)
        || !contains_text(&entry.manufacturer, &filters.manufacturer)
        || !contains_text(&entry.model, &filters.model)
        || !contains_text(&entry.description, &filters.description)
        || !contains_text(&entry.location, &filters.location)
    {
        return false;
    }

    if query.query.is_empty() {
        return true;
    }

    let haystack = [
        entry.asset_number.as_str(),
        entry.serial_number.as_str(),
        entry.manufacturer.as_str(),
        entry.model.as_str(),
        entry.description.as_str(),
        entry.project_name.as_str(),
        entry.location.as_str(),
        entry.assigned_to.as_str(),
        entry.lifecycle_status.as_str(),
        entry.working_status.as_str(),
        entry.condition.as_str(),
        entry.links.as_str(),
        entry.notes.as_str(),
    ]
    .join(" ")
    .to_lowercase();

    haystack.contains(&query.query)
}

fn contains_text(value: &str, needle: &str) -> bool {
    needle.is_empty() || value.to_lowercase().contains(needle)
}

fn compare_query_entries(
    left: &InventoryEntry,
    right: &InventoryEntry,
    sort: &SortState,
) -> Ordering {
    let order = match sort.column.as_str() {
        "qty" => compare_qty(left.qty, right.qty),
        "verified" => left.verified_in_survey.cmp(&right.verified_in_survey),
        "assetNumber" => compare_text_empty_last(&left.asset_number, &right.asset_number),
        "model" => compare_text_empty_last(&left.model, &right.model),
        "description" => compare_text_empty_last(&left.description, &right.description),
        "projectName" => compare_text_empty_last(&left.project_name, &right.project_name),
        "location" => compare_text_empty_last(&left.location, &right.location),
        "links" => compare_text_empty_last(&left.links, &right.links),
        _ => compare_text_empty_last(&left.manufacturer, &right.manufacturer),
    };

    apply_sort_direction(order, &sort.direction)
        .then_with(|| right.updated_at.cmp(&left.updated_at))
        .then_with(|| numeric_id(&right.id).cmp(&numeric_id(&left.id)))
}

fn compare_qty(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(left), Some(right)) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
    }
}

fn compare_text_empty_last(left: &str, right: &str) -> Ordering {
    let left = left.trim().to_lowercase();
    let right = right.trim().to_lowercase();
    match (left.is_empty(), right.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => left.cmp(&right),
    }
}

fn apply_sort_direction(order: Ordering, direction: &str) -> Ordering {
    if direction == "desc" && order != Ordering::Equal {
        order.reverse()
    } else {
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_filters_scope_and_sort() {
        let entries = vec![
            test_entry("1", "uuid-1", "Mitutoyo", "Caliper", false, 2.0, true),
            test_entry("2", "uuid-2", "Fluke", "Meter", false, 1.0, false),
            test_entry("3", "uuid-3", "Mitutoyo", "Archived", true, 3.0, false),
        ];

        let input = InventoryQueryInput {
            filters: FilterState {
                manufacturer: "mitu".to_string(),
                ..FilterState::default()
            },
            query: "caliper".to_string(),
            sort: SortState {
                column: "qty".to_string(),
                direction: "desc".to_string(),
            },
            ..InventoryQueryInput::default()
        };

        let (result, total_filtered) = query_entries(&entries, input);

        assert_eq!(total_filtered, 1);
        assert_eq!(result[0].id, "1");
    }

    #[test]
    fn query_counts_include_archive_and_verified() {
        let entries = vec![
            test_entry("1", "uuid-1", "A", "One", false, 1.0, true),
            test_entry("2", "uuid-2", "B", "Two", true, 1.0, false),
        ];

        let counts = get_inventory_counts(&entries);

        assert_eq!(counts.total, 2);
        assert_eq!(counts.inventory, 1);
        assert_eq!(counts.archive, 1);
        assert_eq!(counts.verified, 1);
    }

    #[test]
    fn query_applies_offset_and_limit() {
        let entries = vec![
            test_entry("1", "uuid-1", "A", "One", false, 1.0, false),
            test_entry("2", "uuid-2", "B", "Two", false, 1.0, false),
            test_entry("3", "uuid-3", "C", "Three", false, 1.0, false),
        ];

        let input = InventoryQueryInput {
            limit: Some(1),
            offset: Some(1),
            sort: SortState {
                column: "manufacturer".to_string(),
                direction: "asc".to_string(),
            },
            ..InventoryQueryInput::default()
        };

        let (result, total_filtered) = query_entries(&entries, input);

        assert_eq!(total_filtered, 3);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "2");
    }

    pub(crate) fn test_entry(
        id: &str,
        entry_uuid: &str,
        manufacturer: &str,
        description: &str,
        archived: bool,
        qty: f64,
        verified: bool,
    ) -> InventoryEntry {
        InventoryEntry {
            id: id.to_string(),
            database_id: id.parse::<i64>().ok(),
            entry_uuid: entry_uuid.to_string(),
            asset_number: format!("ME-{id}"),
            serial_number: String::new(),
            qty: Some(qty),
            manufacturer: manufacturer.to_string(),
            model: String::new(),
            description: description.to_string(),
            project_name: String::new(),
            location: "Lab".to_string(),
            assigned_to: String::new(),
            links: String::new(),
            notes: String::new(),
            lifecycle_status: "active".to_string(),
            working_status: "unknown".to_string(),
            condition: String::new(),
            verified_in_survey: verified,
            archived,
            manual_entry: false,
            picture_path: String::new(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }
}
