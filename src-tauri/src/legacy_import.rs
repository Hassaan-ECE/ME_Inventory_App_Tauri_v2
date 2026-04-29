mod paths;
mod sqlite;

use std::path::{Path, PathBuf};

use crate::{
    model::{CommandResult, LegacyImportResult},
    store::InventoryDb,
};

pub(crate) const LEGACY_SQLITE_ENV: &str = "ME_INVENTORY_LEGACY_SQLITE";
const DB_FILENAME: &str = "me_inventory.db";
const LEGACY_DB_FILENAME: &str = "me_lab_inventory.db";

pub(crate) fn resolve_legacy_sqlite_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    paths::legacy_sqlite_candidates(app)
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
    db.flush();
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
    db.flush();

    Ok(LegacyImportResult {
        imported,
        source_path,
        message: format!("Imported {imported} entries from legacy SQLite into FeOxDB."),
    })
}

fn import_legacy_sqlite_from_path(db: &InventoryDb, sqlite_path: &Path) -> CommandResult<usize> {
    let source = sqlite::LegacySqlite::open(sqlite_path)?;
    source.validate_entries()?;

    let mut imported = 0usize;
    let mut max_id = 0;
    source.for_each_entry(|entry| {
        if let Ok(id) = entry.id.parse::<i64>() {
            max_id = max_id.max(id);
        }
        db.put_entry(&entry)?;
        imported += 1;
        Ok(())
    })?;

    db.set_next_entry_id(max_id + 1)?;

    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::{
        env, fs,
        path::{Path, PathBuf},
    };
    use uuid::Uuid;

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
    fn legacy_import_sets_next_entry_id_after_import() {
        let root = unique_test_dir("legacy-import-next-id");
        fs::create_dir_all(&root).unwrap();
        let sqlite_path = root.join("legacy.db");
        create_test_sqlite(&sqlite_path, "entries", "entry_id", "entry_uuid");
        let db = InventoryDb::open_at(root.join("inventory.feox"), Some(sqlite_path)).unwrap();

        assert_eq!(ensure_legacy_imported(&db).unwrap(), 1);
        assert_eq!(db.next_entry_id().unwrap(), 8);
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

    #[test]
    fn legacy_import_missing_database_fails_without_marking_imported() {
        let root = unique_test_dir("legacy-import-missing-db");
        fs::create_dir_all(&root).unwrap();
        let sqlite_path = root.join("missing.db");
        let db = InventoryDb::open_at(root.join("inventory.feox"), Some(sqlite_path)).unwrap();

        let error = ensure_legacy_imported(&db).unwrap_err();

        assert!(error.contains("Could not open legacy SQLite database"));
        assert!(!db.has_legacy_import_marker());
        assert!(!db.has_entries().unwrap());
    }

    #[test]
    fn legacy_import_unknown_schema_fails_without_writing_entries() {
        let root = unique_test_dir("legacy-import-unknown-schema");
        fs::create_dir_all(&root).unwrap();
        let sqlite_path = root.join("legacy.db");
        let connection = Connection::open(&sqlite_path).unwrap();
        connection
            .execute("CREATE TABLE unrelated (id INTEGER PRIMARY KEY)", [])
            .unwrap();
        let db = InventoryDb::open_at(root.join("inventory.feox"), Some(sqlite_path)).unwrap();

        let error = ensure_legacy_imported(&db).unwrap_err();

        assert!(error.contains("entries or equipment table"));
        assert!(!db.has_legacy_import_marker());
        assert!(!db.has_entries().unwrap());
    }

    #[test]
    fn legacy_import_decode_error_fails_without_writing_entries() {
        let root = unique_test_dir("legacy-import-decode-error");
        fs::create_dir_all(&root).unwrap();
        let sqlite_path = root.join("legacy.db");
        create_test_sqlite(&sqlite_path, "entries", "entry_id", "entry_uuid");
        let connection = Connection::open(&sqlite_path).unwrap();
        connection
            .execute(
                "
                INSERT INTO entries (
                  entry_id,
                  entry_uuid,
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
                  8,
                  'bad-entry-uuid',
                  'ME-8',
                  '',
                  1,
                  'Mitutoyo',
                  'Caliper',
                  'Malformed row',
                  'ME',
                  'Lab',
                  '',
                  '',
                  '',
                  'active',
                  'working',
                  '',
                  'not-an-integer',
                  0,
                  0,
                  '',
                  '2025-01-01T00:00:00Z',
                  '2025-01-02T00:00:00Z'
                )
                ",
                [],
            )
            .unwrap();
        drop(connection);
        let db = InventoryDb::open_at(root.join("inventory.feox"), Some(sqlite_path)).unwrap();

        let error = ensure_legacy_imported(&db).unwrap_err();

        assert!(!error.is_empty());
        assert!(!db.has_legacy_import_marker());
        assert!(!db.has_entries().unwrap());
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
