#[allow(dead_code)]
#[path = "../src/domain/model.rs"]
mod model;
#[allow(dead_code)]
#[path = "../src/domain/query.rs"]
mod query;
#[allow(dead_code)]
#[path = "../src/storage/mod.rs"]
mod store;

use std::{
    env, fs,
    hint::black_box,
    time::{Duration, Instant},
};

use model::{FilterState, InventoryEntry, InventoryQueryInput, SortState};
use query::{get_inventory_counts, query_entries};
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
    let db = perf_db("current");
    seed_synthetic_entries(&db, 500, "current");
    db
}

fn synthetic_inventory_db(size: usize) -> InventoryDb {
    let db = perf_db(&format!("synthetic-{size}"));
    seed_synthetic_entries(&db, size, &format!("synthetic_{size}"));
    db
}

fn seed_synthetic_entries(db: &InventoryDb, size: usize, dataset: &str) {
    let started = Instant::now();
    for index in 0..size {
        db.put_entry(&synthetic_entry(index)).unwrap();
    }
    db.flush();
    println!(
        "PERF_SEED dataset={dataset} entries={size} elapsedMs={:.3}",
        duration_ms(started.elapsed())
    );
}

fn perf_db(prefix: &str) -> InventoryDb {
    let root = env::temp_dir().join(format!(
        "me-inventory-perf-{prefix}-{}",
        Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    InventoryDb::open_at_with_size(root.join("inventory.feox"), PERF_DB_SIZE).unwrap()
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
