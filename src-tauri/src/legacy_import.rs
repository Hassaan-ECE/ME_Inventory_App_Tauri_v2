use rusqlite::{Connection, OpenFlags, Row};
use std::{
    env,
    path::{Path, PathBuf},
};
use tauri::Manager;
use uuid::Uuid;

use crate::{
    model::{
        db_error, default_lifecycle_status, default_working_status, CommandResult, InventoryEntry,
        LegacyImportResult,
    },
    store::InventoryDb,
};

pub(crate) const LEGACY_SQLITE_ENV: &str = "ME_INVENTORY_LEGACY_SQLITE";
const DB_FILENAME: &str = "me_inventory.db";
const LEGACY_DB_FILENAME: &str = "me_lab_inventory.db";

pub(crate) fn resolve_legacy_sqlite_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    legacy_sqlite_candidates(app)
        .into_iter()
        .find(|candidate| candidate.exists())
}

pub(crate) fn ensure_legacy_imported(db: &InventoryDb) -> CommandResult<usize> {
    if db.has_legacy_import_marker() || db.has_entries()? {
        return Ok(0);
    }

    let Some(sqlite_path) = db.legacy_sqlite_path() else {
        return Ok(0);
    };

    let imported = import_legacy_sqlite_from_path(db, sqlite_path)?;
    db.mark_legacy_imported(&sqlite_path.to_string_lossy())?;
    Ok(imported)
}

pub(crate) fn import_legacy_sqlite(db: &InventoryDb) -> CommandResult<LegacyImportResult> {
    let sqlite_path = db.legacy_sqlite_path().ok_or_else(|| {
        format!("Legacy SQLite database was not found. Set {LEGACY_SQLITE_ENV} to import manually.")
    })?;
    let source_path = sqlite_path.to_string_lossy().into_owned();

    if db.has_legacy_import_marker() || db.has_entries()? {
        return Ok(LegacyImportResult {
            imported: 0,
            source_path,
            message: "Legacy SQLite import skipped because FeOxDB already has entries.".to_string(),
        });
    }

    let imported = import_legacy_sqlite_from_path(db, sqlite_path)?;
    db.mark_legacy_imported(&source_path)?;

    Ok(LegacyImportResult {
        imported,
        source_path,
        message: format!("Imported {imported} entries from legacy SQLite into FeOxDB."),
    })
}

fn legacy_sqlite_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(path) = env::var(LEGACY_SQLITE_ENV) {
        let path = path.trim();
        if !path.is_empty() {
            candidates.push(PathBuf::from(path));
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(project_root) = manifest_dir.parent() {
        candidates.push(project_root.join("data").join(DB_FILENAME));
        candidates.push(project_root.join("data").join(LEGACY_DB_FILENAME));
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join(DB_FILENAME));
        candidates.push(resource_dir.join(LEGACY_DB_FILENAME));
        candidates.push(resource_dir.join("data").join(DB_FILENAME));
        candidates.push(resource_dir.join("data").join(LEGACY_DB_FILENAME));
    }

    if let Ok(current_dir) = env::current_dir() {
        candidates.push(current_dir.join("data").join(DB_FILENAME));
        candidates.push(current_dir.join("data").join(LEGACY_DB_FILENAME));
    }

    dedupe_paths(candidates)
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut result: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !result.iter().any(|existing| same_path(existing, &path)) {
            result.push(path);
        }
    }
    result
}

fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn import_legacy_sqlite_from_path(db: &InventoryDb, sqlite_path: &Path) -> CommandResult<usize> {
    let connection = Connection::open_with_flags(sqlite_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("Could not open legacy SQLite database: {error}"))?;

    let table = detect_inventory_table(&connection)?;
    let (id_column, uuid_column) = match table.as_str() {
        "equipment" => ("record_id", "record_uuid"),
        _ => ("entry_id", "entry_uuid"),
    };

    let sql = format!(
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
    );

    let mut statement = connection.prepare(&sql).map_err(db_error)?;
    let entries = statement
        .query_map([], sqlite_row_to_entry)
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;

    let mut max_id = 0;
    for entry in &entries {
        if let Ok(id) = entry.id.parse::<i64>() {
            max_id = max_id.max(id);
        }
        db.put_entry(entry)?;
    }
    db.set_next_entry_id(max_id + 1)?;

    Ok(entries.len())
}

fn detect_inventory_table(connection: &Connection) -> CommandResult<String> {
    if sqlite_table_exists(connection, "entries")? {
        return Ok("entries".to_string());
    }
    if sqlite_table_exists(connection, "equipment")? {
        return Ok("equipment".to_string());
    }

    Err("Legacy SQLite database does not contain an entries or equipment table.".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::fs;

    #[test]
    fn legacy_import_is_idempotent_after_first_startup() {
        let root = unique_test_dir("legacy-import-idempotent");
        fs::create_dir_all(&root).unwrap();
        let sqlite_path = root.join("legacy.db");
        create_test_sqlite(&sqlite_path, "entries", "entry_id", "entry_uuid");
        let db = InventoryDb::open_at(root.join("inventory.feox"), Some(sqlite_path)).unwrap();

        assert_eq!(ensure_legacy_imported(&db).unwrap(), 1);
        assert_eq!(ensure_legacy_imported(&db).unwrap(), 0);
        assert_eq!(db.load_entries().unwrap().len(), 1);
    }

    #[test]
    fn legacy_import_supports_old_equipment_table() {
        let root = unique_test_dir("legacy-import-equipment");
        fs::create_dir_all(&root).unwrap();
        let sqlite_path = root.join("legacy.db");
        create_test_sqlite(&sqlite_path, "equipment", "record_id", "record_uuid");
        let db = InventoryDb::open_at(root.join("inventory.feox"), Some(sqlite_path)).unwrap();

        assert_eq!(ensure_legacy_imported(&db).unwrap(), 1);
        let entries = db.load_entries().unwrap();
        assert_eq!(entries[0].id, "7");
        assert_eq!(entries[0].entry_uuid, "test-entry-uuid");
    }

    fn create_test_sqlite(path: &Path, table: &str, id_column: &str, uuid_column: &str) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute(
                &format!(
                    "
                    CREATE TABLE {table} (
                      {id_column} INTEGER PRIMARY KEY,
                      {uuid_column} TEXT,
                      asset_number TEXT,
                      serial_number TEXT,
                      qty REAL,
                      manufacturer TEXT,
                      model TEXT,
                      description TEXT,
                      project_name TEXT,
                      location TEXT,
                      assigned_to TEXT,
                      links TEXT,
                      notes TEXT,
                      lifecycle_status TEXT,
                      working_status TEXT,
                      condition TEXT,
                      verified_in_survey INTEGER,
                      is_archived INTEGER,
                      manual_entry INTEGER,
                      picture_path TEXT,
                      created_at TEXT,
                      updated_at TEXT
                    )
                    "
                ),
                [],
            )
            .unwrap();
        connection
            .execute(
                &format!(
                    "
                    INSERT INTO {table} (
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
                    ) VALUES (
                      7,
                      'test-entry-uuid',
                      'ME-7',
                      '',
                      2,
                      'Mitutoyo',
                      'Caliper',
                      'Digital caliper',
                      'ME',
                      'Lab',
                      '',
                      '',
                      '',
                      'active',
                      'working',
                      '',
                      1,
                      0,
                      0,
                      '',
                      '2026-01-01T00:00:00Z',
                      '2026-01-02T00:00:00Z'
                    )
                    "
                ),
                [],
            )
            .unwrap();
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4().simple()))
    }
}
