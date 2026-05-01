use crate::model::InventoryEntry;

pub(super) const INVENTORY_COLUMNS: [InventoryColumn; 19] = [
    InventoryColumn::new(
        "Asset Number",
        16.0,
        InventoryField::AssetNumber,
        CellKind::Text,
    ),
    InventoryColumn::new(
        "Serial Number",
        20.0,
        InventoryField::SerialNumber,
        CellKind::Text,
    ),
    InventoryColumn::new("Qty", 9.0, InventoryField::Qty, CellKind::Number),
    InventoryColumn::new(
        "Manufacturer",
        18.0,
        InventoryField::Manufacturer,
        CellKind::Text,
    ),
    InventoryColumn::new("Model", 16.0, InventoryField::Model, CellKind::Text),
    InventoryColumn::new(
        "Description",
        32.0,
        InventoryField::Description,
        CellKind::WrappedText,
    ),
    InventoryColumn::new("Project", 20.0, InventoryField::Project, CellKind::Text),
    InventoryColumn::new("Location", 24.0, InventoryField::Location, CellKind::Text),
    InventoryColumn::new(
        "Assigned To",
        14.0,
        InventoryField::AssignedTo,
        CellKind::Text,
    ),
    InventoryColumn::new(
        "Lifecycle",
        13.0,
        InventoryField::Lifecycle,
        CellKind::Lifecycle,
    ),
    InventoryColumn::new("Working", 13.0, InventoryField::Working, CellKind::Working),
    InventoryColumn::new(
        "Condition",
        22.0,
        InventoryField::Condition,
        CellKind::WrappedText,
    ),
    InventoryColumn::new(
        "Verified",
        14.0,
        InventoryField::Verified,
        CellKind::Centered,
    ),
    InventoryColumn::new(
        "Archived",
        12.0,
        InventoryField::Archived,
        CellKind::Centered,
    ),
    InventoryColumn::new(
        "Picture Path",
        34.0,
        InventoryField::PicturePath,
        CellKind::Text,
    ),
    InventoryColumn::new("Links", 28.0, InventoryField::Links, CellKind::WrappedText),
    InventoryColumn::new("Notes", 40.0, InventoryField::Notes, CellKind::WrappedText),
    InventoryColumn::new(
        "Created At",
        22.0,
        InventoryField::CreatedAt,
        CellKind::Text,
    ),
    InventoryColumn::new(
        "Updated At",
        22.0,
        InventoryField::UpdatedAt,
        CellKind::Text,
    ),
];

#[derive(Clone, Copy)]
pub(super) struct InventoryColumn {
    pub(super) header: &'static str,
    pub(super) width: f64,
    pub(super) field: InventoryField,
    pub(super) kind: CellKind,
}

impl InventoryColumn {
    const fn new(header: &'static str, width: f64, field: InventoryField, kind: CellKind) -> Self {
        Self {
            header,
            width,
            field,
            kind,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum InventoryField {
    AssetNumber,
    SerialNumber,
    Qty,
    Manufacturer,
    Model,
    Description,
    Project,
    Location,
    AssignedTo,
    Lifecycle,
    Working,
    Condition,
    Verified,
    Archived,
    PicturePath,
    Links,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(Clone, Copy)]
pub(super) enum CellKind {
    Centered,
    Lifecycle,
    Number,
    Text,
    WrappedText,
    Working,
}

pub(super) fn inventory_text(entry: &InventoryEntry, field: InventoryField) -> &str {
    match field {
        InventoryField::AssetNumber => &entry.asset_number,
        InventoryField::SerialNumber => &entry.serial_number,
        InventoryField::Manufacturer => &entry.manufacturer,
        InventoryField::Model => &entry.model,
        InventoryField::Description => &entry.description,
        InventoryField::Project => &entry.project_name,
        InventoryField::Location => &entry.location,
        InventoryField::AssignedTo => &entry.assigned_to,
        InventoryField::Lifecycle => &entry.lifecycle_status,
        InventoryField::Working => &entry.working_status,
        InventoryField::Condition => &entry.condition,
        InventoryField::PicturePath => &entry.picture_path,
        InventoryField::Links => &entry.links,
        InventoryField::Notes => &entry.notes,
        InventoryField::CreatedAt => &entry.created_at,
        InventoryField::UpdatedAt => &entry.updated_at,
        InventoryField::Qty | InventoryField::Verified | InventoryField::Archived => "",
    }
}

pub(super) fn yes_if(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        ""
    }
}
