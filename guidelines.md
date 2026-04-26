# Inventory Management App Architecture Guidelines

**Stack:** Bun, Tauri 2, React 19, TypeScript, Vite, Tailwind CSS v4, lucide-react, FeOxDB
**App type:** Windows desktop inventory lookup and lightweight inventory editing
**Expected scale:** up to ~10 concurrent users, local-first, shared-drive sync

---

## 1. Core Rule

**Never let multiple clients directly mutate one shared FeOxDB file on a network drive.**

FeOxDB should be treated as a fast local embedded database. The shared drive should be used only for:

- append-only sync operation files
- periodic snapshots
- backups
- manifests written by a single compactor/merger

The shared drive is not a transactional multi-user database.

---

## 2. Recommended Architecture

```text
React UI
  -> typed Tauri commands
    -> Rust domain/services layer
      -> local FeOxDB
      -> durable outbox queue
      -> background sync engine
        -> shared-drive append-only operation log
        -> periodic snapshots
```

For up to ~10 concurrent users with modest write volume, a shared-drive append-only operation log is acceptable if the app has:

- immutable operation files
- idempotent sync
- deterministic merge rules
- single-writer compaction
- crash recovery
- conflict detection
- backups and migrations
- tests for offline and concurrent edits

A central server or API is still the better long-term architecture if user count, write volume, audit needs, or reliability requirements grow.

---

## 3. Goals

The architecture should support:

- fast local inventory lookup
- offline or degraded-network usage
- safe local writes
- reliable eventual sync
- deterministic conflict handling
- crash-safe recovery
- low-friction deployment on Windows machines
- clear audit history for inventory changes

---

## 4. Non-Goals

This design does not try to provide:

- real-time global consistency
- distributed transactions
- direct multi-writer database-file access
- guaranteed instant visibility of every edit on every machine
- server-grade concurrency from a shared drive alone

If those become requirements, use a central service with a real transactional database.

---

## 5. Data Ownership Model

Each installed app owns its local database.

```text
Local FeOxDB:
  - current inventory projection
  - local search indexes
  - durable outbox
  - sync watermarks
  - applied operation IDs
  - conflict records
  - schema metadata
```

The shared drive owns exchanged sync artifacts.

```text
Shared drive:
  - append-only operation files
  - compacted snapshots
  - manifest
  - merger lock/lease
  - backup copies
```

No client should open another client's local database. No client should mutate a shared FeOxDB database file.

---

## 6. Shared Drive Layout

```text
shared-drive/inventory/
  manifest.json

  snapshots/
    snapshot-000001.json.zst
    snapshot-000002.json.zst

  ops/
    client-01HZX.../
      000000000001.op.json
      000000000002.op.json
    client-01HZY.../
      000000000001.op.json

  locks/
    merger.lock

  backups/
    manifest-2026-04-26T120000Z.json
```

Rules:

- Each client writes only to its own folder under `ops/{client_id}/`.
- Operation files are immutable after creation.
- Operation filenames use monotonically increasing local sequence numbers.
- `manifest.json` is written only by the merger/compactor.
- Snapshots are written only by the merger/compactor.
- All writes use temp-file-then-rename.

---

## 7. Atomic File Write Pattern

Never write final operation files directly.

Use this pattern:

```text
1. Write file to:
   000000000123.op.json.tmp-{process_id}-{random}

2. Flush file contents if possible.

3. Atomically rename to:
   000000000123.op.json

4. Never modify it again.
```

Readers must ignore:

- `.tmp` files
- files with unknown extensions
- corrupt JSON
- files with invalid checksum
- files with mismatched `client_id` or `local_seq`

---

## 8. Client Identity

Each installation gets a stable `client_id`.

Store it locally:

```text
meta:client_id = "01HZX..."
```

Do not regenerate it unless the user intentionally resets the local app identity.

Each user/session should also include:

```text
user_id
device_id
app_version
schema_version
```

`client_id` identifies the app installation.
`user_id` identifies the human actor.
`device_id` can identify the Windows machine.

---

## 9. Operation Identity

Each operation must be globally unique and idempotent.

Recommended fields:

```json
{
  "op_id": "01HZZ...",
  "client_id": "01HZX...",
  "local_seq": 123,
  "schema_version": 1,
  "app_version": "0.1.0",
  "created_at_utc": "2026-04-26T17:30:00Z",
  "actor_user_id": "user-123",
  "device_id": "desktop-abc",
  "type": "inventory.adjust_quantity",
  "entity_id": "item-123",
  "base_version": "version-or-null",
  "payload": {},
  "checksum": "sha256..."
}
```

Rules:

- `op_id` is unique.
- `client_id + local_seq` is unique.
- Duplicate `op_id` must be ignored.
- Duplicate `client_id + local_seq` with different contents is corruption and must be flagged.
- Timestamps are for audit/display, not authoritative ordering.
- Merge ordering should be deterministic.

---

## 10. Inventory Data Model

Do not model stock as only:

```text
sku.quantity = 48
```

Use an immutable inventory ledger.

Quantity-affecting events should be represented as deltas:

```json
{
  "type": "inventory.adjust_quantity",
  "sku_id": "sku-123",
  "location_id": "main-stockroom",
  "delta_qty": -3,
  "reason": "usage",
  "reference": "work-order-456"
}
```

Supported operation types:

- receive stock
- use/sell stock
- transfer stock
- cycle count adjustment
- damaged/lost stock
- return to stock
- create item
- update item metadata
- rename SKU
- merge duplicate items
- tombstone item
- restore item

Current stock is derived from:

```text
snapshot quantity + ledger operations after snapshot
```

---

## 11. Quantity Rules

Quantity changes should be ledger-based whenever possible.

Rules:

- Receiving stock creates a positive delta.
- Usage/sale/removal creates a negative delta.
- Transfer creates two linked deltas:
  - negative from source location
  - positive to destination location
- Cycle counts create adjustment operations.
- Cycle counts must not blindly overwrite history.
- Negative stock should either be blocked or explicitly allowed by policy.
- Every quantity change needs a reason.
- Every quantity change must be auditable.

Example cycle count:

```text
Current derived quantity: 48
User counts: 45
Operation written: delta_qty = -3, reason = "cycle_count"
```

---

## 12. Metadata Conflict Policy

Quantity changes are usually mergeable. Metadata changes are not always mergeable.

Examples of metadata:

- item name
- description
- category
- unit of measure
- preferred vendor
- reorder point
- barcode
- SKU

Rules:

- Duplicate SKUs should be rejected or routed to manual conflict resolution.
- Item deletes should create tombstones, not hard deletes.
- Rename/merge operations should preserve aliases or redirects.
- Metadata edits should include `base_version`.
- If two users edit the same field from the same base version, mark conflict.
- If two users edit different fields, field-level merge is acceptable.
- Last-write-wins may be used only for low-risk fields and must be documented.

Recommended policy:

```text
Quantity deltas:
  merge automatically

Different metadata fields:
  field-level merge

Same metadata field:
  conflict unless field is explicitly last-write-wins

Deletes:
  tombstone wins, but conflicting later edits are retained for review

SKU/barcode uniqueness:
  strict conflict
```

---

## 13. Local FeOxDB Keyspaces

Use explicit key prefixes.

```text
inventory:item:{item_id}
inventory:sku:{sku_id}
inventory:location:{location_id}
inventory:ledger:{op_id}

inventory:index:sku:{canonical_sku}
inventory:index:barcode:{barcode}
inventory:index:search:{token}:{item_id}

sync:outbox:{local_seq}
sync:applied:{op_id}
sync:client_seq:{client_id}
sync:watermark:{source}
sync:conflict:{conflict_id}

meta:schema_version
meta:client_id
meta:last_snapshot_id
```

Keep durable sync state separate from inventory projection state.

---

## 14. FeOxDB Practices

Use FeOxDB for local performance, not shared-drive multi-user coordination.

Best practices:

- Use CAS for optimistic local updates.
- Use timestamps for conflict metadata, not global ordering.
- Use explicit `flush()` before considering critical local data durable.
- Keep operation records immutable.
- Keep projections rebuildable from snapshots and operations.
- Store applied operation IDs to guarantee idempotency.
- Validate JSON before writing to the DB.
- Avoid storing giant blobs inside hot inventory records.
- Keep search indexes separate and rebuildable.

---

## 15. Local Write Flow

When the user makes an inventory change:

```text
1. Validate command input in Rust.
2. Build domain operation.
3. Assign op_id and local_seq.
4. Store operation in local outbox.
5. Apply operation to local projection.
6. Store applied op marker.
7. Flush local DB if the operation must survive crash immediately.
8. Return success to UI.
9. Background sync writes operation to shared drive.
```

The UI should never directly mutate database files.

---

## 16. Sync Push Flow

Background sync should:

```text
1. Read unsynced local outbox entries.
2. For each operation:
   - serialize canonical JSON
   - compute checksum
   - write temp file to shared drive
   - rename atomically
3. Mark local outbox state as written_to_shared.
4. Do not delete local outbox immediately.
5. Wait until operation appears in applied snapshot or remote readback before pruning.
```

Outbox states:

```text
pending_local
writing_to_shared
written_to_shared
seen_in_snapshot
conflicted
failed_retryable
failed_permanent
```

---

## 17. Sync Pull Flow

Each client should periodically:

```text
1. Read manifest.json.
2. Download/apply latest snapshot if needed.
3. Scan ops folders for new operation files.
4. Validate operation files.
5. Ignore already applied op_id values.
6. Apply operations deterministically.
7. Update local watermarks.
8. Rebuild affected search indexes.
9. Persist sync status.
```

Clients must tolerate:

- missing files
- partially written temp files
- duplicate operations
- out-of-order operation discovery
- stale manifest
- unavailable shared drive
- permission errors

---

## 18. Manifest Rules

`manifest.json` should be small and single-writer.

Example:

```json
{
  "schema_version": 1,
  "snapshot_id": "snapshot-000002",
  "snapshot_path": "snapshots/snapshot-000002.json.zst",
  "created_at_utc": "2026-04-26T18:00:00Z",
  "created_by_client_id": "merger-01",
  "included_ops": {
    "client-01HZX": 125,
    "client-01HZY": 88
  },
  "checksum": "sha256..."
}
```

Rules:

- Only the merger/compactor writes `manifest.json`.
- Clients may read it frequently.
- Write manifest using temp-file-then-rename.
- Keep backups of previous manifests.
- Include checksum for snapshot verification.

---

## 19. Merger / Compactor

The merger builds compacted snapshots from operation files.

It may run:

- inside one chosen desktop app
- as a scheduled task on one machine
- as a small background service
- later, as a server process

Rules:

- Only one merger runs at a time.
- Use a lock or lease file.
- Detect stale locks.
- Never delete operation files until a verified snapshot includes them.
- Keep old snapshots for rollback.
- Write new snapshot before updating manifest.
- Updating manifest is the final publish step.

---

## 20. Lock / Lease Rules

Use a lock file for compaction:

```json
{
  "owner_id": "client-01HZX",
  "process_id": 1234,
  "acquired_at_utc": "2026-04-26T18:00:00Z",
  "expires_at_utc": "2026-04-26T18:05:00Z"
}
```

Rules:

- The merger periodically renews the lease.
- Other clients do not compact while lease is valid.
- If expired, another merger may take over.
- Stale lock takeover must be logged.
- Lock handling must be conservative.

---

## 21. Search Design

Search should be local and fast.

Recommended:

- normalize SKUs
- normalize barcodes
- tokenize item names/descriptions
- maintain search indexes in FeOxDB
- debounce UI search input
- support prefix search for SKU/barcode
- support fuzzy search only if needed
- cap result counts
- show exact SKU/barcode matches first

Search should handle:

- empty query
- whitespace-only query
- case differences
- punctuation differences
- barcode scanner input
- deleted/tombstoned items
- duplicate aliases
- large result sets

---

## 22. Tauri Command Boundary

Expose narrow typed commands only.

Example frontend API:

```ts
searchInventory(query)
getItem(id)
adjustQuantity(input)
createItem(input)
updateItemMetadata(input)
getSyncStatus()
startSync()
pauseSync()
resolveConflict(input)
```

Do not expose:

- raw database reads/writes
- arbitrary file access
- arbitrary shell commands
- generic SQL-like query execution
- unrestricted shared-drive path mutation

Rust must validate all IPC inputs.

---

## 23. Frontend Practices

React should be responsible for:

- presentation
- local component state
- forms
- optimistic UI states
- calling typed Tauri commands
- rendering sync/conflict state

React should not own:

- inventory business rules
- sync logic
- DB schema
- operation merge rules
- file system mutation
- permission-sensitive behavior

UI requirements:

- clear loading states
- clear empty states
- clear sync status
- clear conflict badges
- accessible buttons and forms
- keyboard-friendly lookup
- barcode scanner-friendly search
- no hidden destructive actions

---

## 24. TypeScript Practices

Use:

- `strict: true`
- no implicit `any`
- typed command wrappers
- runtime validation for IPC responses
- discriminated unions for command results
- explicit error types

Example result shape:

```ts
type CommandResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: AppError };
```

---

## 25. Rust Domain Layer

Rust should contain:

- inventory domain rules
- sync engine
- FeOxDB access layer
- migration logic
- file write/read safety
- conflict detection
- audit/event creation
- validation

Suggested module shape:

```text
src-tauri/src/
  commands/
  domain/
    inventory/
    sync/
    search/
  db/
  migrations/
  shared_drive/
  errors/
```

Keep Tauri command handlers thin. They should call services, not contain business logic.

---

## 26. Error Handling

Use user-safe error messages.

Categories:

```text
validation_error
not_found
conflict
shared_drive_unavailable
permission_denied
db_error
sync_retryable
sync_permanent
migration_required
corrupt_remote_file
```

Rules:

- Log internal detail locally.
- Show actionable messages in UI.
- Do not expose stack traces to users.
- Retry only retryable errors.
- Permanent errors require user/admin action.

---

## 27. Security

Follow least privilege.

Rules:

- Restrict Tauri capabilities.
- Do not expose raw filesystem APIs to React.
- Do not store secrets in source code.
- Validate shared-drive paths.
- Prevent path traversal.
- Treat operation files as untrusted input.
- Verify checksums.
- Reject unknown schema versions unless migration exists.
- Log suspicious or corrupt files.
- Avoid hidden network calls.

---

## 28. Migrations

Every schema change needs a migration.

Rules:

- Store local schema version.
- Backup before destructive migration.
- Make migrations idempotent if possible.
- Test migration from previous released versions.
- Never silently drop inventory ledger history.
- Keep operation schema backward compatibility when practical.
- Include app version in operations.

---

## 29. Backups

Backups should include:

- latest manifest
- recent snapshots
- operation files
- local DB backup before migration

Rules:

- Keep multiple generations.
- Verify backup checksums.
- Do not rely on a single current snapshot.
- Allow rebuild from snapshot + ops.
- Document restore procedure.

---

## 30. Observability

The app should expose sync health.

Track:

- last successful sync time
- shared drive availability
- local outbox count
- remote operations pending
- conflicts count
- corrupt remote files count
- current schema version
- local DB size
- last snapshot applied
- app version

Logs should include:

- operation creation
- sync push success/failure
- sync pull success/failure
- conflict creation/resolution
- migration start/end
- compaction start/end

---

## 31. Testing Requirements

Minimum tests:

- create item locally
- adjust quantity locally
- rebuild quantity from ledger
- duplicate operation is ignored
- out-of-order operations apply correctly
- app crash after local write
- app crash after shared write but before local status update
- corrupt `.op` file is ignored/logged
- duplicate SKU conflict
- concurrent metadata edit conflict
- transfer creates balanced deltas
- cycle count creates adjustment
- migration preserves data
- snapshot + ops rebuild matches current projection

Manual test scenarios:

- shared drive unplugged/unavailable
- permission denied on shared drive
- antivirus/file lock simulation
- two users edit same item
- two users adjust same SKU quantity
- app closed during sync
- app upgraded with pending local outbox

---

## 32. Professional Quality Bar

Before shipping meaningful changes:

- acceptance criteria are clear
- code is scoped
- local instructions are followed
- validation is run where practical
- unrun checks are reported honestly
- sync behavior is tested
- conflict behavior is documented
- migrations are tested
- user-facing errors are understandable
- no direct shared DB mutation exists

---

## 33. Strong Recommendation

For this app, the best near-term architecture is:

```text
Local FeOxDB per user
+ immutable operation outbox
+ shared-drive append-only sync
+ single-writer snapshot compaction
+ explicit conflict handling
```

This is appropriate for up to ~10 concurrent users if write volume is modest and sync delay is acceptable.

If the app grows beyond that, move the sync engine behind a central service:

```text
Local FeOxDB
  -> API/server
    -> transactional database
    -> shared snapshots or client sync feed
```

The ledger and immutable operation design should remain either way.

