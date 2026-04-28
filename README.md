# ME Inventory

ME Inventory is a Tauri 2 desktop inventory app built with React 19, TypeScript, Vite, Tailwind CSS v4, Bun, and FeOxDB.

Built by Syed Hassaan Shah.

The app keeps the existing ME Lab Inventory UI workflow and uses FeOxDB as the authoritative local runtime database. SQLite is used only as a one-time legacy import source.

Current app display name: `ME Inventory v0.9.6`.

## Migration Baseline

This folder is the active Tauri v2 port:

- active repo: `D:\Projects\Active\ME_Inventory_App_Tauri_v2`
- source baseline imported from: `D:\coding\ME Lab Inventory`
- behavior reference only: `D:\Projects\Active\ME_Inventory_App`

The old Electron source, generated build output, dependency folders, and release artifacts are intentionally not copied into this repo. When behavior is unclear, compare against the original Electron app, then implement the Tauri version in React/Rust source here.

## Project Docs

- `guidelines.md` is the working architecture guide for local FeOxDB storage, shared-drive operation sync, snapshots, compaction, conflict handling, and quality rules.
- `PORTING_TODO.md` is the active parity checklist against the original Electron app.

## Current Status

Implemented in this Tauri/FeOxDB migration:

- Tauri 2 desktop shell with one main window
- NSIS Windows installer targeting current-user install
- Bun package workflow
- React inventory/archive UI from the existing ME Lab Inventory app
- global search, column filters, sorting, column visibility, Color Rows, and theme persistence
- add, edit, verify, archive, restore, and delete entry flows
- right-click context menu and full entry dialog
- FeOxDB-backed local inventory storage
- first-run import from the legacy SQLite database
- native external URL opening, local image opening, preview-safe local image loading, and picture path picker commands
- native two-sheet Excel export for the current Tauri entry fields
- safe Tauri updater command scaffold, not configured for real updates yet
- automated current-user NSIS installer smoke for install, launch, bundled resources, first-run import, app data location, and uninstall/reinstall preservation
- shared operation-log sync foundation using local FeOxDB outbox records, append-only shared operation files, last-write-wins entry state, conflict logging, and a native shared-ops watcher

Deferred native ports:

- HTML export
- shared sync snapshots, manifest compaction, conflict UI, and shared media storage
- real app updater signing, endpoint configuration, and release hosting

Those deferred controls are hidden, disabled, or report that the feature is not available in this Tauri build.

## Setup

Install dependencies:

```powershell
bun install
```

Run the web UI only:

```powershell
bun run dev
```

Run the Tauri desktop app:

```powershell
bun tauri dev
```

Build the frontend:

```powershell
bun run build
```

Run tests:

```powershell
bun run test
```

Run the one-machine shared-sync smoke:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\smoke-sync-one-machine.ps1
```

The smoke script creates two isolated FeOxDB clients under `%TEMP%\me-inventory-sync-smoke`, writes operation files under `shared-root\shared\inventory\ops\{client_id}`, verifies convergence, confirms stale operations are logged as conflicts, and checks delete plus newer-restore behavior.

Build the Windows NSIS installer:

```powershell
bun tauri build --bundles nsis
```

The installer is written under:

```powershell
src-tauri\target\release\bundle\nsis\
```

## Data Model And Storage

Runtime storage is FeOxDB:

- file: `inventory.feox`
- location: Tauri app data directory for `com.me.inventory`
- keys: `entry:{entry_uuid}`
- values: JSON `InventoryEntry` records

Each entry includes:

- `id` / `databaseId`
- `entryUuid`
- asset number, serial number, quantity
- manufacturer, model, description, project, location, assigned user
- links, notes, lifecycle status, working status, condition
- verified, archived, manual entry, picture path
- created and updated timestamps

FeOxDB metadata keys are used for:

- next numeric entry ID
- legacy SQLite import state

This is the current compatibility projection for the existing ME Lab Inventory entry workflow. The shared-data architecture is documented in `guidelines.md`: each installation owns its local FeOxDB file, shared sync starts with durable local outbox records plus append-only operation files, and later waves should add snapshots plus a single-writer manifest/compactor on the shared drive.

## Legacy SQLite Import

On first startup, if FeOxDB has no entries and no import marker, the app attempts to import from SQLite.

Import candidates:

- `ME_INVENTORY_LEGACY_SQLITE` environment variable, when set
- `data\me_inventory.db`
- `data\me_lab_inventory.db`
- bundled resource copies of those files

SQLite remains read-only migration input. After import, FeOxDB is authoritative.

Supported source schemas:

- current `entries` schema
- legacy `equipment` schema with `record_id` / `record_uuid` compatibility mapping

## Tauri Commands

The React bridge calls these Tauri commands:

- `load_inventory`
- `query_inventory`
- `sync_inventory`
- `create_entry`
- `update_entry`
- `toggle_verified_entry`
- `set_archived_entry`
- `delete_entry`
- `import_legacy_sqlite`
- `export_excel`
- `open_external`
- `open_path`
- `pick_picture_path`
- `check_for_update`
- `download_update`
- `install_update`

`query_inventory` range-scans FeOxDB entries in memory, then applies scope, search, filters, sort, offset, and limit. This is intentional for the current dataset size. Add secondary indexes later if the inventory grows enough to need them.

The native open and preview commands validate their inputs before calling OS opener APIs or reading files. External links are limited to `http`, `https`, and `mailto`; local picture opening and preview loading reject URL-like input and require an existing absolute image path with a supported image extension.

`export_excel` writes a workbook with exactly two sheets: `Inventory` for active entries and `Archive` for archived entries. The current export uses the Tauri `InventoryEntry` fields only; old Electron-only calibration/rental fields remain out of scope until the data model supports them.

The updater commands are a safe scaffold. They do not hardcode fake public keys, fake endpoints, or insecure transport. Real Tauri updater release support still requires signed updater artifacts, a public key, `createUpdaterArtifacts`, and an HTTPS/static or dynamic update source.

## Release QA Status

Automated release smoke on 2026-04-26 validated the current-user NSIS installer path, installed app launch, `ME Inventory` product name, version `0.9.7`, `com.me.inventory` app data behavior, bundled SQLite resource hashes, first-run import of 146 seed rows, and uninstall/reinstall preservation of app data. Syed's manual installed-app smoke validated the visible installer/shortcut flow, delta icon, installed launch, first-run rows, no duplicate import, search, CRUD persistence, verify/archive/restore/delete, Search Online, Excel cancel/save/open, active plus archived export data, and quiet updater scaffold. Post-picture-fix evidence records the latest shared installer hash and Syed's confirmation that local picture preview, picture open, missing-picture state, `Open Saved Link`, Excel two-sheet export, local-only sync status, and updater quiet state work. The detailed logs are in `docs/installer-smoke-2026-04-26-worker-a.md`, `docs/release-qa-2026-04-26-manager.md`, `docs/manual-smoke-2026-04-26-syed.md`, and `docs/post-picture-fix-smoke-2026-04-26.md`.

The remaining release-smoke gaps are optional unsafe-link manual testing, shared media storage, and Windows SmartScreen or enterprise-policy behavior for the unsigned installer.

## Validation

Recommended checks:

```powershell
bun run build
bun run test

Push-Location src-tauri
cargo fmt -- --check
cargo check
cargo test
Pop-Location

bun tauri build --bundles nsis
```

## Electron Migration Notes

- Electron IPC has been replaced by Tauri commands through `src/lib/tauriInventoryBridge.ts`.
- SQLite is no longer runtime storage. It is only a migration source read by Rust.
- The old shared SQLite sync/outbox model is not used. The Tauri sync foundation uses the `guidelines.md` design: local FeOxDB per installation, durable operation outbox, shared-drive append-only operation files, per-entry last-write-wins state, and logged stale-operation conflicts. Periodic snapshots, manifest publishing, conflict UI, and single-writer compaction remain TODOs.
- The next shared-sync implementation slice is documented in `docs/sync-architecture-next-slice.md`.
- Node APIs should not be used from React. Native work belongs in Rust/Tauri commands.
- Old Electron files remain in `D:\Projects\Active\ME_Inventory_App` as reference material only and are not part of this Tauri source tree.
