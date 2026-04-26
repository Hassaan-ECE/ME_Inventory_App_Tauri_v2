# ME Inventory

ME Inventory is a Tauri 2 desktop inventory app built with React 19, TypeScript, Vite, Tailwind CSS v4, Bun, and FeOxDB.

The app keeps the existing ME Lab Inventory UI workflow and uses FeOxDB as the authoritative local runtime database. SQLite is used only as a one-time legacy import source.

Current app display name: `ME Inventory v0.9.7`.

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
- native external URL opening, local file opening, and picture path picker commands
- local-only desktop bridge compatible with the existing React UI

Deferred native ports:

- Excel export
- HTML export
- shared workspace sync
- app updater

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

This is the current compatibility projection for the existing ME Lab Inventory entry workflow. The target shared-data architecture is documented in `guidelines.md`: each installation owns its local FeOxDB file, and future shared sync should use durable local outbox records plus append-only operation files, snapshots, and a single-writer manifest/compactor on the shared drive.

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
- `open_external`
- `open_path`
- `pick_picture_path`

`query_inventory` range-scans FeOxDB entries in memory, then applies scope, search, filters, sort, offset, and limit. This is intentional for the current dataset size. Add secondary indexes later if the inventory grows enough to need them.

The native open commands validate their inputs before calling OS opener APIs. External links are limited to `http`, `https`, and `mailto`; local path opening rejects URL-like input and requires an absolute path that exists.

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
- The old shared SQLite sync/outbox model is deferred. The intended replacement is the `guidelines.md` design: local FeOxDB per installation, durable operation outbox, shared-drive append-only operation files, periodic snapshots, and single-writer compaction.
- Node APIs should not be used from React. Native work belongs in Rust/Tauri commands.
- Old Electron files remain in `D:\Projects\Active\ME_Inventory_App` as reference material only and are not part of this Tauri source tree.
