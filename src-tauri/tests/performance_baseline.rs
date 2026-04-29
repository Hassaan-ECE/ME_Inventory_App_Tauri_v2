#[allow(dead_code)]
#[path = "../src/model.rs"]
mod model;
#[allow(dead_code)]
#[path = "../src/query.rs"]
mod query;
#[allow(dead_code)]
#[path = "../src/store.rs"]
mod store;

use std::{
    env, fs,
    hint::black_box,
    path::PathBuf,
    time::{Duration, Instant},
};

use model::{FilterState, InventoryEntry, InventoryQueryInput, SortState};
use query::{get_inventory_counts, query_entries};
use rusqlite::{Connection, OpenFlags, Row};
use serde::Serialize;
use store::InventoryDb;
use uuid::Uuid;

const PERF_DB_SIZE: u64 = 128 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Metric {
    dataset: String,
    entries: usize,
    iterations: usize,
    max_ms: f64,
    median_ms: f64,
    min_ms: f64,
    operation: String,
    p95_ms: f64,
}

#[ignore = "benchmark harness; run explicitly with --ignored --nocapture"]
#[test]
fn inventory_backend_performance_baseline() {
    let mut metrics = Vec::new();

    let current_db = current_inventory_db();
    let current_entries = current_db.load_entries().unwrap();
    metrics.extend(measure_dataset(
        "current",
        current_entries.len(),
        &current_db,
    ));

    for size in [1_000, 10_000] {
        let db = synthetic_inventory_db(size);
        metrics.extend(measure_dataset(&format!("synthetic_{size}"), size, &db));
    }

    println!(
        "PERF_BACKEND_JSON={}",
        serde_json::to_string_pretty(&metrics).unwrap()
    );
}

fn measure_dataset(dataset: &str, entries: usize, db: &InventoryDb) -> Vec<Metric> {
    let mut metrics = Vec::new();
    let query_input = benchmark_query_input();
    let loaded_entries = db.load_entries().unwrap();

    metrics.push(measure_operation(
        dataset,
        entries,
        "load_entries",
        9,
        || {
            let result = db.load_entries().unwrap();
            black_box(result.len());
        },
    ));

    metrics.push(measure_operation(
        dataset,
        entries,
        "query_inventory_equivalent_load_count_filter_sort_page",
        9,
        || {
            let all_entries = db.load_entries().unwrap();
            let counts = get_inventory_counts(&all_entries);
            let (page, total_filtered) = query_entries(&all_entries, query_input.clone());
            black_box((counts.total, page.len(), total_filtered));
        },
    ));

    metrics.push(measure_operation(
        dataset,
        entries,
        "query_entries_in_memory_filter_sort_page",
        25,
        || {
            let (page, total_filtered) = query_entries(&loaded_entries, query_input.clone());
            black_box((page.len(), total_filtered));
        },
    ));

    metrics
}

fn measure_operation(
    dataset: &str,
    entries: usize,
    operation: &str,
    iterations: usize,
    mut run: impl FnMut(),
) -> Metric {
    let mut samples: Vec<Duration> = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let started = Instant::now();
        run();
        samples.push(started.elapsed());
    }

    samples.sort_unstable();
    let last_index = samples.len().saturating_sub(1);
    let p95_index = ((last_index as f64) * 0.95).ceil() as usize;

    Metric {
        dataset: dataset.to_string(),
        entries,
        iterations,
        max_ms: duration_ms(samples[last_index]),
        median_ms: duration_ms(samples[samples.len() / 2]),
        min_ms: duration_ms(samples[0]),
        operation: operation.to_string(),
        p95_ms: duration_ms(samples[p95_index.min(last_index)]),
    }
}

fn benchmark_query_input() -> InventoryQueryInput {
    InventoryQueryInput {
        filters: FilterState {
            manufacturer: "maker".to_string(),
            location: "bay".to_string(),
            ..FilterState::default()
        },
        limit: Some(250),
        offset: Some(0),
        query: "calibration".to_string(),
        scope: "inventory".to_string(),
        sort: SortState {
            column: "manufacturer".to_string(),
            direction: "asc".to_string(),
        },
    }
}

fn current_inventory_db() -> InventoryDb {
    let sqlite_path = project_root().join("data").join("me_inventory.db");
    let db = perf_db("current", None);
    for entry in read_sqlite_entries(&sqlite_path) {
        db.put_entry(&entry).unwrap();
    }
    db.flush();
    db
}

fn synthetic_inventory_db(size: usize) -> InventoryDb {
    let db = perf_db(&format!("synthetic-{size}"), None);
    let started = Instant::now();
    for index in 0..size {
        db.put_entry(&synthetic_entry(index)).unwrap();
    }
    db.flush();
    println!(
        "PERF_SEED dataset=synthetic_{size} entries={size} elapsedMs={:.3}",
        duration_ms(started.elapsed())
    );
    db
}

fn perf_db(prefix: &str, legacy_sqlite_path: Option<PathBuf>) -> InventoryDb {
    let root = env::temp_dir().join(format!(
        "me-inventory-perf-{prefix}-{}",
        Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    InventoryDb::open_at_with_size(
        root.join("inventory.feox"),
        legacy_sqlite_path,
        PERF_DB_SIZE,
    )
    .unwrap()
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri has a parent project root")
        .to_path_buf()
}

fn read_sqlite_entries(sqlite_path: &PathBuf) -> Vec<InventoryEntry> {
    let connection = Connection::open_with_flags(sqlite_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap_or_else(|error| panic!("could not open {}: {error}", sqlite_path.display()));
    let table = if sqlite_table_exists(&connection, "entries") {
        LegacyTable::Entries
    } else if sqlite_table_exists(&connection, "equipment") {
        LegacyTable::Equipment
    } else {
        panic!("legacy SQLite database has no entries or equipment table");
    };

    let mut statement = connection.prepare(&table.select_sql()).unwrap();
    let rows = statement.query_map([], sqlite_row_to_entry).unwrap();
    rows.map(|row| row.unwrap()).collect()
}

enum LegacyTable {
    Entries,
    Equipment,
}

impl LegacyTable {
    fn select_sql(&self) -> String {
        let (table, id_column, uuid_column) = match self {
            Self::Entries => ("entries", "entry_id", "entry_uuid"),
            Self::Equipment => ("equipment", "record_id", "record_uuid"),
        };

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

fn sqlite_table_exists(connection: &Connection, table_name: &str) -> bool {
    connection
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' AND name = ? LIMIT 1")
        .unwrap()
        .exists([table_name])
        .unwrap()
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
        lifecycle_status: optional_text(row, 13)?.unwrap_or_else(|| "active".to_string()),
        working_status: optional_text(row, 14)?.unwrap_or_else(|| "unknown".to_string()),
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

fn synthetic_entry(index: usize) -> InventoryEntry {
    let id = index + 1;
    let maker = format!("Maker {:02}", index % 37);
    let location = format!("Bay {}", index % 16);
    let lifecycle_status = match index % 5 {
        0 => "active",
        1 => "repair",
        2 => "scrapped",
        3 => "missing",
        _ => "rental",
    };
    let working_status = match index % 4 {
        0 => "working",
        1 => "limited",
        2 => "not_working",
        _ => "unknown",
    };

    InventoryEntry {
        id: id.to_string(),
        database_id: Some(id as i64),
        entry_uuid: format!("perf-entry-{id:05}"),
        asset_number: format!("ME-{id:05}"),
        serial_number: format!("SN-{id:05}"),
        qty: (index % 11 != 0).then_some(((index % 23) + 1) as f64),
        manufacturer: maker,
        model: format!("Model {}", index % 113),
        description: format!("Calibration fixture and measurement asset {id}"),
        project_name: format!("Project {}", index % 29),
        location,
        assigned_to: format!("User {}", index % 19),
        links: if index % 13 == 0 {
            format!("https://example.com/assets/{id}")
        } else {
            String::new()
        },
        notes: format!("Synthetic performance note {id} with calibration history"),
        lifecycle_status: lifecycle_status.to_string(),
        working_status: working_status.to_string(),
        condition: if index % 7 == 0 {
            "Calibration due".to_string()
        } else {
            "Good".to_string()
        },
        verified_in_survey: index % 3 == 0,
        archived: index % 10 == 0,
        manual_entry: false,
        picture_path: String::new(),
        created_at: format!("2026-04-{:02}T08:00:00.000Z", (index % 28) + 1),
        updated_at: format!("2026-04-{:02}T12:00:00.000Z", (index % 28) + 1),
    }
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
