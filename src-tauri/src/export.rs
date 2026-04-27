use std::path::{Path, PathBuf};

use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook, Worksheet, XlsxError};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::{
    legacy_import,
    model::{CommandResult, InventoryEntry},
    store::InventoryDb,
};

pub(crate) const DEFAULT_EXCEL_EXPORT_FILENAME: &str = "ME_Inventory_Export.xlsx";

const INVENTORY_SHEET: &str = "Inventory";
const ARCHIVE_SHEET: &str = "Archive";

const INVENTORY_COLUMNS: [InventoryColumn; 19] = [
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExcelExportResult {
    pub canceled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExcelExportStats {
    pub archived_count: usize,
    pub inventory_count: usize,
    pub output_path: String,
    pub total_count: usize,
}

#[tauri::command]
pub(crate) async fn export_excel(
    app: AppHandle,
    db: State<'_, InventoryDb>,
) -> CommandResult<ExcelExportResult> {
    let Some(output_path) = pick_export_path(&app) else {
        return Ok(ExcelExportResult::canceled());
    };

    match export_excel_to_path(&db, &output_path) {
        Ok(stats) => Ok(ExcelExportResult::success(stats.output_path)),
        Err(error) => Ok(ExcelExportResult::failed(error)),
    }
}

pub(crate) fn export_excel_to_path(
    db: &InventoryDb,
    output_path: impl AsRef<Path>,
) -> CommandResult<ExcelExportStats> {
    legacy_import::ensure_legacy_imported(db)?;
    let entries = db.load_entries()?;
    write_inventory_workbook(&entries, output_path)
}

pub(crate) fn write_inventory_workbook(
    entries: &[InventoryEntry],
    output_path: impl AsRef<Path>,
) -> CommandResult<ExcelExportStats> {
    let output_path = output_path.as_ref();
    let summary = ExportSummary::from_entries(entries);
    let formats = WorkbookFormats::new();
    let mut workbook = Workbook::new();

    {
        let worksheet = workbook.add_worksheet();
        build_inventory_sheet(
            worksheet,
            INVENTORY_SHEET,
            entries.iter().filter(|entry| !entry.archived),
            summary.inventory_entries,
            &formats,
        )
        .map_err(export_error)?;
    }

    {
        let worksheet = workbook.add_worksheet();
        build_inventory_sheet(
            worksheet,
            ARCHIVE_SHEET,
            entries.iter().filter(|entry| entry.archived),
            summary.archived_entries,
            &formats,
        )
        .map_err(export_error)?;
    }

    workbook.save(output_path).map_err(export_error)?;

    Ok(ExcelExportStats {
        archived_count: summary.archived_entries,
        inventory_count: summary.inventory_entries,
        output_path: output_path.to_string_lossy().to_string(),
        total_count: summary.total_entries,
    })
}

impl ExcelExportResult {
    fn canceled() -> Self {
        Self {
            canceled: true,
            output_path: None,
            error: None,
        }
    }

    fn success(output_path: String) -> Self {
        Self {
            canceled: false,
            output_path: Some(output_path),
            error: None,
        }
    }

    fn failed(error: String) -> Self {
        Self {
            canceled: false,
            output_path: None,
            error: Some(error),
        }
    }
}

#[derive(Clone, Copy)]
struct InventoryColumn {
    header: &'static str,
    width: f64,
    field: InventoryField,
    kind: CellKind,
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
enum InventoryField {
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
enum CellKind {
    Centered,
    Lifecycle,
    Number,
    Text,
    WrappedText,
    Working,
}

#[derive(Debug, Clone)]
struct ExportSummary {
    archived_entries: usize,
    inventory_entries: usize,
    total_entries: usize,
}

impl ExportSummary {
    fn from_entries(entries: &[InventoryEntry]) -> Self {
        let archived_entries = entries.iter().filter(|entry| entry.archived).count();

        Self {
            archived_entries,
            inventory_entries: entries.len() - archived_entries,
            total_entries: entries.len(),
        }
    }
}

struct WorkbookFormats {
    centered_even: Format,
    centered_odd: Format,
    header: Format,
    lifecycle_active: Format,
    lifecycle_missing: Format,
    lifecycle_rental: Format,
    lifecycle_repair: Format,
    lifecycle_scrapped: Format,
    number_even: Format,
    number_odd: Format,
    text_even: Format,
    text_odd: Format,
    working_limited: Format,
    working_not_working: Format,
    working_working: Format,
    wrapped_even: Format,
    wrapped_odd: Format,
}

impl WorkbookFormats {
    fn new() -> Self {
        Self {
            centered_even: cell_format("F9FAFB", FormatAlign::Center, false),
            centered_odd: cell_format("F3F4F6", FormatAlign::Center, false),
            header: Format::new()
                .set_bold()
                .set_font_color("FFFFFF")
                .set_font_size(11)
                .set_background_color("1F2937")
                .set_align(FormatAlign::Center)
                .set_align(FormatAlign::VerticalCenter)
                .set_text_wrap()
                .set_border(FormatBorder::Thin)
                .set_border_color("374151")
                .set_border_bottom(FormatBorder::Medium),
            lifecycle_active: status_format("DCFCE7", "166534"),
            lifecycle_missing: status_format("FCE7F3", "9D174D"),
            lifecycle_rental: status_format("DBEAFE", "1E40AF"),
            lifecycle_repair: status_format("FEF3C7", "92400E"),
            lifecycle_scrapped: status_format("FEE2E2", "991B1B"),
            number_even: cell_format("F9FAFB", FormatAlign::Right, false).set_num_format("0.##"),
            number_odd: cell_format("F3F4F6", FormatAlign::Right, false).set_num_format("0.##"),
            text_even: cell_format("F9FAFB", FormatAlign::Left, false),
            text_odd: cell_format("F3F4F6", FormatAlign::Left, false),
            working_limited: status_format("FEF3C7", "92400E"),
            working_not_working: status_format("FEE2E2", "991B1B"),
            working_working: status_format("DCFCE7", "166534"),
            wrapped_even: cell_format("F9FAFB", FormatAlign::Left, true),
            wrapped_odd: cell_format("F3F4F6", FormatAlign::Left, true),
        }
    }

    fn format_for<'a>(
        &'a self,
        column: &InventoryColumn,
        entry: &InventoryEntry,
        row_index: usize,
    ) -> &'a Format {
        match column.kind {
            CellKind::Centered => {
                if row_index % 2 == 0 {
                    &self.centered_even
                } else {
                    &self.centered_odd
                }
            }
            CellKind::Lifecycle => self.lifecycle_format(&entry.lifecycle_status, row_index),
            CellKind::Number => {
                if row_index % 2 == 0 {
                    &self.number_even
                } else {
                    &self.number_odd
                }
            }
            CellKind::Text => {
                if row_index % 2 == 0 {
                    &self.text_even
                } else {
                    &self.text_odd
                }
            }
            CellKind::WrappedText => {
                if row_index % 2 == 0 {
                    &self.wrapped_even
                } else {
                    &self.wrapped_odd
                }
            }
            CellKind::Working => self.working_format(&entry.working_status, row_index),
        }
    }

    fn lifecycle_format(&self, value: &str, row_index: usize) -> &Format {
        match value {
            "active" => &self.lifecycle_active,
            "missing" => &self.lifecycle_missing,
            "rental" => &self.lifecycle_rental,
            "repair" => &self.lifecycle_repair,
            "scrapped" => &self.lifecycle_scrapped,
            _ => {
                if row_index % 2 == 0 {
                    &self.centered_even
                } else {
                    &self.centered_odd
                }
            }
        }
    }

    fn working_format(&self, value: &str, row_index: usize) -> &Format {
        match value {
            "limited" => &self.working_limited,
            "not_working" => &self.working_not_working,
            "working" => &self.working_working,
            _ => {
                if row_index % 2 == 0 {
                    &self.centered_even
                } else {
                    &self.centered_odd
                }
            }
        }
    }
}

fn pick_export_path(app: &AppHandle) -> Option<PathBuf> {
    app.dialog()
        .file()
        .set_title("Export All Entries to Excel")
        .set_file_name(DEFAULT_EXCEL_EXPORT_FILENAME)
        .add_filter("Excel Workbook", &["xlsx"])
        .blocking_save_file()
        .and_then(|file_path| file_path.simplified().into_path().ok())
}

fn build_inventory_sheet<'a>(
    worksheet: &mut Worksheet,
    sheet_name: &str,
    entries: impl Iterator<Item = &'a InventoryEntry>,
    entry_count: usize,
    formats: &WorkbookFormats,
) -> Result<(), XlsxError> {
    worksheet.set_name(sheet_name)?;
    worksheet.set_landscape();
    worksheet.set_print_fit_to_pages(1, 0);
    worksheet.set_freeze_panes(1, 0)?;
    worksheet.set_row_height(0, 28.0)?;

    for (col, column) in INVENTORY_COLUMNS.iter().enumerate() {
        let col = col as u16;
        worksheet.set_column_width(col, column.width)?;
        worksheet.write_string_with_format(0, col, column.header, &formats.header)?;
    }

    for (row_index, entry) in entries.enumerate() {
        let row = (row_index + 1) as u32;
        worksheet.set_row_height(row, 20.0)?;

        for (col, column) in INVENTORY_COLUMNS.iter().enumerate() {
            write_inventory_cell(
                worksheet, row, col as u16, row_index, entry, column, formats,
            )?;
        }
    }

    worksheet.autofilter(
        0,
        0,
        entry_count as u32,
        (INVENTORY_COLUMNS.len() - 1) as u16,
    )?;

    Ok(())
}

fn write_inventory_cell(
    worksheet: &mut Worksheet,
    row: u32,
    col: u16,
    row_index: usize,
    entry: &InventoryEntry,
    column: &InventoryColumn,
    formats: &WorkbookFormats,
) -> Result<(), XlsxError> {
    let format = formats.format_for(column, entry, row_index);

    match column.field {
        InventoryField::Qty => {
            if let Some(qty) = entry.qty {
                worksheet.write_number_with_format(row, col, qty, format)?;
            } else {
                worksheet.write_string_with_format(row, col, "", format)?;
            }
        }
        InventoryField::Verified => {
            worksheet.write_string_with_format(
                row,
                col,
                yes_if(entry.verified_in_survey),
                format,
            )?;
        }
        InventoryField::Archived => {
            worksheet.write_string_with_format(row, col, yes_if(entry.archived), format)?;
        }
        _ => {
            worksheet.write_string_with_format(
                row,
                col,
                inventory_text(entry, column.field),
                format,
            )?;
        }
    }

    Ok(())
}

fn inventory_text(entry: &InventoryEntry, field: InventoryField) -> &str {
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

fn yes_if(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        ""
    }
}

fn cell_format(background: &'static str, align: FormatAlign, wrap: bool) -> Format {
    let mut format = Format::new()
        .set_font_color("1F2937")
        .set_font_size(10)
        .set_background_color(background)
        .set_align(align)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_border_color("D1D5DB");

    if wrap {
        format = format.set_text_wrap();
    }

    format
}

fn status_format(background: &'static str, font_color: &'static str) -> Format {
    Format::new()
        .set_font_color(font_color)
        .set_font_size(10)
        .set_background_color(background)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_border_color("D1D5DB")
}

fn export_error(error: impl std::fmt::Display) -> String {
    format!("Excel export failed: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env, fs,
        fs::File,
        io::Read,
        path::{Path, PathBuf},
    };
    use uuid::Uuid;
    use zip::ZipArchive;

    #[test]
    fn write_workbook_creates_inventory_and_archive_sheets() {
        let path = temp_xlsx_path("required-sheets");
        let entries = vec![
            test_entry("1", false, true, Some(2.0)),
            test_entry("2", true, false, None),
        ];

        let stats = write_inventory_workbook(&entries, &path).unwrap();

        assert_eq!(stats.total_count, 2);
        assert_eq!(stats.inventory_count, 1);
        assert_eq!(stats.archived_count, 1);
        assert!(path.exists());

        let workbook_xml = read_xlsx_member(&path, "xl/workbook.xml");
        assert!(workbook_xml.contains(r#"name="Inventory""#));
        assert!(workbook_xml.contains(r#"name="Archive""#));
        assert!(!workbook_xml.contains(r#"name="Import Issues""#));
        assert!(!workbook_xml.contains(r#"name="Export Summary""#));

        let shared_strings = shared_strings(&path);
        let inventory_rows = worksheet_rows(&path, "xl/worksheets/sheet1.xml", &shared_strings);
        assert_eq!(inventory_rows[0], inventory_headers());
        assert_eq!(inventory_rows[1][0], "ME-1");
        assert_eq!(inventory_rows[1][2], "2");
        assert_eq!(inventory_rows[1][12], "Yes");
        assert_eq!(inventory_rows.len(), 2);

        let archive_rows = worksheet_rows(&path, "xl/worksheets/sheet2.xml", &shared_strings);
        assert_eq!(archive_rows[0], inventory_headers());
        assert_eq!(archive_rows[1][0], "ME-2");
        assert_eq!(archive_rows[1][13], "Yes");
        assert_eq!(archive_rows.len(), 2);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn inventory_sheet_preserves_current_entry_field_contract() {
        let path = temp_xlsx_path("field-contract");

        write_inventory_workbook(&[test_entry("42", false, true, Some(3.5))], &path).unwrap();

        let shared_strings = shared_strings(&path);
        let inventory_rows = worksheet_rows(&path, "xl/worksheets/sheet1.xml", &shared_strings);
        assert_eq!(inventory_rows[0], inventory_headers());
        assert_eq!(
            inventory_rows[1],
            vec![
                "ME-42",
                "SN-42",
                "3.5",
                "Mitutoyo",
                "Model 42",
                "Entry 42",
                "Project",
                "Lab",
                "ME",
                "active",
                "working",
                "Good",
                "Yes",
                "",
                r"C:\Pictures\42.jpg",
                "https://example.com",
                "Notes",
                "2026-01-01T00:00:00.000Z",
                "2026-01-02T00:00:00.000Z",
            ]
        );

        let _ = fs::remove_file(path);
    }

    fn test_entry(id: &str, archived: bool, verified: bool, qty: Option<f64>) -> InventoryEntry {
        InventoryEntry {
            id: id.to_string(),
            database_id: id.parse::<i64>().ok(),
            entry_uuid: format!("uuid-{id}"),
            asset_number: format!("ME-{id}"),
            serial_number: format!("SN-{id}"),
            qty,
            manufacturer: "Mitutoyo".to_string(),
            model: format!("Model {id}"),
            description: format!("Entry {id}"),
            project_name: "Project".to_string(),
            location: "Lab".to_string(),
            assigned_to: "ME".to_string(),
            links: "https://example.com".to_string(),
            notes: "Notes".to_string(),
            lifecycle_status: if archived { "scrapped" } else { "active" }.to_string(),
            working_status: "working".to_string(),
            condition: "Good".to_string(),
            verified_in_survey: verified,
            archived,
            manual_entry: false,
            picture_path: format!(r"C:\Pictures\{id}.jpg"),
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-02T00:00:00.000Z".to_string(),
        }
    }

    fn inventory_headers() -> Vec<&'static str> {
        INVENTORY_COLUMNS
            .iter()
            .map(|column| column.header)
            .collect()
    }

    fn temp_xlsx_path(test_name: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "me-inventory-{test_name}-{}.xlsx",
            Uuid::new_v4().simple()
        ))
    }

    fn read_xlsx_member(path: &Path, member_name: &str) -> String {
        let file = File::open(path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let mut member = archive.by_name(member_name).unwrap();
        let mut contents = String::new();
        member.read_to_string(&mut contents).unwrap();
        contents
    }

    fn shared_strings(path: &Path) -> Vec<String> {
        let xml = read_xlsx_member(path, "xl/sharedStrings.xml");

        xml.split("<si")
            .skip(1)
            .map(|block| extract_text_nodes(block.split("</si>").next().unwrap_or_default()))
            .collect()
    }

    fn worksheet_rows(
        path: &Path,
        sheet_member_name: &str,
        shared_strings: &[String],
    ) -> Vec<Vec<String>> {
        let xml = read_xlsx_member(path, sheet_member_name);
        xml.split("<row ")
            .skip(1)
            .map(|row_block| {
                let row_body = row_block.split("</row>").next().unwrap_or_default();
                parse_cells(row_body, shared_strings)
            })
            .collect()
    }

    fn parse_cells(row_body: &str, shared_strings: &[String]) -> Vec<String> {
        let mut cells = Vec::new();
        let mut cursor = row_body;

        while let Some(cell_start) = cursor.find("<c ") {
            cursor = &cursor[cell_start..];
            let Some(tag_end) = cursor.find('>') else {
                break;
            };
            let cell_tag = &cursor[..=tag_end];
            let cell_body_start = tag_end + 1;
            let (cell_body, cursor_start) = if cell_tag.ends_with("/>") {
                ("", cell_body_start)
            } else {
                let Some(cell_end) = cursor[cell_body_start..].find("</c>") else {
                    break;
                };
                (
                    &cursor[cell_body_start..cell_body_start + cell_end],
                    cell_body_start + cell_end + "</c>".len(),
                )
            };
            let col = attr_value(cell_tag, "r")
                .map(column_index)
                .unwrap_or(cells.len());
            if cells.len() <= col {
                cells.resize(col + 1, String::new());
            }
            cells[col] = parse_cell_value(cell_tag, cell_body, shared_strings);
            cursor = &cursor[cursor_start..];
        }

        cells
    }

    fn parse_cell_value(cell_tag: &str, cell_body: &str, shared_strings: &[String]) -> String {
        if cell_tag.contains(r#"t="s""#) {
            let index = extract_value(cell_body)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap();
            return shared_strings[index].clone();
        }

        if cell_tag.contains(r#"t="inlineStr""#) {
            return extract_text_nodes(cell_body);
        }

        extract_value(cell_body).unwrap_or_default()
    }

    fn extract_value(cell_body: &str) -> Option<String> {
        let start = cell_body.find("<v>")? + "<v>".len();
        let end = cell_body[start..].find("</v>")?;
        Some(xml_unescape(&cell_body[start..start + end]))
    }

    fn extract_text_nodes(block: &str) -> String {
        let mut value = String::new();
        let mut cursor = block;

        while let Some(text_start) = cursor.find("<t") {
            cursor = &cursor[text_start..];
            let Some(tag_end) = cursor.find('>') else {
                break;
            };
            cursor = &cursor[tag_end + 1..];
            let Some(text_end) = cursor.find("</t>") else {
                break;
            };
            value.push_str(&xml_unescape(&cursor[..text_end]));
            cursor = &cursor[text_end + "</t>".len()..];
        }

        value
    }

    fn attr_value<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
        let pattern = format!(r#"{name}=""#);
        let start = tag.find(&pattern)? + pattern.len();
        let end = tag[start..].find('"')?;
        Some(&tag[start..start + end])
    }

    fn column_index(cell_ref: &str) -> usize {
        let mut index = 0usize;
        for byte in cell_ref
            .bytes()
            .take_while(|byte| byte.is_ascii_alphabetic())
        {
            index = index * 26 + usize::from(byte.to_ascii_uppercase() - b'A' + 1);
        }
        index.saturating_sub(1)
    }

    fn xml_unescape(value: &str) -> String {
        value
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
    }
}
