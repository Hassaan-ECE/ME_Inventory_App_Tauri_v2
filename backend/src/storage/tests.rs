use super::*;
use std::{env, fs, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct TestSyncRecord {
    label: String,
    local_seq: u64,
}

#[test]
fn find_entry_supports_key_uuid_and_legacy_numeric_id() {
    let db = test_db();
    let entry = test_entry("42", "legacy-uuid-42");

    db.put_json(keys::entry_key(&entry.entry_uuid).as_bytes(), &entry)
        .unwrap();

    assert_eq!(
        db.find_entry(&keys::entry_key(&entry.entry_uuid))
            .unwrap()
            .unwrap()
            .id,
        "42"
    );
    assert_eq!(db.find_entry(&entry.entry_uuid).unwrap().unwrap().id, "42");
    assert_eq!(
        db.find_entry("42").unwrap().unwrap().entry_uuid,
        "legacy-uuid-42"
    );
    assert!(db.store.contains_key(keys::entry_id_key("42").as_bytes()));
}

#[test]
fn delete_entry_removes_entry_and_id_index() {
    let db = test_db();
    let entry = test_entry("7", "uuid-7");

    db.put_entry(&entry).unwrap();
    assert!(db
        .store
        .contains_key(keys::entry_key(&entry.entry_uuid).as_bytes()));
    assert!(db
        .store
        .contains_key(keys::entry_id_key(&entry.id).as_bytes()));

    db.delete_entry(&entry).unwrap();

    assert!(!db
        .store
        .contains_key(keys::entry_key(&entry.entry_uuid).as_bytes()));
    assert!(!db
        .store
        .contains_key(keys::entry_id_key(&entry.id).as_bytes()));
}

#[test]
fn sync_metadata_identity_and_local_seq_are_stable() {
    let db = test_db();

    let metadata = db.sync_metadata().unwrap();
    assert_eq!(metadata.next_local_seq, 1);
    assert_eq!(metadata.schema_version, None);
    assert_eq!(metadata.sync_schema_version, None);
    assert_eq!(metadata.client_id, None);
    assert_eq!(metadata.device_id, None);
    assert_eq!(metadata.last_snapshot_id, None);

    db.set_schema_version(1).unwrap();
    db.set_sync_schema_version(1).unwrap();
    db.set_last_snapshot_id("snapshot-000001").unwrap();

    let client_id = db.get_or_create_client_id().unwrap();
    assert_eq!(db.get_or_create_client_id().unwrap(), client_id);
    let device_id = db.get_or_create_device_id().unwrap();
    assert_eq!(db.get_or_create_device_id().unwrap(), device_id);

    assert!(db.set_next_local_seq(0).is_err());
    assert_eq!(db.reserve_next_local_seq().unwrap(), 1);
    assert_eq!(db.reserve_next_local_seq().unwrap(), 2);
    assert_eq!(db.next_local_seq().unwrap(), 3);

    let metadata = db.sync_metadata().unwrap();
    assert_eq!(metadata.schema_version, Some(1));
    assert_eq!(metadata.sync_schema_version, Some(1));
    assert_eq!(metadata.client_id.as_deref(), Some(client_id.as_str()));
    assert_eq!(metadata.device_id.as_deref(), Some(device_id.as_str()));
    assert_eq!(metadata.next_local_seq, 3);
    assert_eq!(
        metadata.last_snapshot_id.as_deref(),
        Some("snapshot-000001")
    );

    db.clear_last_snapshot_id().unwrap();
    assert_eq!(db.last_snapshot_id().unwrap(), None);
}

#[test]
fn sync_records_round_trip_and_scan_in_key_order() {
    let db = test_db();

    db.put_sync_outbox_record(2, &sync_record("second", 2))
        .unwrap();
    db.put_sync_outbox_record(1, &sync_record("first", 1))
        .unwrap();

    assert_eq!(
        db.sync_outbox_record::<TestSyncRecord>(1).unwrap(),
        Some(sync_record("first", 1))
    );

    let mut outbox = Vec::new();
    db.scan_sync_outbox_records::<TestSyncRecord, _>(None, 10, |local_seq, record| {
        outbox.push((local_seq, record.label));
        Ok(true)
    })
    .unwrap();
    assert_eq!(
        outbox,
        vec![(1, "first".to_string()), (2, "second".to_string())]
    );

    let mut outbox_after_one = Vec::new();
    db.scan_sync_outbox_records::<TestSyncRecord, _>(Some(1), 10, |local_seq, record| {
        outbox_after_one.push((local_seq, record.label));
        Ok(true)
    })
    .unwrap();
    assert_eq!(outbox_after_one, vec![(2, "second".to_string())]);

    db.delete_sync_outbox_record(1).unwrap();
    assert_eq!(db.sync_outbox_record::<TestSyncRecord>(1).unwrap(), None);

    db.put_sync_applied_marker("op-1", &sync_record("applied", 1))
        .unwrap();
    assert!(db.has_sync_applied_marker("op-1").unwrap());
    assert_eq!(
        db.sync_applied_marker::<TestSyncRecord>("op-1")
            .unwrap()
            .unwrap()
            .label,
        "applied"
    );
    db.delete_sync_applied_marker("op-1").unwrap();
    assert!(!db.has_sync_applied_marker("op-1").unwrap());

    db.put_sync_client_seq_marker("client-1", 7, &sync_record("client-seq", 7))
        .unwrap();
    assert_eq!(
        db.sync_client_seq_marker::<TestSyncRecord>("client-1", 7)
            .unwrap()
            .unwrap()
            .label,
        "client-seq"
    );
    db.delete_sync_client_seq_marker("client-1", 7).unwrap();
    assert_eq!(
        db.sync_client_seq_marker::<TestSyncRecord>("client-1", 7)
            .unwrap(),
        None
    );

    db.set_sync_watermark("client-1", 9).unwrap();
    assert_eq!(db.sync_watermark("client-1").unwrap(), Some(9));
    let mut watermarks = Vec::new();
    db.scan_sync_watermarks(10, |client_id, local_seq| {
        watermarks.push((client_id, local_seq));
        Ok(true)
    })
    .unwrap();
    assert_eq!(watermarks, vec![("client-1".to_string(), 9)]);
    db.clear_sync_watermark("client-1").unwrap();
    assert_eq!(db.sync_watermark("client-1").unwrap(), None);

    db.put_sync_tombstone("entry-1", &sync_record("deleted", 8))
        .unwrap();
    assert!(db.has_sync_tombstone("entry-1").unwrap());
    assert_eq!(
        db.sync_tombstone::<TestSyncRecord>("entry-1")
            .unwrap()
            .unwrap()
            .label,
        "deleted"
    );
    let mut tombstones = Vec::new();
    db.scan_sync_tombstones::<TestSyncRecord, _>(10, |entry_uuid, record| {
        tombstones.push((entry_uuid, record.label));
        Ok(true)
    })
    .unwrap();
    assert_eq!(
        tombstones,
        vec![("entry-1".to_string(), "deleted".to_string())]
    );
    db.delete_sync_tombstone("entry-1").unwrap();
    assert!(!db.has_sync_tombstone("entry-1").unwrap());

    db.put_sync_corrupt_record("hash-1", &sync_record("bad-json", 10))
        .unwrap();
    assert_eq!(
        db.sync_corrupt_record::<TestSyncRecord>("hash-1")
            .unwrap()
            .unwrap()
            .label,
        "bad-json"
    );

    let mut corrupt_records = Vec::new();
    db.scan_sync_corrupt_records::<TestSyncRecord, _>(10, |record_id, record| {
        corrupt_records.push((record_id, record.label));
        Ok(true)
    })
    .unwrap();
    assert_eq!(
        corrupt_records,
        vec![("hash-1".to_string(), "bad-json".to_string())]
    );

    let mut raw_outbox_keys = Vec::new();
    db.scan_sync_range(SyncKeyspace::Outbox, 10, |key, _| {
        raw_outbox_keys.push(String::from_utf8(key.to_vec()).unwrap());
        Ok(true)
    })
    .unwrap();
    assert_eq!(raw_outbox_keys, vec!["sync:outbox:000000000002"]);

    db.delete_sync_corrupt_record("hash-1").unwrap();
    assert_eq!(
        db.sync_corrupt_record::<TestSyncRecord>("hash-1").unwrap(),
        None
    );

    for keyspace in [
        SyncKeyspace::Applied,
        SyncKeyspace::ClientSeq,
        SyncKeyspace::Watermark,
        SyncKeyspace::Tombstone,
        SyncKeyspace::Conflict,
        SyncKeyspace::CorruptRemote,
    ] {
        db.scan_sync_range(keyspace, 10, |_, _| Ok(true)).unwrap();
    }
}

#[test]
fn sync_keys_do_not_appear_in_entry_scans() {
    let db = test_db();

    db.set_schema_version(1).unwrap();
    db.set_sync_schema_version(1).unwrap();
    db.set_client_id("client-1").unwrap();
    db.set_device_id("device-1").unwrap();
    db.set_next_local_seq(2).unwrap();
    db.set_sync_watermark("client-1", 1).unwrap();
    db.put_sync_outbox_record(1, &sync_record("pending", 1))
        .unwrap();
    db.put_sync_applied_marker("op-1", &sync_record("applied", 1))
        .unwrap();
    db.put_sync_tombstone("entry-1", &sync_record("deleted", 1))
        .unwrap();
    db.put_sync_corrupt_record("hash-1", &sync_record("corrupt", 1))
        .unwrap();
    db.put_sync_value("meta:test_raw", b"meta").unwrap();
    db.put_sync_value("sync:test_raw", b"sync").unwrap();

    assert!(!db.has_entries().unwrap());
    assert!(db.load_entries().unwrap().is_empty());
    assert_eq!(
        db.get_sync_value("meta:test_raw").unwrap().as_deref(),
        Some(&b"meta"[..])
    );
    assert!(db.put_sync_value("entry:test_raw", b"bad").is_err());

    let entry = test_entry("11", "uuid-11");
    db.put_entry(&entry).unwrap();

    let entries = db.load_entries().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entry_uuid, "uuid-11");
}

fn sync_record(label: &str, local_seq: u64) -> TestSyncRecord {
    TestSyncRecord {
        label: label.to_string(),
        local_seq,
    }
}

fn test_entry(id: &str, entry_uuid: &str) -> crate::model::InventoryEntry {
    crate::model::InventoryEntry {
        id: id.to_string(),
        database_id: id.parse::<i64>().ok(),
        entry_uuid: entry_uuid.to_string(),
        asset_number: format!("ME-{id}"),
        serial_number: String::new(),
        qty: Some(1.0),
        manufacturer: "Mitutoyo".to_string(),
        model: String::new(),
        description: "Caliper".to_string(),
        project_name: String::new(),
        location: "Lab".to_string(),
        assigned_to: String::new(),
        links: String::new(),
        notes: String::new(),
        lifecycle_status: "active".to_string(),
        working_status: "unknown".to_string(),
        condition: String::new(),
        verified_in_survey: false,
        archived: false,
        manual_entry: false,
        picture_path: String::new(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn test_db() -> InventoryDb {
    let root = unique_test_dir("store");
    fs::create_dir_all(&root).unwrap();
    InventoryDb::open_at(root.join("inventory.feox")).unwrap()
}

fn unique_test_dir(prefix: &str) -> PathBuf {
    env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4().simple()))
}
