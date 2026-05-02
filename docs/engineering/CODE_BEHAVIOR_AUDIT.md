# Code Behavior Audit

Date: 2026-05-01

Historical note: this audit records the source-inspection findings that started the hardening pass. Several findings have since been remediated or partially remediated. Use `docs/engineering/CODE_BEHAVIOR_REMEDIATION_CHECKLIST.md` for current status and remaining release gates.

Scope: high-risk source-code audit of the Tauri inventory app. This report is based on code inspection and commands run in this working tree. README and existing docs were not used as evidence for behavior claims.

## Executive Summary

The application has a clean separation between the React/TypeScript frontend, Tauri bridge, Rust command layer, FeOxDB storage, and shared filesystem sync. The strongest areas are sync envelope validation, checksum handling, corrupt remote-file detection, URL/path filtering, and broad test coverage around normal sync and conflict flows.

No confirmed `Critical` defect was found in this pass, and the audit should not be read as "the app is broken." The findings are hardening and professional-readiness risks: normal-path behavior is well covered, while crash recovery, concurrency, and security posture need deliberate tightening before the app should be treated as mature production software.

The main professional-readiness risks are concentrated in places where the code relies on environmental trust or multi-step state changes:

- Disabled Tauri CSP plus broad updater/custom command surface raises the impact of any renderer compromise.
- Inventory mutation, sync queueing, snapshot replacement, and shared-file compaction are multi-step flows without an explicit transaction or crash-recovery protocol visible in source.
- Backend shared-sync work can overlap through mutation-triggered background publish and manual/full sync paths.
- Shared sync checksums detect corruption but do not authenticate authorized writers to the shared root. This is acceptable only if the shared drive is intentionally treated as a trusted-write boundary enforced by Windows/share ACLs.
- Frontend runtime behavior trusts Tauri payload shapes and localStorage values more than the backend tests enforce.
- The plain `bun` command is broken on this machine because its PowerShell shim points to a missing npm-installed `bun.exe`, but the real Bun binary under `.bun\bin` works.

## Code-Observed Architecture

- App startup in `backend/src/lib.rs` registers Tauri plugins, opens `InventoryDb`, manages `SharedSyncWatcher`, and exposes inventory CRUD/sync/export/native commands through `generate_handler`.
- Local storage uses FeOxDB through `backend/src/storage/*`, with entry records, numeric-id indexes, metadata keys, outbox records, applied markers, client sequence markers, watermarks, tombstones, entry states, conflict records, and corrupt remote records.
- Inventory mutations in `backend/src/api/commands.rs` normalize and minimally validate input, update local entries, enqueue sync operations, flush, then schedule a background shared publish.
- Shared sync in `backend/src/sync/apply.rs` bootstraps local entries, pushes pending operations, applies snapshots when safe, pulls remote operation files, maybe publishes snapshots, and reports shared status.
- Operation files in `backend/src/sync/operation_file.rs` are immutable JSON envelopes with canonical SHA-256 checksums, path-segment validation, client/sequence identity checks, entity/payload validation, and duplicate-sequence detection.
- Snapshot code in `backend/src/sync/snapshot.rs` reads/writes manifest plus snapshot files, replaces local entries/tombstones/entry states/watermarks, compacts covered operation files, and prunes old snapshots.
- Frontend state is centered in `frontend/src/features/inventory/components/InventoryShell.tsx`, with desktop/mock loading, local filtering/sorting, sync polling, mutation feedback, updater UI, export, archive/delete, and entry editing.
- The desktop bridge in `frontend/src/integrations/tauri/tauriInventoryBridge.ts` attaches typed functions to `window.inventoryDesktop` using generic `invoke<T>` calls.

## Findings

### High - Tauri CSP is disabled while privileged commands are exposed

Observed behavior:
- `backend/tauri.conf.json:25` sets `"csp": null`.
- `backend/capabilities/default.json:7` grants `core:default`, and `backend/capabilities/default.json:11` grants `updater:default`.
- `backend/src/lib.rs:29-42` exposes inventory CRUD/sync, Excel export, picture preview, external URL opening, local path opening, and file picker commands.
- `frontend/src/integrations/tauri/tauriInventoryBridge.ts:59-62` exposes update check/download/install functions to the renderer bridge.

Risk:
- A renderer XSS or dependency compromise would have a larger blast radius because the webview has no CSP guardrail and can reach custom commands and updater actions.
- The Rust-side URL and picture-path checks reduce some command abuse, but they do not reduce the need for renderer hardening.

Recommendation:
- Add an explicit Tauri CSP that permits only required local assets and blocks inline/script injection where practical.
- Review `core:default` and `updater:default` against the minimum permissions the UI actually needs.
- Keep updater install behind explicit user intent and verify endpoint ownership, release signing, and private-key custody outside the codebase.

Validation status:
- Source-inspected only. Existing tests/builds do not check CSP or capability minimization.

### High - Multi-step local mutation and sync writes are not visibly transactional

Observed behavior:
- Create writes the entry, advances next id, queues a sync operation, and flushes in separate calls (`backend/src/api/commands.rs:160-179`).
- Update and delete perform similar local mutation plus sync-queue steps, with best-effort rollback on queue failure (`backend/src/api/commands.rs:222-236`, `backend/src/api/commands.rs:333-346`).
- Persisting a local sync operation writes the outbox and applied marker, then may write tombstone/entry-state/revision data (`backend/src/sync/queue.rs:207-220`).
- Snapshot apply replaces multiple local keyspaces and metadata values across separate calls (`backend/src/sync/snapshot.rs:123-125`, `backend/src/sync/snapshot.rs:194-214`).

Risk:
- A process crash, power loss, FeOxDB write failure, or unexpected panic between steps can leave entry state, metadata, outbox, applied markers, tombstones, or revision values partially advanced.
- The code has rollback paths for queue errors, but rollback itself is best-effort and not equivalent to an atomic commit.

Recommendation:
- Define explicit invariants for local mutation commits, for example: "entry state, outbox record, applied marker, tombstone, and revision are either all committed or recoverable on next startup."
- Add startup recovery checks that repair or flag mismatched entry/outbox/applied-marker state.
- Add tests that inject failures between the individual write steps, especially create, delete, snapshot apply, and local operation persistence.

Validation status:
- `cargo test` passed normal and integration flows. No crash/failure-injection test was found.

### Medium - Shared filesystem sync is robust against corruption but not authenticated

Observed behavior:
- Shared root is taken from environment/default path without authentication metadata (`backend/src/sync/shared_paths.rs:18-24`).
- Operation files validate path segment, local sequence, schema, client/folder identity, entity payload identity, and checksum (`backend/src/sync/operation_file.rs:131-184`, `backend/src/sync/operation_file.rs:338-354`).
- Operation checksums are canonical SHA-256, not keyed signatures (`backend/src/sync/operation_file.rs:52-57`, `backend/src/sync/operation_file.rs:356-361`).
- Scanning records malformed files and duplicate sequence conflicts (`backend/src/sync/scanning.rs:55-85`).

Risk:
- The code can detect accidental corruption and malformed files well.
- Any user/process with write access to the shared root can still create a structurally valid operation or snapshot.
- This is not automatically a defect for the current lab workflow if the S-drive is an intentional trusted-write boundary and Windows/share ACLs restrict writers to trusted users and machines. It becomes a security defect if untrusted users or compromised machines can write to the shared folder.

Recommendation:
- Treat the shared root as a trusted-write boundary and enforce that with filesystem ACLs.
- If the shared root can be modified by untrusted users or compromised machines, add operation/snapshot authentication such as HMAC/signature verification. For the current trusted-lab model, crypto signing is optional hardening rather than a required 1.0.x fix.
- Record the trust model in the final user-facing docs after code behavior and policy are aligned.

Validation status:
- Tests cover malformed/corrupt operation files and sync conflict flows. They do not establish malicious-writer resistance.

### Medium - Shared sync and background publish concurrency is implicit

Observed behavior:
- Mutations call `schedule_shared_publish`, which spawns a blocking task and ignores the result of `publish_pending_local_changes` (`backend/src/api/commands.rs:128-132`).
- `sync_inventory` also runs shared sync in a blocking task (`backend/src/api/commands.rs:36-44`).
- Frontend sync can be triggered by initial load, polling, delayed mutation sync, and filesystem watcher events (`frontend/src/features/inventory/components/InventoryShell.tsx:178-208`, `frontend/src/features/inventory/components/InventoryShell.tsx:343-352`).
- The frontend serializes its own sync calls with `syncInFlightRef`, but that does not serialize backend work started from mutation-triggered background publishes.

Risk:
- Concurrent backend publish/sync/mutation flows can interleave reads, FeOxDB writes, watermarks, outbox scans, operation-file writes, snapshot publish, and compaction.
- FeOxDB may be safe for concurrent access, but source-level invariants for sync interleaving are not obvious.
- Ignored publish errors delay visibility until a later status refresh.

Recommendation:
- Add a backend-level shared-sync coordinator or mutex if only one push/pull/snapshot operation should run at a time.
- If concurrent sync is intended, document the invariants in code comments near the shared entry points and add stress tests for overlapping mutation, publish, watcher event, and manual sync.
- Return or persist background publish failures in a way the UI can surface promptly.

Validation status:
- Existing integration tests are strong for sequential convergence and conflicts. No concurrent stress test was observed.

### Medium - Conflict ordering compares timestamp strings directly

Observed behavior:
- Last-write-wins uses lexicographic string comparison on `mutation_ts_utc`, with `op_id` as a tie-breaker (`backend/src/sync/conflicts.rs:78-85`).
- Disjoint-field merge also uses string comparison to pick the max timestamp (`backend/src/sync/apply.rs:296`, `backend/src/sync/apply.rs:321-327`).
- Operation-file validation checks envelope identity and checksum, but does not parse or validate `mutation_ts_utc` as RFC3339.

Risk:
- The current local timestamp producer emits RFC3339 UTC strings, which sort correctly when all writers behave.
- A malformed but checksum-valid remote operation can influence ordering unexpectedly.
- Clock skew across machines is still a product-level conflict behavior risk even with well-formed timestamps.

Recommendation:
- Validate remote operation timestamps during operation-file read.
- Store parsed timestamp values or compare with a parser instead of raw strings.
- Add tests for malformed timestamps, non-UTC offsets, old/future timestamps, and clock-skew conflict outcomes.

Validation status:
- Existing tests cover newer/older/equal timestamp conflict behavior, but not malformed timestamp rejection.

### Medium - Backend input validation is intentionally minimal

Observed behavior:
- Backend normalization trims text and coerces enum values to defaults (`backend/src/domain/model.rs:223-243`).
- Backend validation only requires at least one identity field (`backend/src/domain/model.rs:247-261`).
- Frontend form validation checks quantity is finite and requires identity (`frontend/src/features/inventory/components/entry-dialog/form.ts:52-61`, `frontend/src/features/inventory/components/entry-dialog/form.ts:152-156`).
- Picture path and link values are saved as text; native preview/open paths validate when those commands are used.

Risk:
- Tauri command callers can send negative quantities, very large quantities, very long strings, arbitrary link text, or arbitrary picture-path text to persistence as long as one identity field is present.
- Overlarge fields can affect UI rendering, export size, sync payload size, and snapshot size.

Recommendation:
- Move durable validation into the Rust command/domain layer, not only the form layer.
- Define bounds for quantity, field lengths, and accepted path/link formats if those values have operational meaning.
- Add backend tests for invalid quantity ranges, long strings, and path/link handling.

Validation status:
- Current tests cover empty identity and enum trimming/defaulting. Broader validation tests were not observed.

### Medium - Frontend trusts Tauri response shapes at runtime

Observed behavior:
- The bridge declares TypeScript types in `frontend/src/integrations/tauri/desktop-bridge.d.ts:13-50`.
- `tauriInventoryBridge.ts` uses generic `invoke<T>` calls, which provide compile-time shape assumptions but no runtime validation.
- `InventoryShell` writes `payload.entries` and mutation `result.entry` directly into React state (`frontend/src/features/inventory/components/InventoryShell.tsx:140-147`, `frontend/src/features/inventory/components/InventoryShell.tsx:471-528`).

Risk:
- Backend/frontend contract drift, corrupted local data, or unexpected migration output can turn into render errors or silent state corruption in the UI.
- The risk is lower than an external API because the backend is local and strongly typed, but Tauri serialization boundaries still deserve defensive normalization.

Recommendation:
- Add small runtime guards/normalizers at the bridge boundary for entry arrays, shared status, mutation results, and update states.
- Add tests with malformed bridge responses to ensure the shell fails closed with a clear status message.

Validation status:
- Frontend tests cover many expected bridge behaviors. Malformed bridge payload tests were not observed.

### Medium - Query/filter paths load and process all entries in memory

Observed behavior:
- Backend `query_inventory_from_store` loads all entries, counts, filters, sorts, and pages in memory (`backend/src/api/commands.rs:135-150`).
- `InventoryDb::load_entries` scans all entries then sorts them (`backend/src/storage/entries.rs:20-28`).
- Frontend also filters and sorts the current entry array locally (`frontend/src/features/inventory/components/InventoryShell.tsx:364-367`).
- Performance baselines exist but are opt-in/ignored by default.

Risk:
- This is simple and likely fine for modest inventory sizes.
- Large inventories or large shared snapshots can make load, search, sort, export, and sync visibly slow or memory-heavy. This is not urgent until expected inventory size or benchmark data shows pressure.

Recommendation:
- Define expected maximum inventory size.
- Run the backend and frontend performance baselines before release.
- If the expected size is high, move paging/filtering/sorting closer to indexed storage or add incremental query paths.

Validation status:
- Normal tests and build passed. Performance baseline tests were intentionally not run in this audit pass.

### Low - Stored column visibility can create an unusable table state

Observed behavior:
- Stored visibility is merged over defaults without repair (`frontend/src/features/inventory/lib/columns.ts:11-15`).
- Visible columns are computed by filtering only truthy stored values (`frontend/src/features/inventory/lib/columns.ts:17-18`).
- The UI prevents toggling off the last visible data column during normal interaction, but `readColumnVisibility` accepts stored JSON as-is after parsing (`frontend/src/features/inventory/components/shell/helpers.ts:197-210`).

Risk:
- A bad or stale `localStorage` value can hide all data columns even though the in-app menu tries to prevent that state.

Recommendation:
- Repair stored visibility on read so at least one data column remains visible.
- Add a test for corrupted/no-column localStorage recovery.

Validation status:
- Existing tests cover the UI prevention path, not corrupted stored values.

### Low - Picture preview validation is extension and size based, not content based

Observed behavior:
- Picture paths must be absolute local paths with allowed image extensions (`backend/src/integrations/native.rs:148-170`).
- Preview generation rejects missing files and files larger than 50 MB, then copies the source into app cache (`backend/src/integrations/native.rs:188-204`, `backend/src/integrations/native.rs:235-238`).
- Frontend converts the cached path through `convertFileSrc` (`frontend/src/integrations/tauri/tauriInventoryBridge.ts:48-50`).

Risk:
- A file with an image extension but non-image content can be copied into app cache and exposed through the app-cache asset protocol.
- UNC paths and filesystem reparse behavior are accepted if they look like absolute local paths and pass `is_file`.

Recommendation:
- Validate image magic bytes or decode metadata before caching previews.
- Consider canonicalization/reparse handling and whether UNC paths should be allowed for previews.
- Keep the app-cache asset scope narrow.

Validation status:
- Tests cover extension, missing-file, and size behavior. Content-sniffing and reparse/UNC behavior were not covered.

### Info - Excel export appears to use string cells for user-controlled text

Observed behavior:
- Quantity is written as a number; other inventory fields are written with `write_string_with_format` (`backend/src/integrations/export/workbook.rs:141-165`).
- Formula-specific writer APIs were not observed in the export path.

Risk:
- Using string cells is a good sign for formula-injection resistance.
- There is no explicit regression test for user values beginning with `=`, `+`, `-`, or `@`.

Recommendation:
- Add an export test that opens the generated XLSX XML and confirms formula-like user values are stored as strings, not formulas.

Validation status:
- Export tests passed through `cargo test`, but no formula-like input test was observed.

### Info - Tooling coverage has environment and configuration gaps

Observed behavior:
- `bun run lint`, `bun run test`, and `bun run build` all emitted a PowerShell shim error pointing at missing `C:\Users\Syed.H.Shah\AppData\Roaming\npm\node_modules\bun\bin\bun.exe`.
- The real Bun binary exists at `C:\Users\Syed.H.Shah\.bun\bin\bun.exe` and reports version `1.3.13`.
- Direct execution through `C:\Users\Syed.H.Shah\.bun\bin\bun.exe` passed lint, frontend tests, and frontend build.
- `npm run lint`, `npm run test`, and `npm run build` passed.
- `cargo clippy --all-targets -- -D warnings` could not run because `cargo-clippy.exe` is not installed for `stable-x86_64-pc-windows-msvc`.
- `tsconfig.json` references frontend app/node configs, while `frontend/tsconfig.app.json` includes only `src`; frontend tests run under Vitest but are not part of `tsc -b`.
- ESLint uses recommended non-type-aware TypeScript linting.
- No dependency audit script was observed for npm/Bun or Cargo.

Risk:
- The app validates cleanly through npm, direct Bun, and Cargo tests. The remaining issue is local command resolution: the plain `bun` shim is broken in this environment.
- Clippy cannot currently serve as a release gate on this machine.
- Test files can drift from strict TypeScript checking if Vitest transpilation accepts code that `tsc` never checks.

Recommendation:
- Fix or remove the broken Bun shim, call the direct Bun binary in local release scripts, or standardize scripts/docs on npm.
- Install Clippy with `rustup component add clippy`.
- Add type-check coverage for frontend tests if they are intended to be first-class TypeScript code.
- Add dependency audit commands or CI steps for frontend and Rust dependencies.

Validation status:
- See validation results below.

## Positive Controls Observed

- Operation files use temp-file plus rename writes and refuse conflicting existing operation content (`backend/src/sync/operation_file.rs:46-78`).
- Operation scanner records malformed filenames, corrupt JSON/envelopes, watermarked files, temp files, and duplicate sequence conflicts (`backend/src/sync/scanning.rs:16-85`).
- Native external URL handling rejects local Windows paths and only allows `http`, `https`, and `mailto` schemes (`backend/src/integrations/native.rs:87-99`).
- Frontend saved-link handling also restricts URL protocols before opening (`frontend/src/shared/lib/externalUrl.ts:1-17`).
- Frontend sync state uses mounted/request/in-flight guards to avoid common stale async updates (`frontend/src/features/inventory/components/InventoryShell.tsx:94-104`, `frontend/src/features/inventory/components/InventoryShell.tsx:178-208`).
- Backend and frontend tests are meaningfully broad for normal inventory behavior, URL/path safety, bridge behavior, sync convergence, and conflict resolution.

## Validation Results

| Command | Result | Notes |
| --- | --- | --- |
| `git status --short` | Passed | Clean before adding this audit report. |
| `bun run lint` | Did not execute target | PowerShell shim emitted missing `bun.exe` error. Process reported exit code 0, but ESLint did not run. |
| `bun run test` | Did not execute target | Same missing `bun.exe` shim error. Vitest did not run through Bun. |
| `bun run build` | Did not execute target | Same missing `bun.exe` shim error. Build did not run through Bun. |
| `C:\Users\Syed.H.Shah\.bun\bin\bun.exe run lint` | Passed | Direct Bun binary ran `eslint .` successfully. |
| `C:\Users\Syed.H.Shah\.bun\bin\bun.exe run test` | Passed | Direct Bun binary ran Vitest: 8 files passed, 1 skipped; 65 tests passed, 1 skipped. |
| `C:\Users\Syed.H.Shah\.bun\bin\bun.exe run build` | Passed | Direct Bun binary ran `tsc -b` and Vite production build successfully. |
| `npm run lint` | Passed | Fallback validation: `eslint .` completed successfully. |
| `npm run test` | Passed | Fallback validation: 8 frontend test files passed, 1 skipped; 65 tests passed, 1 skipped. |
| `npm run build` | Passed | Fallback validation: `tsc -b` and Vite production build completed successfully. |
| `cd backend; cargo fmt -- --check` | Passed | No formatting changes required. |
| `cd backend; cargo test` | Passed | Backend unit/integration suites passed; ignored performance baseline stayed ignored. |
| `cd backend; cargo clippy --all-targets -- -D warnings` | Blocked | Clippy component is not installed for the active stable MSVC toolchain. |
| `powershell -File scripts\smoke-sync-one-machine.ps1` | Blocked | Local PowerShell execution policy blocks script execution. |
| `powershell -ExecutionPolicy Bypass -File scripts\smoke-sync-one-machine.ps1` | Passed | Process-local fallback: one-machine sync smoke passed; clients converged and stale update logged as conflict. |

## Prioritized Response Plan

The audit supports a staged hardening response rather than an emergency rewrite.

### 1.0.1 hardening

1. Add a backend shared-sync mutex/coordinator so background publish and manual/full sync cannot overlap unsafely.
2. Validate and parse remote operation timestamps as RFC3339 before conflict ordering.
3. Repair bad stored column visibility on read.
4. Add durable backend input bounds for quantity and field sizes.
5. Add an Excel export formula-injection regression test for values beginning with `=`, `+`, `-`, and `@`.
6. Add image magic-byte checks before preview caching.

### 1.0.2 reliability

1. Add startup recovery/invariant checks for partial local mutation, outbox, applied-marker, tombstone, revision, and snapshot state.
2. Add failure-injection tests around create, update, delete, local operation persistence, and snapshot apply.
3. Surface background publish errors in shared status instead of dropping them until a later sync/status refresh.

### Security and tooling pass

1. Add explicit Tauri CSP and tighten capabilities.
2. Verify S-drive ACLs and document the shared-root trust model.
3. Fix the broken plain `bun` shim, use the direct Bun binary where needed, or standardize local scripts/docs on npm.
4. Install/run Clippy and add dependency audit commands for frontend and Rust dependencies.

## Original Audit Remediation Order

1. Add CSP and review Tauri capabilities/updater exposure.
2. Define and test crash-recovery invariants for local mutation plus sync metadata writes.
3. Serialize or explicitly validate concurrent backend sync/publish behavior.
4. Validate remote operation timestamps and document clock-skew conflict behavior.
5. Strengthen backend input validation for quantities, field sizes, and durable path/link expectations.
6. Add bridge-boundary runtime guards and corrupted localStorage recovery.
7. Fix validation tooling gaps: Bun shim, Clippy component, dependency audits, and test type-checking coverage.

## Assumptions and Non-Goals

- This audit did not compare code behavior against README or existing docs.
- This audit did not modify runtime code, configs, schemas, tests, or public APIs.
- Performance baselines were not run by default because the frontend baseline writes `.tmp/performance-baseline-frontend.json` and the backend baseline is ignored unless explicitly requested.
- No external security review of release signing keys, GitHub release ownership, installer signing, or shared-drive ACLs was performed.
