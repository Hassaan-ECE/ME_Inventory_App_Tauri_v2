use rust_xlsxwriter::{Format, FormatAlign, FormatBorder};

use crate::model::InventoryEntry;

use super::columns::{CellKind, InventoryColumn};

pub(super) struct WorkbookFormats {
    centered_even: Format,
    centered_odd: Format,
    pub(super) header: Format,
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
    pub(super) fn new() -> Self {
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

    pub(super) fn format_for<'a>(
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
