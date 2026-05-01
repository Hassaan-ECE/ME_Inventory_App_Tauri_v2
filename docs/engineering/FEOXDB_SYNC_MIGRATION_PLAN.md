# FeOxDB Shared Sync Plan

Last updated: 2026-05-01

## Current Shape

`1.0.0` is the FeOxDB-only cutover:

- each machine owns one local `inventory.feox`
- the S-drive is sync transport, not a database file
- shared sync uses operation files, snapshots, a manifest, locks, and backups
- old app-owned `.db` files are quarantined locally and are not import sources

Shared layout:

```text
S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\shared\inventory\
  manifest.json
  ops\<client-id>\000000000001.op.json
  snapshots\snapshot-*.snapshot.json
  locks\snapshot.lock
  backups\
```

## Sync Behavior

- Local edits write to FeOxDB and durable local outbox records before flush.
- A backend task publishes pending local operations to S-drive after saves.
- Sync applies the latest verified snapshot first when safe, then applies operation files newer than the snapshot watermarks.
- Snapshot application is skipped when local-only pending changes exist.
- Snapshot failures do not replace local FeOxDB data.
- Covered operation files are compacted only after a snapshot and manifest are written and verified.
- The latest three snapshots are retained.

## Conflict Behavior

Concurrent non-overlapping field edits merge when both edits started from the same base version.

Example:

- Machine A changes `location`
- Machine B changes `notes`

Result:

- both fields are kept
- no duplicate row is created
- overlapping edits still use newer-operation-wins behavior and record a stale conflict

## Release Acceptance

Before treating `1.0.0` as shipped:

- update two installed `0.9.9` machines to `1.0.0`
- confirm both preserve their local `inventory.feox`
- confirm a clean profile hydrates from `manifest.json`, snapshot, and newer ops
- confirm create/update/delete/archive/verify converges both ways
- confirm different-field concurrent edits merge
- confirm same-field concurrent edits record a conflict and keep the newer value
- confirm old local `.db` files move to `deprecated-db-backups`
- confirm the S-drive operation folder compacts after snapshot publication
