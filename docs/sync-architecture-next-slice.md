# Shared Sync Next Slice

This is the implementation-ready vertical slice for shared sync after the parity wave. It keeps the current `InventoryEntry` compatibility projection and does not introduce the future SKU ledger model yet.

Status after the Electron-parity sync wave: the operation-log slice is implemented in Rust. Local FeOxDB now stores sync identity, local sequence metadata, durable outbox records, applied markers, per-entry sync state, tombstones, stale-operation conflict records, and corrupt remote-file records. `sync_inventory` bootstraps existing entries once, pushes local operation files, pulls remote operation files, applies last-write-wins by `(mutation_ts_utc, op_id)`, and reports the local sync revision in `InventorySharedStatus`. A native shared-ops watcher emits `inventory:shared-changed`; the frontend coalesces delayed mutation syncs and queues one follow-up pass if sync is already in flight. Snapshots, manifest compaction, user-facing conflict resolution, locked-file smoke, and shared media storage remain out of scope.

## Scope

- Keep local FeOxDB as the runtime database.
- Keep current records under `entry:{entry_uuid}`.
- Add sync metadata under `meta:*` and `sync:*` keys so `load_entries()` range scans remain isolated to entries.
- Use `entry_uuid` as the sync entity ID. Numeric `id` and `databaseId` stay local display/import metadata.
- Do not mutate a shared FeOxDB file.

## Local Keyspaces

```text
entry:{entry_uuid}
__meta:next_entry_id
__meta:legacy_import_path

meta:schema_version
meta:sync_schema_version
meta:client_id
meta:device_id
meta:next_local_seq
meta:sync_revision
meta:last_snapshot_id

sync:outbox:{local_seq_padded}
sync:applied:{op_id}
sync:seq:{client_id}:{local_seq_padded}
sync:entry_state:{entry_uuid}
sync:tombstone:{entry_uuid}
sync:watermark:{client_id}
sync:conflict:{conflict_id}
sync:corrupt_remote:{content_hash_or_file_id}
```

## Operation Envelope

```json
{
  "schema_version": 1,
  "op_id": "uuid",
  "client_id": "uuid",
  "device_id": "machine-or-install-id",
  "local_seq": 1,
  "app_version": "0.9.6",
  "created_at_utc": "2026-04-26T00:00:00.000Z",
  "type": "inventory.entry.update",
  "entity_type": "inventory_entry",
  "entity_id": "entry_uuid",
  "base_version": null,
  "mutation_ts_utc": "2026-04-26T00:00:00.000Z",
  "payload": {},
  "checksum": "sha256-canonical-json-without-checksum"
}
```

First operation types:

- `inventory.entry.create`
- `inventory.entry.update`
- `inventory.entry.verify`
- `inventory.entry.archive`
- `inventory.entry.delete`

Non-delete operations carry the full `InventoryEntry` projection and optional `changed_fields`. Delete operations carry `entry_uuid` and `deleted_at_utc` only.

## Shared Drive Layout

Resolve the shared root from `ME_LAB_SHARED_ROOT`, falling back to:

```text
S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME
```

Use this artifact layout:

```text
<root>\shared\inventory\manifest.json
<root>\shared\inventory\ops\{client_id}\000000000001.op.json
<root>\shared\inventory\ops\{client_id}\000000000001.op.json.tmp-{pid}-{random}
<root>\shared\inventory\snapshots\snapshot-000001.json
<root>\shared\inventory\locks\merger.lock
<root>\shared\inventory\backups\manifest-YYYYMMDDTHHMMSSZ.json
```

Legacy shared SQLite paths remain migration/reference inputs only.

## First Implementation Steps

1. Add sync structs, key helpers, stable `client_id`, `device_id`, and `next_local_seq`.
2. Wrap create/update/archive/verify/delete so local projection, applied marker, and outbox op are written together.
3. Implement shared-root resolution and operation-log bootstrap.
4. Implement `sync_inventory` as push pending local ops, then pull remote ops.
5. Implement per-entry sync state, stale-operation conflict logging, and revision tracking.
6. Watch the shared operation directory and emit `inventory:shared-changed` when operation files change.
7. Keep `InventorySharedStatus` as the frontend status contract.

## Test Plan

- Bootstrap creates folders, manifest, identity, and local metadata.
- Unavailable root keeps local CRUD usable and queues outbox records.
- Push/pull syncs a created or updated entry between two clients by `entry_uuid`.
- Duplicate `op_id` is ignored.
- Duplicate `client_id + local_seq` with a different checksum is flagged as corrupt.
- Delete writes a tombstone and older upserts do not resurrect the entry.
- Newer upserts after a tombstone restore the entry.
- Older remote operations are skipped and logged once under `sync:conflict:*`.
- `.tmp`, malformed JSON, bad checksum, unknown extension, and identity-mismatched operation files are ignored/logged.

## Decisions Still Needed

- Whether `sharedDbPath` remains null, points at the legacy DB, or is repurposed to the manifest path.
- Whether an empty shared workspace should be bootstrapped by one upsert op per existing imported entry.
- Whether `ME_LAB_SHARED_ROOT` is retained permanently as the override name.
- Whether first snapshots are plain JSON or include compression/checksum immediately.
