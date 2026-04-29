# ME Inventory

Last consolidated: 2026-04-29

ME Inventory is a Windows desktop inventory app built with Tauri 2, React 19, TypeScript, Vite, Tailwind CSS v4, Bun, Rust, and FeOxDB.

Built by Syed Hassaan Shah.

This README is the canonical project doc. Older planning notes, smoke logs, and architecture drafts were folded into this file so the current working state lives in one place.

## Current Source Truth

- Active workspace: `c:\Projects\Active\ME_Inventory_App_Tauri_v2`
- App name: `ME Inventory`
- Display name: `ME Inventory v0.9.8`
- Version source: `package.json`, `src-tauri\Cargo.toml`, and `src-tauri\tauri.conf.json`
- Tauri identifier: `com.me.inventory`
- Install mode: current-user NSIS install
- Updater: signed Tauri updater with GitHub Releases static metadata
- Runtime database: local FeOxDB file named `inventory.feox`
- Legacy SQLite role: read-only first-run import source only

Version note: `0.9.8` is the current source truth for this release checkpoint.

## What Works Now

### Desktop App

- Tauri 2 shell with one main window.
- Current-user NSIS packaging.
- App data is stored under the Tauri app data folder for `com.me.inventory`.
- Bundled seed resources include `data\me_inventory.db` and `data\me_lab_inventory.db`.
- The installed app has been smoke-tested in the past for launch, first-run import, local persistence, and uninstall/reinstall preserving app data. Rerun the release checklist before shipping a new build.

### Inventory UI

- Inventory and Archive views.
- Global search.
- Column filters for asset number, manufacturer, model, description, and location.
- Sorting and column visibility.
- At least one data column must remain visible.
- Color Rows toggle.
- Theme persistence.
- Virtualized table rendering for larger result sets.
- Add, edit, verify, archive, restore, and delete flows.
- Full entry dialog.
- Right-click context menu with open, saved-link, online-search, archive/restore, and delete actions.
- Styled in-app delete confirmation.

### Entry Fields

The current `InventoryEntry` projection supports:

- `id`, `databaseId`, and `entryUuid`
- asset number and serial number
- quantity
- manufacturer, model, description, project, location, and assigned user
- links and notes
- lifecycle status
- working status
- condition
- verified state
- archived state
- manual-entry marker
- picture path
- created and updated timestamps

This is a compatibility projection for the existing ME Lab Inventory workflow. It is not the future ledger-based stock model yet.

### Local Storage And Import

- FeOxDB is the authoritative runtime store.
- Entries are stored under `entry:{entry_uuid}`.
- Metadata stores next numeric entry ID, legacy import state, sync identity, local sequence state, outbox records, applied operation markers, entry sync state, tombstones, conflicts, and corrupt remote-file records.
- First startup imports from legacy SQLite only when FeOxDB has no entries and no import marker.
- Import candidates:
  - `ME_INVENTORY_LEGACY_SQLITE`
  - `data\me_inventory.db`
  - `data\me_lab_inventory.db`
  - bundled resource copies of those files
- Supported SQLite schemas:
  - current `entries`
  - legacy `equipment` with `record_id` and `record_uuid` compatibility mapping

SQLite stays read-only. After import, FeOxDB owns the data.

### Native Links And Pictures

- Saved browser/email links open through Rust/Tauri after validation.
- Allowed external schemes are `http`, `https`, and `mailto`.
- Unsafe schemes and local filesystem paths are rejected by the native opener path.
- Local picture opening uses a separate Rust command and accepts only absolute local image paths.
- Supported picture extensions are `png`, `jpg`, `jpeg`, `webp`, `gif`, `bmp`, `tif`, and `tiff`.
- Picture previews use a validated cache-backed app-cache copy and Tauri asset URLs.
- Preview loading rejects missing, invalid, URL-like, unsupported-extension, and oversized source files. The current preview source limit is 50 MB.
- The native picker saves the selected absolute picture path on the entry.

### Excel Export

- `Export > Excel` uses a native save dialog.
- Default filename: `ME_Inventory_Export.xlsx`.
- Export includes all entries, not only the visible filtered rows.
- The workbook has exactly two sheets:
  - `Inventory` for active entries
  - `Archive` for archived entries
- The workbook includes headers, borders, zebra striping, frozen top row, autofilter, landscape print setup, and status coloring where practical.
- The current export covers the current Tauri `InventoryEntry` fields only.

### Shared Sync Foundation

The app has the first shared-drive operation-log sync layer.

- Each installation owns its local FeOxDB file.
- Clients do not mutate one shared FeOxDB file.
- Shared root resolution:
  - `ME_LAB_SHARED_ROOT`
  - fallback: `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME`
- Operation files are written under:

```text
<shared root>\shared\inventory\ops\{client_id}\000000000001.op.json
```

- Local mutations queue durable outbox operations before FeOxDB flush.
- Shared sync pushes pending local operations and pulls remote operations.
- Operation files use checksums and temp-file-then-rename writes.
- Readers ignore temp files, corrupt JSON, unknown extensions, checksum-invalid files, and identity-mismatched operation files.
- Last-write-wins entry state uses `(mutation_ts_utc, op_id)` ordering.
- Deletes create tombstones.
- Older operations are skipped and logged as conflicts.
- Newer upserts after a tombstone restore the entry.
- Repeated syncs are intended to be idempotent.
- The native watcher emits `inventory:shared-changed`; the frontend coalesces sync work so overlapping sync passes do not stack up.
- The frontend status reports local readiness, shared-root availability, local-only pending state, mutation mode, and revision.

Shared sync is not complete yet. Snapshots, manifest publishing, single-writer compaction, conflict UI, shared media storage, and multi-machine release smoke remain open.

### Signed Tauri Updater

The app uses the official signed Tauri updater. Update metadata is expected at:

```text
https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/latest/download/latest.json
```

- `src-tauri\tauri.conf.json` stores the updater public key and endpoint.
- The private signing key is generated outside the repo at `%USERPROFILE%\.tauri\me-inventory-updater.key`.
- The private key and password must never be committed.
- `bundle.createUpdaterArtifacts` is enabled so release builds produce updater artifacts and signatures.
- The frontend keeps the existing `UpdateState` shape and receives real download progress events.

The generated updater key currently has no password. Rotate it before broad distribution if release policy requires a password-protected private key.

## Setup

Install dependencies:

```powershell
bun install
```

Run the web UI:

```powershell
bun run dev
```

Run the Tauri desktop app:

```powershell
bun run dev:desktop
```

Build the frontend:

```powershell
bun run build
```

Run frontend tests:

```powershell
bun run test
```

Run the one-machine shared-sync smoke:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\smoke-sync-one-machine.ps1
```

Build the Windows NSIS installer:

```powershell
bun tauri build --bundles nsis
```

Installer output:

```powershell
src-tauri\target\release\bundle\nsis\
```

## Tauri Commands

The React bridge calls these commands:

- `load_inventory`
- `query_inventory`
- `sync_inventory`
- `create_entry`
- `update_entry`
- `toggle_verified_entry`
- `set_archived_entry`
- `delete_entry`
- `import_legacy_sqlite`
- `load_picture_preview`
- `export_excel`
- `open_external`
- `open_path`
- `pick_picture_path`

`query_inventory` currently range-scans FeOxDB entries in memory, then applies scope, search, filters, sort, offset, and limit. That fits the current dataset. Add secondary indexes, cached normalized search text, or server-side pagination if the inventory grows enough to make scans or table rendering slow.

## Release Checklist

Before building a release candidate:

```powershell
bun run lint
bun run build
bun run test

Push-Location src-tauri
cargo fmt -- --check
cargo check
cargo test
Pop-Location

bun tauri build --bundles nsis
```

For signed updater releases, build with the updater private key available outside the repo:

```powershell
$env:PATH = "$env:USERPROFILE\.bun\bin;$env:PATH"
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content -LiteralPath "$env:USERPROFILE\.tauri\me-inventory-updater.key" -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
bun tauri build --bundles nsis
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

Publish the generated NSIS installer, its `.sig` file, and a GitHub Release asset named `latest.json`. Static updater metadata must include the Tauri updater fields for the Windows platform:

```json
{
  "version": "0.9.8",
  "notes": "Release notes",
  "pub_date": "2026-04-29T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "contents of the generated .sig file",
      "url": "https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/download/v0.9.8/ME_Inventory_0.9.8_x64-setup.exe"
    }
  }
}
```

Manual smoke for a release candidate:

- Confirm `package.json`, `src-tauri\Cargo.toml`, and `src-tauri\tauri.conf.json` versions match.
- Confirm the app identifier is still `com.me.inventory`.
- Install as the current Windows user.
- Launch from the installed shortcut.
- Confirm the visible name and version.
- On clean app data, confirm first-run SQLite import loads entries once.
- Close and reopen, then confirm row count stays stable.
- Add, edit, verify, archive, restore, and delete a disposable smoke entry.
- Save and open a safe `https://` link.
- Run `Search Online`.
- Select a local picture path with spaces, confirm preview, then open it.
- Confirm a missing picture path shows the missing state without crashing.
- Export Excel, cancel once, then save once to a path with spaces.
- Open the workbook and confirm exactly `Inventory` and `Archive` sheets.
- Confirm the updater stays quiet when no newer signed GitHub Release metadata exists.
- If a newer signed release exists, confirm check, download progress, install, and relaunch/update behavior.
- If a shared test root is available, confirm local pending operations push and another client can pull them.
- Record installer path, updater `.sig` path, commit, source version, tester, and date.

Known release caveats:

- The installer and executable are not signed by repo configuration.
- Windows SmartScreen and enterprise policy behavior still need environment-specific verification.
- Tauri updater artifact signing is configured, but Windows code signing is still separate.
- Changing the Tauri identifier after users install the app can strand app data in a different directory.

## Optional Memory And Lifecycle Audit

Use this when the app feels sluggish, memory grows after repeated UI work, or sync/export/picture changes touch lifecycle-sensitive code.

Static source sweep:

```powershell
rg -n "useEffect|addEventListener|removeEventListener|setInterval|clearInterval|setTimeout|clearTimeout|ResizeObserver|URL\.createObjectURL|invoke\(|listen\(|unlisten|on[A-Z].*Changed" src src-tauri/src src-tauri/tests
```

Rust retention and file IO sweep:

```powershell
rg -n "Arc|Mutex|RwLock|static|thread|spawn|channel|range_query|collect::<|Vec<|fs::read|File::|Workbook|Connection::open" src-tauri/src src-tauri/tests
```

Manual exercise:

- Start the app and record `me-inventory` plus `msedgewebview2` memory after idle.
- Run repeated search, filter, sort, menu, dialog, picture preview, CRUD, sync idle, and Excel export cycles.
- Record memory after the exercise and again after idle.
- Close the app and confirm app-owned processes exit.
- Keep profiler output, screenshots, traces, workbooks, and app-data backups out of commits unless a specific evidence file should become durable documentation.

## Open Work

- Bump or reconcile the source version when preparing the next release after `0.9.8`.
- Validate the signed GitHub Releases updater path with a real release asset.
- Finish shared sync snapshots, manifest validation, single-writer compaction, conflict UI, locked-file smoke, and shared media storage.
- Decide whether entries should move from the current compatibility projection to future `inventory:item:*` and ledger keyspaces.
- Add import issue tracking and clearer unknown-schema errors for legacy SQLite import.
- Benchmark real inventory size for search, sort, startup, sync, and table rendering.
- Add FeOxDB schema versioning and a future migration path.
- Confirm UNC picture path behavior in a packaged smoke.
- Keep HTML export as an explicit placeholder unless it becomes a real requirement.
- Keep `com.me.inventory`; do not change the Tauri identifier without a migration plan.
