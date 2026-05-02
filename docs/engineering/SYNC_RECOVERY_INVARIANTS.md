# Sync Recovery Invariants

Date: 2026-05-02

This note records the local recovery rules used by `backend/src/sync/recovery.rs`.

## Local Operation State

For each local outbox operation, recovery treats these records as one recoverable unit:

- outbox operation
- applied marker
- client sequence marker
- entry record for create/update/verify/archive operations
- tombstone for delete operations
- entry state
- next local sequence marker

Recovery repairs the unit when the outbox operation still wins the current entry state. If the current entry state is newer than the outbox operation, recovery repairs only the applied/client sequence markers and does not replay stale entry or tombstone data.

## Tombstones

For deleted entries, the tombstone and entry state should agree on the delete operation.

- A current tombstone deletes any still-present entry and recreates missing delete entry state.
- A newer non-deleted entry state removes a stale tombstone.
- A newer deleted entry state replaces an older tombstone with tombstone data derived from the state.

## Local Sequence

`next_local_seq` must be greater than every local sequence already present in local outbox records or applied markers for the local client. Recovery advances `next_local_seq` when it is stale.

## Snapshot Apply

Snapshot apply writes `meta:snapshot_apply_pending` before replacing local keyspaces and clears it only after the snapshot id, bootstrap marker, and replacement keyspaces are written.

Startup recovery handles this marker conservatively:

- If `last_snapshot_id` already matches the pending snapshot id, recovery clears the stale pending marker.
- Otherwise recovery reports the interrupted snapshot apply in startup/shared status. It does not guess at a repair without the verified shared snapshot manifest and file.

## Known Non-Repairable Window

If a crash leaves only a changed entry record and no outbox operation, no applied marker, no tombstone, no entry state, and no pending mutation journal, startup recovery cannot safely prove whether that entry is a new local edit, old pre-sync data, or a partially restored state.

Current behavior is to avoid guessing. A future pending-mutation journal would be needed to make that exact window fully recoverable.
