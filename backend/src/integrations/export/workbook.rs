mod columns;
mod formats;

use std::path::Path;

use rust_xlsxwriter::{Workbook, Worksheet, XlsxError};

use crate::model::{CommandResult, InventoryEntry};

use self::{
    columns::{inventory_text, yes_if, InventoryColumn, InventoryField, INVENTORY_COLUMNS},
    formats::WorkbookFormats,
};
use super::ExcelExportStats;

const INVENTORY_SHEET: &str = "Inventory";
const ARCHIVE_SHEET: &str = "Archive";

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

#[cfg(test)]
pub(super) fn inventory_headers() -> Vec<&'static str> {
    INVENTORY_COLUMNS
        .iter()
        .map(|column| column.header)
        .collect()
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

fn export_error(error: impl std::fmt::Display) -> String {
    format!("Excel export failed: {error}")
}
