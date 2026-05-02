mod workbook;

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::{
    model::{CommandResult, InventoryEntry},
    store::InventoryDb,
};

pub(crate) const DEFAULT_EXCEL_EXPORT_FILENAME: &str = "ME_Inventory_Export.xlsx";

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
    let entries = db.load_entries()?;
    write_inventory_workbook(&entries, output_path)
}

pub(crate) fn write_inventory_workbook(
    entries: &[InventoryEntry],
    output_path: impl AsRef<Path>,
) -> CommandResult<ExcelExportStats> {
    workbook::write_inventory_workbook(entries, output_path)
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

fn pick_export_path(app: &AppHandle) -> Option<PathBuf> {
    app.dialog()
        .file()
        .set_title("Export All Entries to Excel")
        .set_file_name(DEFAULT_EXCEL_EXPORT_FILENAME)
        .add_filter("Excel Workbook", &["xlsx"])
        .blocking_save_file()
        .and_then(|file_path| file_path.simplified().into_path().ok())
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

    #[test]
    fn inventory_sheet_writes_formula_like_user_text_as_strings() {
        let path = temp_xlsx_path("formula-like-text");
        let mut entry = test_entry("7", false, false, Some(1.0));
        entry.manufacturer = "+SUM(1,2)".to_string();
        entry.model = "-10+20".to_string();
        entry.description = "=2+3".to_string();
        entry.notes = "@username".to_string();

        write_inventory_workbook(&[entry], &path).unwrap();

        let worksheet_xml = read_xlsx_member(&path, "xl/worksheets/sheet1.xml");
        assert!(!worksheet_xml.contains("<f>"));
        assert!(!worksheet_xml.contains("<f "));

        let shared_strings = shared_strings(&path);
        assert!(shared_strings.contains(&"+SUM(1,2)".to_string()));
        assert!(shared_strings.contains(&"-10+20".to_string()));
        assert!(shared_strings.contains(&"=2+3".to_string()));
        assert!(shared_strings.contains(&"@username".to_string()));

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
        super::workbook::inventory_headers()
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
