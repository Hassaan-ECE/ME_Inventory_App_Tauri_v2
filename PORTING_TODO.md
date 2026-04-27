# ME Inventory Tauri Port TODO

Last updated: 2026-04-26

This checklist tracks what still needs to be implemented or decided in the Tauri v2 port to reach parity with the original Electron app at `D:\Projects\Active\ME_Inventory_App`.

Architecture guide:

- `guidelines.md`

Current active repo:

- `D:\Projects\Active\ME_Inventory_App_Tauri_v2`

Original behavior reference:

- `D:\Projects\Active\ME_Inventory_App`

Do not copy Electron code directly into the Tauri app. Use it as a behavior reference, then implement native work in Rust/Tauri commands and keep React free of Node/Electron assumptions.

## Current Baseline

Already implemented in the current Tauri port:

- [x] Vite + React 19 + TypeScript UI
- [x] Tauri 2 shell and NSIS target
- [x] Inventory and archive views
- [x] Search, filters, sorting, column visibility, Color Rows, and theme persistence
- [x] Add, edit, verify, archive, restore, and delete entry flows
- [x] Right-click context menu
- [x] Full entry dialog
- [x] FeOxDB local runtime store
- [x] First-run import from current SQLite `entries` schema
- [x] First-run import from legacy SQLite `equipment` schema
- [x] Basic Tauri bridge for inventory CRUD and query commands
- [x] Native bridge for safe external URL opening, local image path opening, and picture path picking
- [x] Native Excel export for current Tauri entry fields
- [x] Safe Tauri updater command scaffold without fake endpoints/signing
- [x] Automated current-user NSIS installer smoke for install, launch, bundled resources, first-run import, app data path, and uninstall/reinstall preservation
- [x] TypeScript strict mode is enabled
- [x] Frontend tests, lint, frontend build, Rust format, Rust check, and Rust tests pass

Known current stubs or missing native bridge methods:

- [x] `openExternal`
- [x] `openPath`
- [x] `pickPicturePath`
- [x] `exportExcel`
- [x] scaffolded `checkForUpdate`
- [x] scaffolded `downloadUpdate`
- [x] scaffolded `installUpdate`
- [x] real `onSharedInventoryChanged`
- [ ] real `onUpdateStateChanged`

Remaining non-scaffolded native systems:

- [ ] real Tauri updater signing, endpoint configuration, release hosting, download, and install flow
- [x] shared workspace sync operation log foundation

## Priority 0 - Early Decisions Before More Work

### 1. Implement The Shared Data Architecture From `guidelines.md`

Original Electron reference:

- `electron/inventory-runtime.mjs`
- `electron/inventory-db.mjs`
- `src/test/inventory-db.test.ts`

Current Tauri state:

- Local runtime database is FeOxDB.
- SQLite files are treated as one-time import sources.
- Shared sync now uses a local FeOxDB outbox plus append-only shared operation files under `<shared root>\shared\inventory\ops\{client_id}`.
- Snapshot publishing, manifest compaction, conflict UI, and shared media storage are still deferred.

Architecture decision:

- [x] Do not directly mutate one shared FeOxDB file on a network drive.
- [x] Use local FeOxDB per installation.
- [x] Use durable local outbox records.
- [x] Use shared-drive append-only operation files.
- [x] Use periodic snapshots, backups, and a single-writer merger/compactor.

Recommended direction:

- Follow `guidelines.md`.
- Use `docs/sync-architecture-next-slice.md` as the first compatibility implementation slice.
- Keep the original Electron shared SQLite code as a behavior and migration reference only.
- Prefer an operation-ledger model for quantity changes and deterministic conflict handling.

Acceptance criteria:

- [x] We know whether `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME` must remain the default shared workspace.
- [x] We know whether `ME_LAB_SHARED_ROOT` must remain supported.
- [x] We know whether installed Electron users must interoperate with new Tauri users during a transition period.
- [x] We document the chosen architecture in `README.md`.
- [ ] Shared sync implementation uses operation files, snapshots, manifest, and single-writer compaction rather than shared FeOxDB mutation.

### 2. Decide App Identifier And Data Directory Stability

Original Electron app id:

- `com.syedhassaan.me-inventory`

Current Tauri identifier:

- `com.me.inventory`

Why this matters:

- Tauri app identifier affects install identity and app data paths.
- Changing it after users install the Tauri build can strand local data in a different app data directory.

Tasks:

- [ ] Decide whether to change Tauri `identifier` to match the original app id.
- [ ] Decide whether Tauri should import from an existing Electron `userData` database on first run.
- [ ] Document the migration path before creating real installers for users.

Reference files:

- `src-tauri/tauri.conf.json`
- Original `package.json` build metadata
- Original `electron/inventory-runtime.mjs`

Acceptance criteria:

- [x] Product name, version, identifier, icon, and installer behavior are intentional and documented.
- [ ] Fresh install and upgrade-from-Electron scenarios are tested.

## Priority 1 - Native Feature Parity

### 3. Implement Safe Native External URL Opening

Original Electron reference:

- `electron/main.mjs`
- `shared/external-url.mjs`
- `src/lib/externalUrl.ts`

Current Tauri state:

- React validates URLs before calling native code.
- Rust validates URLs again before invoking OS opener APIs.
- The Tauri bridge exposes `openExternal`.

Tasks:

- [x] Add a Tauri command such as `open_external`.
- [x] Wire `window.inventoryDesktop.openExternal` in `src/lib/tauriInventoryBridge.ts`.
- [x] Reuse or mirror the current safe URL rules: allow `http:`, `https:`, and `mailto:` only.
- [x] Reject Windows paths and unsafe protocols before invoking native opener APIs.
- [x] Return `true` or `false` consistently to match the current bridge contract.

Likely edit areas:

- `src-tauri/src/lib.rs`
- `src-tauri/src/native.rs`
- `src/lib/tauriInventoryBridge.ts`
- `src/types/desktop-bridge.d.ts`
- `src/test/external-url.test.ts`
- `src/test/inventory-entry-actions.test.tsx`

Acceptance criteria:

- [x] "Open Saved Link" opens a safe saved link in the default browser from the Tauri desktop app.
- [x] "Search Online" opens a safe Google search URL from the Tauri desktop app.
- [x] Unsafe protocols such as `file:`, `javascript:`, and arbitrary shell targets are rejected.
- [ ] Unit tests cover accepted and rejected URL cases.

### 4. Implement Native Open Path For Pictures

Original Electron reference:

- `electron/main.mjs` handler `inventory:open-path`
- `src/components/inventory/EntryDialog.tsx`

Current Tauri state:

- The UI can render file URL previews.
- The UI loads local picture previews through a native validated cache-backed asset command in packaged desktop builds.
- Double-click/open picture can call `openPath` when the selected picture path is local.

Tasks:

- [x] Add a Tauri command such as `open_path`.
- [x] Add a Tauri command such as `load_picture_preview` for validated local preview cache paths.
- [x] Validate that the input is a local filesystem path.
- [x] Trim whitespace and reject empty paths.
- [x] Reject obvious URL values here and route URLs through `openExternal`.
- [x] Check whether the path exists before opening and return `false`.
- [x] Wire `window.inventoryDesktop.openPath`.

Acceptance criteria:

- [x] Double-clicking a valid local picture preview opens it with the default Windows handler.
- [x] Missing picture paths show the existing "Picture not found" state and do not crash.
- [x] Invalid paths do not execute shell commands.
- [x] Tests cover valid path, missing path, oversized image, and unsafe input behavior.

### 5. Implement Native Picture Picker

Original Electron reference:

- `electron/main.mjs` handler `inventory:pick-picture-path`
- `src/test/entry-dialog.test.tsx`

Current Tauri state:

- "Browse" calls a Tauri dialog command when running in the desktop app.

Tasks:

- [x] Add a Tauri dialog/file-picker command such as `pick_picture_path`.
- [x] Filter common image extensions: `png`, `jpg`, `jpeg`, `webp`, `gif`, `bmp`, `tif`, `tiff`.
- [x] Return `string | null` through the bridge.
- [x] Preserve the selected absolute path in the entry `picturePath`.
- [x] Confirm paths with spaces can be selected and saved in a packaged desktop smoke test.
- [ ] Confirm UNC paths behave correctly in a packaged desktop smoke test.
- [ ] Implement future shared media storage so selected pictures are copied into the shared inventory media folder and entries store the shared/relative path rather than a user-local path.

Acceptance criteria:

- [ ] Browse opens a native file picker in the Tauri app.
- [ ] Cancel returns `null` and leaves the form unchanged.
- [x] Selecting an image updates the picture path and preview in packaged desktop smoke after the native preview fix.
- [x] Tests cover bridge wiring and UI behavior.

### 6. Implement Native Excel Export

Original Electron reference:

- `electron/inventory-export.mjs`
- `electron/inventory-export-worker.mjs`
- `src/test/inventory-export.test.ts`

Current Tauri state:

- `Export > Excel` exists in the UI.
- The bridge has an optional `exportExcel` type.
- Tauri bridge implements `exportExcel` through the Rust `export_excel` command.
- The first export uses the current Tauri `InventoryEntry` fields only.

Tasks:

- [x] Choose a lightweight Rust XLSX library and confirm it supports styles, autofilter, frozen rows, and print setup.
- [x] Add a save-file dialog with default filename `ME_Inventory_Export.xlsx`.
- [x] Export all entries, not just currently filtered rows.
- [x] Split active and archived entries into two sheets: `Inventory` and `Archive`.
- [x] Preserve print-friendly styling:
  - [x] styled header row
  - [x] zebra striping
  - [x] borders
  - [x] frozen top row
  - [x] autofilter
  - [x] landscape print setup
  - [x] lifecycle/status fills where practical
- [x] Return `{ canceled, outputPath, error }` through `window.inventoryDesktop.exportExcel`.

Acceptance criteria:

- [x] Desktop `Export > Excel` opens a native save dialog in a packaged/desktop smoke test.
- [x] Cancel is silent.
- [x] Saved workbook opens in Excel in a manual smoke test.
- [x] Packaged smoke confirms workbook has exactly `Inventory` and `Archive` sheets.
- [x] Automated tests validate two-sheet workbook structure and active/archive row placement.

### 7. Implement Shared Workspace Sync

Original Electron reference:

- `electron/inventory-runtime.mjs`
- `electron/inventory-db.mjs`
- `src/test/inventory-db.test.ts`
- `README.md` "Shared Workspace"

Original behavior to preserve if shared sync remains in scope:

- Default shared root: `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME`
- Override env var: `ME_LAB_SHARED_ROOT`
- Shared DB path: `<shared root>\shared\me_inventory_shared.db`
- Legacy shared DB path: `<shared root>\shared\me_lab_shared.db`
- Local-first mutations
- Offline mutation queue
- Idempotent operations by `op_id`
- Tombstones for deletes
- Last-write-wins conflict handling by mutation timestamp and operation id
- Conflict details logged in `sync_conflicts`
- UI state through `InventorySharedStatus`

Current Tauri state:

- `sync_inventory` bootstraps existing entries once, pushes pending local outbox operations, pulls remote operation files, applies tombstones, and returns changed entries only when the local projection changed.
- FeOxDB stores per-entry sync state under `sync:entry_state:{entry_uuid}`, a monotonic `meta:sync_revision`, and stale-operation conflict records under `sync:conflict:{conflict_id}`.
- Last-write-wins parity is implemented with `(mutation_ts_utc, op_id)` ordering. Older operations are skipped and logged. Newer deletes tombstone entries. Newer upserts after a tombstone restore entries.
- `InventorySharedStatus.enabled` is true for the shared-sync foundation and reports available/unavailable root state, pending local changes, root path, local/shared mutation mode, and revision.
- `onSharedInventoryChanged` listens to the Tauri `inventory:shared-changed` event from the native shared-ops watcher and cleans up the listener.

Tasks:

- [x] Finalize architecture from Priority 0.
- [x] Map the `guidelines.md` operation schema onto current `InventoryEntry` fields.
- [ ] Decide whether current entries become `inventory:item:*` records immediately or through a compatibility layer.
- [x] Add stable `client_id`, `device_id`, app version, schema version, and local sequence metadata.
- [x] Add durable local outbox keyspace.
- [x] Define Rust data structures for sync operations and tombstones.
- [x] Define Rust data structures for revision state and logged stale-operation conflict records.
- [x] Implement shared root resolution.
- [x] Implement shared operation-log bootstrap.
- [x] Implement push pending local operations.
- [x] Implement pull missing shared operations.
- [x] Implement delete tombstones.
- [x] Implement conflict detection and logging.
- [x] Implement revision tracking.
- [x] Implement local-only mutation state and `hasLocalOnlyChanges`.
- [ ] Implement manifest reading and validation.
- [ ] Implement snapshot reading and validation.
- [x] Implement checksum validation for operation files.
- [ ] Implement checksum validation for snapshots.
- [x] Implement temp-file-then-rename writes for operation files.
- [ ] Implement temp-file-then-rename writes for snapshots and manifests.
- [x] Ignore `.tmp`, corrupt, unknown-extension, checksum-invalid, and identity-mismatched operation files.
- [x] Emit a frontend event when shared inventory changes.
- [x] Avoid reloading/repainting when data did not change.
- [x] Add clear shared sync status messages.

Acceptance criteria:

- [x] When shared workspace is unavailable, add/edit/delete/verify/archive still work locally.
- [x] UI reports local-only pending state.
- [x] When shared workspace reconnects, pending local operations are pushed.
- [x] Changes made by another client are pulled.
- [x] Older/equal upserts do not resurrect deletes because tombstones are applied.
- [x] Decide and implement explicit restore/newer-upsert policy after tombstone conflicts.
- [x] Repeated syncs are idempotent.
- [ ] Busy or locked shared files do not corrupt data.
- [x] Tests cover bootstrap, unavailable shared root, push, pull, conflicts, delete tombstones, and reconnect behavior.

Implemented foundation coverage now includes bootstrap, unavailable roots, push/pull between two FeOxDB instances, repeated sync idempotency, corrupt operation-file handling, last-write-wins stale-operation logging, equal-timestamp operation-id tie-breaking, delete tombstones, newer restore after delete, revision increments, watcher setup, frontend coalesced sync triggers, and the `scripts\smoke-sync-one-machine.ps1` two-client smoke. Conflict UI/resolution, snapshots, manifest compaction, and locked-file smoke remain open.

### 8. Implement Shared Drive Updater

Original Electron reference:

- `electron/updater.mjs`
- `electron/update-download-worker.mjs`
- `src/test/updater.test.ts`
- `README.md` "Shared Updates"

Original behavior:

- Check manifest at `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\current.json`.
- Manifest advertises latest version, installer path, and SHA-256.
- Missing or invalid SHA-256 is rejected.
- Download happens in the background to a local app-data cache.
- Downloaded installer hash is verified.
- UI changes from `Update available` to `Install update`.
- Install opens the visible installer without closing the app first.

Current Tauri state:

- `checkForUpdate`, `downloadUpdate`, and `installUpdate` are safe Tauri updater scaffolds.
- No real updater key, endpoint, artifacts, download, or install flow is configured.
- `onUpdateStateChanged` is a no-op.

Tasks:

- [x] Decide whether to keep custom shared-drive updater or use a Tauri updater flow.
- [ ] Implement version comparison in Rust or TypeScript with tests.
- [ ] Configure real Tauri updater public key, signed artifacts, and HTTPS/static or dynamic endpoint.
- [ ] Parse and validate real Tauri updater metadata.
- [ ] Download signed Tauri updater artifact.
- [ ] Verify signed updater artifact through Tauri updater.
- [ ] Emit update-state events to the UI.
- [ ] Install through Tauri updater.
- [ ] Handle errors without breaking inventory usage.
- [x] Return safe not-configured states without fake availability.

Acceptance criteria:

- [x] No update available state is quiet.
- [ ] Newer manifest shows `Update available`.
- [ ] Download progress is represented in `UpdateState`.
- [ ] Bad updater metadata/signature is rejected with a clear error.
- [ ] Signature mismatch is rejected.
- [ ] Ready installer changes action to install.
- [ ] Install uses Tauri updater and keeps app behavior predictable.
- [x] Scaffold tests verify safe not-configured states.
- [ ] Tests port the important real updater cases after signing/endpoint config exists.

## Priority 2 - Data Migration And Storage Hardening

### 9. Port Robust Legacy SQLite Schema Migration

Original Electron reference:

- `electron/inventory-runtime.mjs`
- `src/test/inventory-db.test.ts`

Current Tauri state:

- Rust detects and imports both the current `entries` schema and old `equipment` / `record_*` schema.
- Import issues are not tracked.

Tasks:

- [x] Detect current `entries` schema.
- [x] Detect legacy `equipment` schema.
- [x] Map old names such as `record_id` and `record_uuid` into `entry_id` and `entry_uuid`.
- [x] Preserve compatibility with `data\me_lab_inventory.db`.
- [x] Preserve compatibility with `data\me_inventory.db`.
- [ ] Decide whether to rewrite/migrate SQLite copies or only read from them.
- [ ] Add import issue tracking for a future import review/reporting surface if needed.
- [ ] Add clear errors for unknown schema versions.

Acceptance criteria:

- [ ] First run imports from current SQLite seed.
- [ ] First run imports from old legacy SQLite seed.
- [ ] Existing FeOxDB data is not overwritten by a later import.
- [ ] Bad SQLite paths fail safely.
- [ ] Rust tests cover current schema, legacy schema, missing DB, and unknown schema.

### 10. Revisit FeOxDB Query Performance

Original Electron reference:

- SQLite query path in `electron/inventory-db.mjs`
- Search index and triggers in `electron/inventory-runtime.mjs`

Current Tauri state:

- `query_inventory` range-scans FeOxDB entries, filters/sorts in memory, and returns up to `100_000` rows.
- React table renders returned rows directly.

Tasks:

- [ ] Benchmark startup, query, sort, and table render time using real inventory size.
- [ ] Decide acceptable max row count before virtualization or pagination is required.
- [ ] Consider secondary indexes or cached normalized search text in FeOxDB.
- [ ] Consider server-side pagination instead of returning up to `100_000` rows.
- [ ] Consider table virtualization if the UI must display very large result sets.
- [ ] Ensure background sync or import does not block the UI thread.

Acceptance criteria:

- [ ] Document target inventory size and performance budget.
- [ ] Searching and sorting remain responsive on the real dataset.
- [ ] Initial load does not freeze the UI.
- [ ] Performance tests or repeatable measurement scripts exist for future regressions.

### 11. Persist Import And Mutation Metadata Cleanly

Current Tauri state:

- FeOxDB stores entry records and simple metadata for next ID and legacy import path.

Tasks:

- [ ] Define all metadata keys in one module/section.
- [ ] Store schema version for FeOxDB data.
- [ ] Store import source and import timestamp.
- [ ] Store shared sync metadata if shared sync is implemented.
- [ ] Store app/client identity if operation sync is implemented.
- [ ] Add a migration path for future FeOxDB schema changes.

Acceptance criteria:

- [ ] New data stores can be versioned.
- [ ] Future migrations can run once and be audited.
- [ ] Metadata keys cannot collide with entry keys.

## Priority 3 - UI And Workflow Parity

### 12. Confirm Inventory Shell Parity Against Electron

Original reference:

- `src/components/inventory/InventoryShell.tsx` in both repos
- Original `README.md` "App Behavior"

Tasks:

- [ ] Compare all visible controls against the original app.
- [ ] Confirm result labels match original copy.
- [ ] Confirm archive/inventory scope behavior.
- [ ] Confirm filtered empty states.
- [ ] Confirm column visibility minimum-one-data-column rule.
- [ ] Confirm Color Rows selected styling and row coloring.
- [ ] Confirm theme persistence.
- [ ] Confirm update button behavior after updater is ported.
- [ ] Confirm export menu behavior after Excel export is ported.

Acceptance criteria:

- [ ] UI behavior matches the original where parity is intended.
- [ ] Any intentional UX changes are documented.
- [ ] Browser and Tauri desktop smoke tests cover the main workflow.

### 13. Confirm Entry Dialog Parity

Original reference:

- `src/components/inventory/EntryDialog.tsx`
- Original `README.md` "Full Entry Dialog"

Tasks:

- [ ] Confirm all original editable fields are present.
- [ ] Confirm add/edit validation matches original.
- [ ] Confirm dark-mode select/dropdown readability.
- [ ] Confirm sidebar metadata layout at large viewport.
- [ ] Confirm footer actions remain reachable at default window size.
- [ ] Confirm picture picker preview/open after native preview command is added.
- [ ] Confirm URL picture paths and local picture paths behave predictably.

Acceptance criteria:

- [ ] Add entry works for minimum valid identity.
- [ ] Edit entry preserves unchanged fields.
- [ ] Quantity validation handles empty, integer, decimal, and invalid input.
- [ ] Picture picker/open-path flows work in desktop build.

### 14. Confirm Context Menu Parity

Original reference:

- `src/components/inventory/EntryContextMenu.tsx`

Tasks:

- [ ] Confirm right-click positioning near viewport edges.
- [ ] Confirm `Open Full Entry`.
- [x] Confirm `Open Saved Link`.
- [x] Confirm `Search Online`.
- [ ] Confirm `Archive Entry` / `Restore Entry`.
- [ ] Confirm `Delete Entry` opens confirmation dialog.

Acceptance criteria:

- [ ] Keyboard/mouse behavior remains accessible.
- [x] Unsafe links do not open.
- [ ] Status strip messages match expected outcomes.

## Priority 4 - Packaging, Installer, And Release Flow

### 15. Build And Verify Tauri NSIS Installer

Current Tauri state:

- `bun tauri build --bundles nsis` is configured and validated on Windows.
- Packaging assumptions, smoke flow, and current smoke evidence are documented in `docs/desktop-smoke-checklist.md`.
- Automated installer smoke on 2026-04-26 used `src-tauri\target\release\bundle\nsis\ME Inventory_0.9.7_x64-setup.exe` and is logged in `docs/installer-smoke-2026-04-26-worker-a.md`.
- Final manager QA after reviewer fixes rebuilt the NSIS installer, reinstalled it silently, relaunched it against clean app data, confirmed a 146-entry first-run import, and restored the previous app data. The final log is `docs/release-qa-2026-04-26-manager.md`.
- Silent current-user install, installed launch, product/version metadata, bundled SQLite resource hashes, first-run import of 146 seed rows, app data path behavior, and uninstall/reinstall app-data preservation passed.
- Syed manual installed-app smoke on 2026-04-26 validated the visible installer flow, finish-page desktop shortcut creation, delta icon, installed desktop launch, version `0.9.7`, visible imported rows, stable row count after reopen, search, create/edit/verify/archive/restore/delete persistence, Search Online, Excel cancel/save/open, active plus archived export data, and quiet updater scaffold. The log is `docs/manual-smoke-2026-04-26-syed.md`.
- GUI-only checks remain open for SmartScreen behavior and optional unsafe-link manual testing. Post-fix `Open Saved Link`, picture open/missing-path behavior, Excel two-sheet shape, local-only sync status, and updater quiet state passed manual smoke.

Tasks:

- [x] Run full Tauri bundle build.
- [x] Verify installer output path.
- [x] Verify app icon visually in the installed shortcut/window.
- [x] Verify bundled `data\me_inventory.db` and `data\me_lab_inventory.db` resources are available for first-run import.
- [x] Verify app launches after install.
- [x] Verify app data directory behavior.
- [x] Verify uninstall/reinstall does not unexpectedly erase user data.

Acceptance criteria:

- [x] Installer builds successfully on Windows.
- [x] Fresh install imports seed inventory once.
- [x] Reopening app keeps prior GUI-created changes.
- [x] Installed app has correct product name, icon, and version.

### 16. Define Release And Update Publishing Procedure

Original Electron reference:

- Release folder output under `release/`
- Shared update manifest `current.json`

Tasks:

- [ ] Decide Tauri release output location.
- [ ] Decide whether release artifacts are copied to the shared drive.
- [ ] Define manifest generation for the updater.
- [ ] Define SHA-256 calculation step.
- [ ] Define version bump procedure for `package.json`, `src/branding.ts`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
- [ ] Document release checklist.

Acceptance criteria:

- [ ] A maintainer can build, publish, and verify an update using documented steps.
- [ ] Version values stay in sync.
- [ ] Update manifest points to a real installer and valid hash.

## Priority 5 - Testing And QA Coverage

### 17. Port Missing Automated Tests

Original tests not currently present in Tauri port:

- `src/test/inventory-db.test.ts`
- `src/test/inventory-export.test.ts`
- `src/test/updater.test.ts`

Tasks:

- [ ] Port database behavior tests into Rust unit tests or integration tests.
- [x] Port Excel export tests after native export is implemented.
- [x] Port updater tests after updater commands are implemented.
- [ ] Keep React tests focused on UI behavior and bridge contracts.
- [x] Add command-level tests for Tauri command inputs and errors where practical.

Acceptance criteria:

- [ ] Tests cover the old Electron parity risks.
- [ ] `bun run test` stays meaningful for React behavior.
- [ ] `cargo test` covers native storage, sync, export, updater, and path/URL safety.

### 18. Add Desktop Smoke Testing Checklist

Tasks:

- [ ] Launch `bun tauri dev`.
- [x] Verify first-run import.
- [x] Add entry.
- [x] Edit entry.
- [x] Toggle verified.
- [x] Archive and restore.
- [x] Delete with confirmation.
- [x] Search/filter/sort.
- [x] Open saved link.
- [x] Browse/open picture.
- [x] Export Excel cancel/save/open.
- [x] Confirm Excel two-sheet `Inventory`/`Archive` contract in packaged smoke.
- [ ] Simulate shared workspace unavailable.
- [ ] Simulate shared workspace reconnect.
- [ ] Check for update and install update.

Acceptance criteria:

- [x] Manual smoke checklist is documented and repeatable.
- [x] Any failed smoke item maps back to a TODO or issue.

## Priority 6 - Original App Roadmap Items Not Yet Implemented In Electron

These are not required for Electron parity because the original app also listed them as not implemented. Keep them separate from migration parity work.

### 19. Excel Or Database Import

Original status:

- Not implemented in Electron.

Tasks:

- [ ] Define accepted import formats.
- [ ] Decide whether imports write directly to FeOxDB or stage a review screen first.
- [ ] Track import issues so they can appear in an import review/report if that feature is added.
- [ ] Prevent duplicate entries by stable identifiers or configurable matching rules.

### 20. HTML Export

Original status:

- Placeholder only.

Tasks:

- [ ] Decide whether HTML export is still needed.
- [ ] Define output layout.
- [ ] Export active and archived entries.
- [ ] Include local assets safely or keep output self-contained.

### 21. Quick Edit Dialog

Original status:

- Not implemented in Electron.

Tasks:

- [ ] Decide whether quick edit is still valuable now that the full dialog is faster.
- [ ] Define which fields quick edit should expose.
- [ ] Add keyboard/mouse workflow without complicating the table.

### 22. TE / Template Variants

Original status:

- Not implemented in Electron.

Tasks:

- [ ] Define whether variants are separate apps, profiles, config files, or filtered views.
- [ ] Avoid duplicating the whole app for each variant.
- [ ] Make field labels and seed data configurable only if real requirements justify it.

## Cross-Cutting Quality Requirements

Apply these to every implementation item:

- [ ] Keep React UI free of Node/Electron APIs.
- [ ] Prefer Rust/Tauri commands for native work.
- [ ] Validate URLs and paths before opening anything.
- [ ] Avoid hardcoded secrets.
- [ ] Keep the app usable when shared storage or update storage is offline.
- [ ] Avoid blocking the UI during import, sync, export, and updater operations.
- [x] Flush FeOxDB after acknowledged local mutation/import writes.
- [ ] Add tests for behavior that can regress.
- [ ] Update `README.md` when user-visible behavior changes.
- [ ] Run relevant checks before marking a task complete:
  - [x] `bun run lint`
  - [x] `bun run build`
  - [x] `bun run test`
  - [x] `cargo fmt -- --check`
  - [x] `cargo check`
  - [x] `cargo test`
  - [x] `bun tauri build --bundles nsis` for packaging changes
