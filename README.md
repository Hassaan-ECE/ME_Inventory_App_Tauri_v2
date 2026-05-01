# ME Inventory

Last consolidated: 2026-05-01

ME Inventory is a Windows desktop inventory app built with Tauri 2, React 19, TypeScript, Vite, Tailwind CSS v4, Bun, Rust, and FeOxDB.

Built by Syed Hassaan Shah.

This README is the canonical project doc. Older planning notes, smoke logs, and architecture drafts were folded into this file so the current working state lives in one place.

## Current Source Truth

- Active workspace: `c:\Projects\Active\ME_Inventory_App_Tauri_v2`
- App name: `ME Inventory`
- Display name: `ME Inventory v1.0.0`
- Version source: `package.json`, `backend\Cargo.toml`, and `backend\tauri.conf.json`
- Tauri identifier: `com.me.inventory`
- Install mode: current-user NSIS install
- Updater: signed Tauri updater with GitHub Releases static metadata
- Runtime database: local FeOxDB file named `inventory.feox`
- Shared sync: S-drive FeOx operation logs plus manifest/snapshot bootstrap
- Deprecated local `.db` files: quarantined once into app-data backups and never used as data sources

Version note: `1.0.0` is the current source truth for the signed updater target. `0.9.9` is the expected updater baseline for installed-machine smoke.

## Project Layout

```text
frontend/     React/Vite UI, frontend tests, UI assets, and Tauri bridge code
backend/      Tauri/Rust app, commands, storage, sync, export, import, and native helpers
docs/         Engineering runbooks, cleanup checklists, and performance baselines
scripts/      Smoke/manual automation scripts
```

## What Works Now

### Desktop App

- Tauri 2 shell with one main window.
- Current-user NSIS packaging.
- App data is stored under the Tauri app data folder for `com.me.inventory`.
- The installed app has been smoke-tested in the past for launch, local persistence, shared sync, and uninstall/reinstall preserving app data. Rerun the release checklist before shipping a new build.

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

### Local Storage

- FeOxDB is the authoritative runtime store.
- Entries are stored under `entry:{entry_uuid}`.
- Metadata stores next numeric entry ID, sync identity, local sequence state, snapshot state, outbox records, applied operation markers, entry sync state, tombstones, conflicts, and corrupt remote-file records.
- Startup opens `inventory.feox` directly and does not inspect any legacy database files.
- On first `1.0.0` startup, known old app-owned `.db` files are moved to `deprecated-db-backups` under app data.
- Normal commands read and write FeOxDB only; load, query, export, mutation, and sync paths do not inspect any other database format.

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

`0.9.7` moved normal shared workflow to local FeOxDB plus S-drive operation logs. `0.9.8` made FeOxDB shared sync near-live. `0.9.9` showed local FeOxDB rows before shared sync and published saved changes from a backend background task. `1.0.0` removes the old database import path and makes FeOxDB snapshots plus operation logs the clean-install bootstrap path.

- Each installation owns its local FeOxDB file.
- Clients do not mutate one shared FeOxDB file.
- Shared root resolution:
  - `ME_LAB_SHARED_ROOT`
  - fallback: `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME`
- Operation files are written under:

```text
<shared root>\shared\inventory\ops\{client_id}\000000000001.op.json
```

- Snapshot files are written under:

```text
<shared root>\shared\inventory\snapshots\snapshot-*.snapshot.json
```

- The latest snapshot is advertised by:

```text
<shared root>\shared\inventory\manifest.json
```

- Local mutations queue durable outbox operations before FeOxDB flush.
- Shared sync pushes pending local operations and pulls remote operations.
- Clean machines apply the latest verified snapshot, then apply operation files newer than the snapshot watermarks.
- Snapshot publishing uses a single-writer lock under `shared\inventory\locks`.
- Covered operation files are compacted after the snapshot and manifest are verified.
- Snapshot and manifest failures leave local FeOxDB untouched and keep the app on operation-log sync.
- Operation files use checksums and temp-file-then-rename writes.
- Readers ignore temp files, corrupt JSON, unknown extensions, checksum-invalid files, and identity-mismatched operation files.
- Last-write-wins entry state uses `(mutation_ts_utc, op_id)` ordering.
- Deletes create tombstones.
- Older operations are skipped and logged as conflicts.
- Newer upserts after a tombstone restore the entry.
- Repeated syncs are intended to be idempotent.
- The native watcher emits `inventory:shared-changed`; the frontend coalesces sync work so overlapping sync passes do not stack up.
- Open clients also run a 500ms fallback sync poll so S-drive changes still land quickly when the network filesystem watcher misses a remote change.
- The frontend status reports local readiness, shared-root availability, local-only pending state, mutation mode, revision, and last snapshot id.

The FeOxDB operation-log path now merges concurrent non-overlapping field edits when both edits started from the same base version. Overlapping edits still use the existing last-newer-operation-wins behavior and record stale conflicts.

### Signed Tauri Updater

The app uses the official signed Tauri updater. Update metadata is expected at:

```text
https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/latest/download/latest.json
```

- `backend\tauri.conf.json` stores the updater public key and endpoint.
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
bun run build:desktop
```

Installer output:

```powershell
backend\target\release\bundle\nsis\
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
- `load_picture_preview`
- `export_excel`
- `open_external`
- `open_path`
- `pick_picture_path`

`query_inventory` currently range-scans FeOxDB entries in memory, then applies scope, search, filters, sort, offset, and limit. That fits the current dataset. Add secondary indexes, cached normalized search text, or server-side pagination if the inventory grows enough to make scans or table rendering slow.

## Release Checklist

Before building a release candidate:

The global Bun PowerShell shim can resolve to a stale wrapper on this workstation. Use the direct Bun binary for release validation until the shim is fixed:

```powershell
& "$env:USERPROFILE\.bun\bin\bun.exe" run lint
& "$env:USERPROFILE\.bun\bin\bun.exe" run test
& "$env:USERPROFILE\.bun\bin\bun.exe" run build

Push-Location backend
cargo fmt -- --check
cargo check
cargo test
Pop-Location
```

For signed updater releases, build with the updater private key available outside the repo. The fresh `0.9.x` smoke line rotated the updater key at `%USERPROFILE%\.tauri\me-inventory-updater.key`; keep that private key out of the repo.

```powershell
$env:PATH = "$env:USERPROFILE\.bun\bin;$env:PATH"
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content -LiteralPath "$env:USERPROFILE\.tauri\me-inventory-updater.key" -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
& "$env:USERPROFILE\.bun\bin\bun.exe" run build:desktop
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

Fresh shared-release staging uses `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases`. Before restarting at `0.9.0`, archive existing release folders into `backups\old-releases-YYYYMMDD-HHMMSS\` and back up root `current.json` as `backups\current-before-fresh-0.9.0-YYYYMMDD-HHMMSS.json`.

Publish the generated `0.9.1` NSIS installer, its `.sig` file, SHA-256 sums, and a GitHub Release asset named `latest.json`. The GitHub CLI is not installed on this workstation, so upload the staged files manually to a non-draft, non-prerelease GitHub Release tagged `v0.9.1`. Static updater metadata must include the Tauri updater fields for the Windows platform:

```json
{
  "version": "0.9.1",
  "notes": "Release notes",
  "pub_date": "2026-04-30T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "contents of the generated .sig file",
      "url": "https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/download/v0.9.1/ME%20Inventory_0.9.1_x64-setup.exe"
    }
  }
}
```

Fresh `1.0.0` manual smoke:

- Confirm `package.json`, `backend\Cargo.toml`, and `backend\tauri.conf.json` versions match.
- Confirm the app identifier is still `com.me.inventory`.
- Update an installed `0.9.9` machine to `1.0.0`.
- Launch from the installed shortcut.
- Confirm the visible name and version are `ME Inventory v1.0.0`.
- On clean app data, confirm startup hydrates from the S-drive FeOx snapshot and newer operation files.
- Close and reopen, then confirm row count stays stable.
- Add, edit, verify, archive, restore, and delete a disposable smoke entry.
- Save and open a safe `https://` link.
- Run `Search Online`.
- Select a local picture path with spaces, confirm preview, then open it.
- Confirm a missing picture path shows the missing state without crashing.
- Export Excel, cancel once, then save once to a path with spaces.
- Open the workbook and confirm exactly `Inventory` and `Archive` sheets.
- Upload the staged `1.0.0` GitHub Release assets, then from the installed `0.9.9` app confirm update check, download progress, install, and relaunch/update behavior.
- Run a real shared-drive multi-machine smoke and confirm create/update/delete convergence plus stale-update conflict logging.
- Confirm known old app-owned `.db` files are moved to `deprecated-db-backups` and are not loaded.
- Record installer path, updater `.sig` path, GitHub release URL, SHA-256, commit, source version, tester, machines, result, and date.

Known release caveats:

- The installer and executable are not signed by repo configuration.
- Windows SmartScreen and enterprise policy behavior still need environment-specific verification.
- Tauri updater artifact signing is configured, but Windows code signing is still separate.
- Changing the Tauri identifier after users install the app can strand app data in a different directory.

## Optional Memory And Lifecycle Audit

Use this when the app feels sluggish, memory grows after repeated UI work, or sync/export/picture changes touch lifecycle-sensitive code.

Static source sweep:

```powershell
rg -n "useEffect|addEventListener|removeEventListener|setInterval|clearInterval|setTimeout|clearTimeout|ResizeObserver|URL\.createObjectURL|invoke\(|listen\(|unlisten|on[A-Z].*Changed" frontend/src backend/src backend/tests
```

Rust retention and file IO sweep:

```powershell
rg -n "Arc|Mutex|RwLock|static|thread|spawn|channel|range_query|collect::<|Vec<|fs::read|File::|Workbook" backend/src backend/tests
```

Manual exercise:

- Start the app and record `me-inventory` plus `msedgewebview2` memory after idle.
- Run repeated search, filter, sort, menu, dialog, picture preview, CRUD, sync idle, and Excel export cycles.
- Record memory after the exercise and again after idle.
- Close the app and confirm app-owned processes exit.
- Keep profiler output, screenshots, traces, workbooks, and app-data backups out of commits unless a specific evidence file should become durable documentation.

## Open Work

- Run real shared-drive multi-machine `1.0.0` update smoke from installed `0.9.9`.
- Add conflict UI, locked-file smoke, and shared media storage.
- Decide whether entries should move from the current compatibility projection to future `inventory:item:*` and ledger keyspaces.
- Benchmark real inventory size for search, sort, startup, sync, and table rendering.
- Add FeOxDB schema versioning and a future migration path.
- Confirm UNC picture path behavior in a packaged smoke.
- Keep HTML export as an explicit placeholder unless it becomes a real requirement.
- Keep `com.me.inventory`; do not change the Tauri identifier without a migration plan.
