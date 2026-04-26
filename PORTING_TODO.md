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
- [x] Native bridge for safe external URL opening, local path opening, and picture path picking
- [x] TypeScript strict mode is enabled
- [x] Frontend tests, lint, frontend build, Rust format, Rust check, and Rust tests pass

Known current stubs or missing native bridge methods:

- [x] `openExternal`
- [x] `openPath`
- [x] `pickPicturePath`
- [ ] `exportExcel`
- [ ] real `checkForUpdate`
- [ ] real `downloadUpdate`
- [ ] real `installUpdate`
- [ ] real `onSharedInventoryChanged`
- [ ] real `onUpdateStateChanged`

## Priority 0 - Early Decisions Before More Work

### 1. Implement The Shared Data Architecture From `guidelines.md`

Original Electron reference:

- `electron/inventory-runtime.mjs`
- `electron/inventory-db.mjs`
- `src/test/inventory-db.test.ts`

Current Tauri state:

- Local runtime database is FeOxDB.
- SQLite files are treated as one-time import sources.
- Shared sync is disabled and reports local-only readiness.

Architecture decision:

- [x] Do not directly mutate one shared FeOxDB file on a network drive.
- [x] Use local FeOxDB per installation.
- [x] Use durable local outbox records.
- [x] Use shared-drive append-only operation files.
- [x] Use periodic snapshots, backups, and a single-writer merger/compactor.

Recommended direction:

- Follow `guidelines.md`.
- Keep the original Electron shared SQLite code as a behavior and migration reference only.
- Prefer an operation-ledger model for quantity changes and deterministic conflict handling.

Acceptance criteria:

- [ ] We know whether `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME` must remain the default shared workspace.
- [ ] We know whether `ME_LAB_SHARED_ROOT` must remain supported.
- [ ] We know whether installed Electron users must interoperate with new Tauri users during a transition period.
- [ ] We document the chosen architecture in `README.md`.
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

- [ ] Product name, version, identifier, icon, and installer behavior are intentional and documented.
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

- [ ] "Open Saved Link" opens a safe saved link in the default browser from the Tauri desktop app.
- [ ] "Search Online" opens a safe Google search URL from the Tauri desktop app.
- [ ] Unsafe protocols such as `file:`, `javascript:`, and arbitrary shell targets are rejected.
- [ ] Unit tests cover accepted and rejected URL cases.

### 4. Implement Native Open Path For Pictures

Original Electron reference:

- `electron/main.mjs` handler `inventory:open-path`
- `src/components/inventory/EntryDialog.tsx`

Current Tauri state:

- The UI can render file URL previews.
- Double-click/open picture can call `openPath` when the selected picture path is local.

Tasks:

- [x] Add a Tauri command such as `open_path`.
- [x] Validate that the input is a local filesystem path.
- [x] Trim whitespace and reject empty paths.
- [x] Reject obvious URL values here and route URLs through `openExternal`.
- [x] Check whether the path exists before opening and return `false`.
- [x] Wire `window.inventoryDesktop.openPath`.

Acceptance criteria:

- [ ] Double-clicking a valid local picture preview opens it with the default Windows handler.
- [ ] Missing picture paths show the existing "Picture not found" state and do not crash.
- [ ] Invalid paths do not execute shell commands.
- [ ] Tests cover valid path, missing path, and unsafe input behavior.

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
- [ ] Confirm paths with spaces and UNC paths behave correctly in a packaged desktop smoke test.

Acceptance criteria:

- [ ] Browse opens a native file picker in the Tauri app.
- [ ] Cancel returns `null` and leaves the form unchanged.
- [ ] Selecting an image updates the picture path and preview.
- [ ] Tests cover bridge wiring and UI behavior.

### 6. Implement Native Excel Export

Original Electron reference:

- `electron/inventory-export.mjs`
- `electron/inventory-export-worker.mjs`
- `src/test/inventory-export.test.ts`

Current Tauri state:

- `Export > Excel` exists in the UI.
- The bridge has an optional `exportExcel` type.
- Tauri bridge does not implement it, so the UI reports "Excel export is only available in the desktop app."

Tasks:

- [ ] Choose a lightweight Rust XLSX library and confirm it supports styles, autofilter, frozen rows, and print setup.
- [ ] Add a save-file dialog with default filename `ME_Inventory_Export.xlsx`.
- [ ] Export all entries, not just currently filtered rows.
- [ ] Include active and archived entries together in the `Inventory` sheet with an `Archived` column.
- [ ] Add an `Import Issues` sheet or decide how import issues are tracked in the FeOxDB migration.
- [ ] Add an `Export Summary` sheet.
- [ ] Preserve print-friendly styling:
  - [ ] styled header row
  - [ ] zebra striping
  - [ ] borders
  - [ ] frozen top row
  - [ ] autofilter
  - [ ] landscape print setup
  - [ ] lifecycle/status fills where practical
- [ ] Return `{ canceled, outputPath, error }` through `window.inventoryDesktop.exportExcel`.

Acceptance criteria:

- [ ] Desktop `Export > Excel` opens a native save dialog.
- [ ] Cancel is silent.
- [ ] Saved workbook opens in Excel.
- [ ] Workbook has `Inventory`, `Import Issues`, and `Export Summary` sheets or a documented replacement.
- [ ] Summary labels match original wording:
  - [ ] `ME Inventory - Export Summary`
  - [ ] `Entry Scope`
  - [ ] `Total Entries`
  - [ ] `Inventory View Entries`
  - [ ] `Archived Entries`
- [ ] Automated tests validate workbook structure and key summary labels.

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

- `sync_inventory` returns no changed entries and a local-ready message.
- `InventorySharedStatus.enabled` is false.
- `onSharedInventoryChanged` is a no-op.

Tasks:

- [ ] Finalize architecture from Priority 0.
- [ ] Map the `guidelines.md` operation schema onto current `InventoryEntry` fields.
- [ ] Decide whether current entries become `inventory:item:*` records immediately or through a compatibility layer.
- [ ] Add stable `client_id`, `device_id`, app version, schema version, and local sequence metadata.
- [ ] Add durable local outbox keyspace.
- [ ] Define Rust data structures for sync operations, tombstones, revision state, and conflict records.
- [ ] Implement shared root resolution.
- [ ] Implement shared operation-log bootstrap.
- [ ] Implement push pending local operations.
- [ ] Implement pull missing shared operations.
- [ ] Implement delete tombstones.
- [ ] Implement conflict detection and logging.
- [ ] Implement revision tracking.
- [ ] Implement local-only mutation state and `hasLocalOnlyChanges`.
- [ ] Implement manifest reading and validation.
- [ ] Implement snapshot reading and validation.
- [ ] Implement checksum validation for operation files and snapshots.
- [ ] Implement temp-file-then-rename writes for all shared artifacts.
- [ ] Ignore `.tmp`, corrupt, unknown-extension, checksum-invalid, and identity-mismatched operation files.
- [ ] Emit a frontend event when shared inventory changes.
- [ ] Avoid reloading/repainting when data did not change.
- [ ] Add clear status messages matching original behavior.

Acceptance criteria:

- [ ] When shared workspace is unavailable, add/edit/delete/verify/archive still work locally.
- [ ] UI reports local-only pending state.
- [ ] When shared workspace reconnects, pending local operations are pushed.
- [ ] Changes made by another client are pulled.
- [ ] Deletes remain deleted across sync because tombstones are applied.
- [ ] Repeated syncs are idempotent.
- [ ] Busy or locked shared files do not corrupt data.
- [ ] Tests cover bootstrap, unavailable shared root, push, pull, conflicts, delete tombstones, and reconnect behavior.

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

- `checkForUpdate`, `downloadUpdate`, and `installUpdate` return idle state.
- `onUpdateStateChanged` is a no-op.

Tasks:

- [ ] Decide whether to keep custom shared-drive updater or use a Tauri updater flow.
- [ ] Implement version comparison in Rust or TypeScript with tests.
- [ ] Resolve the manifest path from the shared root.
- [ ] Parse and validate manifest fields.
- [ ] Verify 64-character hex SHA-256.
- [ ] Download or copy installer into a local cache directory.
- [ ] Verify downloaded hash.
- [ ] Emit update-state events to the UI.
- [ ] Launch the installer visibly.
- [ ] Handle errors without breaking inventory usage.

Acceptance criteria:

- [ ] No update available state is quiet.
- [ ] Newer manifest shows `Update available`.
- [ ] Download progress is represented in `UpdateState`.
- [ ] Bad manifest hash is rejected with a clear error.
- [ ] Hash mismatch is rejected.
- [ ] Ready installer changes action to install.
- [ ] Install opens the installer and keeps app behavior predictable.
- [ ] Tests port the important cases from `src/test/updater.test.ts`.

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
- [ ] Add import issue tracking if Excel export keeps an `Import Issues` sheet.
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
- [ ] Confirm picture picker and open picture after native commands are added.
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
- [ ] Confirm `Open Saved Link`.
- [ ] Confirm `Search Online`.
- [ ] Confirm `Archive Entry` / `Restore Entry`.
- [ ] Confirm `Delete Entry` opens confirmation dialog.

Acceptance criteria:

- [ ] Keyboard/mouse behavior remains accessible.
- [ ] Unsafe links do not open.
- [ ] Status strip messages match expected outcomes.

## Priority 4 - Packaging, Installer, And Release Flow

### 15. Build And Verify Tauri NSIS Installer

Current Tauri state:

- `bun tauri build --bundles nsis` is configured but has not been validated in the clean port during this TODO pass.

Tasks:

- [ ] Run full Tauri bundle build.
- [ ] Verify installer output path.
- [ ] Verify app icon.
- [ ] Verify bundled `data\me_inventory.db` and `data\me_lab_inventory.db` resources are available for first-run import.
- [ ] Verify app launches after install.
- [ ] Verify app data directory behavior.
- [ ] Verify uninstall/reinstall does not unexpectedly erase user data.

Acceptance criteria:

- [ ] Installer builds successfully on Windows.
- [ ] Fresh install imports seed inventory once.
- [ ] Reopening app keeps prior changes.
- [ ] Installed app has correct product name, icon, and version.

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
- [ ] Port Excel export tests after native export is implemented.
- [ ] Port updater tests after updater commands are implemented.
- [ ] Keep React tests focused on UI behavior and bridge contracts.
- [ ] Add command-level tests for Tauri command inputs and errors where practical.

Acceptance criteria:

- [ ] Tests cover the old Electron parity risks.
- [ ] `bun run test` stays meaningful for React behavior.
- [ ] `cargo test` covers native storage, sync, export, updater, and path/URL safety.

### 18. Add Desktop Smoke Testing Checklist

Tasks:

- [ ] Launch `bun tauri dev`.
- [ ] Verify first-run import.
- [ ] Add entry.
- [ ] Edit entry.
- [ ] Toggle verified.
- [ ] Archive and restore.
- [ ] Delete with confirmation.
- [ ] Search/filter/sort.
- [ ] Open saved link.
- [ ] Browse/open picture.
- [ ] Export Excel.
- [ ] Simulate shared workspace unavailable.
- [ ] Simulate shared workspace reconnect.
- [ ] Check for update and install update.

Acceptance criteria:

- [ ] Manual smoke checklist is documented and repeatable.
- [ ] Any failed smoke item maps back to a TODO or issue.

## Priority 6 - Original App Roadmap Items Not Yet Implemented In Electron

These are not required for Electron parity because the original app also listed them as not implemented. Keep them separate from migration parity work.

### 19. Excel Or Database Import

Original status:

- Not implemented in Electron.

Tasks:

- [ ] Define accepted import formats.
- [ ] Decide whether imports write directly to FeOxDB or stage a review screen first.
- [ ] Track import issues so they can appear in exports.
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
- [ ] Add tests for behavior that can regress.
- [ ] Update `README.md` when user-visible behavior changes.
- [ ] Run relevant checks before marking a task complete:
  - [ ] `bun run lint`
  - [ ] `bun run build`
  - [ ] `bun run test`
  - [ ] `cargo fmt -- --check`
  - [ ] `cargo check`
  - [ ] `cargo test`
  - [ ] `bun tauri build --bundles nsis` for packaging changes
