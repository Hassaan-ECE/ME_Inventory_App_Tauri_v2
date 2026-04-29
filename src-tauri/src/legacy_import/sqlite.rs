use std::path::Path;

use rusqlite::{Connection, OpenFlags, Row};
use uuid::Uuid;

use crate::model::{
    db_error, default_lifecycle_status, default_working_status, CommandResult, InventoryEntry,
};

pub(super) struct LegacySqlite {
    connection: Connection,
    select_sql: String,
}

impl LegacySqlite {
    pub(super) fn open(sqlite_path: &Path) -> CommandResult<Self> {
        let connection = Connection::open_with_flags(sqlite_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| format!("Could not open legacy SQLite database: {error}"))?;

        let table = InventoryTable::detect(&connection)?;
        Ok(Self {
            connection,
            select_sql: table.select_sql(),
        })
    }

    pub(super) fn validate_entries(&self) -> CommandResult<()> {
        self.for_each_entry(|_| Ok(()))
    }

    pub(super) fn for_each_entry(
        &self,
        mut visit: impl FnMut(InventoryEntry) -> CommandResult<()>,
    ) -> CommandResult<()> {
        let mut statement = self
            .connection
            .prepare(&self.select_sql)
            .map_err(db_error)?;
        let rows = statement
            .query_map([], sqlite_row_to_entry)
            .map_err(db_error)?;
        for entry in rows {
            let entry = entry.map_err(db_error)?;
            visit(entry)?;
        }
        Ok(())
    }
}

enum InventoryTable {
    Entries,
    Equipment,
}

impl InventoryTable {
    fn detect(connection: &Connection) -> CommandResult<Self> {
        if sqlite_table_exists(connection, "entries")? {
            return Ok(Self::Entries);
        }
        if sqlite_table_exists(connection, "equipment")? {
            return Ok(Self::Equipment);
        }

        Err("Legacy SQLite database does not contain an entries or equipment table.".to_string())
    }

    fn table_name(&self) -> &'static str {
        match self {
            Self::Entries => "entries",
            Self::Equipment => "equipment",
        }
    }

    fn id_column(&self) -> &'static str {
        match self {
            Self::Entries => "entry_id",
            Self::Equipment => "record_id",
        }
    }

    fn uuid_column(&self) -> &'static str {
        match self {
            Self::Entries => "entry_uuid",
            Self::Equipment => "record_uuid",
        }
    }

    fn select_sql(&self) -> String {
        let table = self.table_name();
        let id_column = self.id_column();
        let uuid_column = self.uuid_column();

        format!(
            "
            SELECT
              {id_column},
              {uuid_column},
              asset_number,
              serial_number,
              qty,
              manufacturer,
              model,
              description,
              project_name,
              location,
              assigned_to,
              links,
              notes,
              lifecycle_status,
              working_status,
              condition,
              verified_in_survey,
              is_archived,
              manual_entry,
              picture_path,
              created_at,
              updated_at
            FROM {table}
            ORDER BY updated_at DESC, {id_column} DESC
            "
        )
    }
}

fn sqlite_table_exists(connection: &Connection, table_name: &str) -> CommandResult<bool> {
    let row = connection
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' AND name = ? LIMIT 1")
        .map_err(db_error)?
        .exists([table_name])
        .map_err(db_error)?;
    Ok(row)
}

fn sqlite_row_to_entry(row: &Row<'_>) -> rusqlite::Result<InventoryEntry> {
    let entry_id: i64 = row.get(0)?;
    let entry_uuid: String =
        optional_text(row, 1)?.unwrap_or_else(|| Uuid::new_v4().simple().to_string());

    Ok(InventoryEntry {
        id: entry_id.to_string(),
        database_id: Some(entry_id),
        entry_uuid,
        asset_number: optional_text(row, 2)?.unwrap_or_default(),
        serial_number: optional_text(row, 3)?.unwrap_or_default(),
        qty: row.get::<_, Option<f64>>(4)?,
        manufacturer: optional_text(row, 5)?.unwrap_or_default(),
        model: optional_text(row, 6)?.unwrap_or_default(),
        description: optional_text(row, 7)?.unwrap_or_default(),
        project_name: optional_text(row, 8)?.unwrap_or_default(),
        location: optional_text(row, 9)?.unwrap_or_default(),
        assigned_to: optional_text(row, 10)?.unwrap_or_default(),
        links: optional_text(row, 11)?.unwrap_or_default(),
        notes: optional_text(row, 12)?.unwrap_or_default(),
        lifecycle_status: optional_text(row, 13)?.unwrap_or_else(default_lifecycle_status),
        working_status: optional_text(row, 14)?.unwrap_or_else(default_working_status),
        condition: optional_text(row, 15)?.unwrap_or_default(),
        verified_in_survey: row.get::<_, i64>(16)? != 0,
        archived: row.get::<_, i64>(17)? != 0,
        manual_entry: row.get::<_, i64>(18)? != 0,
        picture_path: optional_text(row, 19)?.unwrap_or_default(),
        created_at: optional_text(row, 20)?.unwrap_or_default(),
        updated_at: optional_text(row, 21)?.unwrap_or_default(),
    })
}

fn optional_text(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<String>> {
    row.get::<_, Option<String>>(index)
}
