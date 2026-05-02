# ME Inventory

Last consolidated: 2026-05-02

ME Inventory is a Windows desktop inventory app built with Tauri 2, React 19, TypeScript, Vite, Tailwind CSS v4, Bun, Rust, and FeOxDB.

Built by Syed Hassaan Shah.

This README is the current project entry point. Detailed engineering notes live under `docs/engineering/`; release evidence and audit notes there may be historical, so prefer this file and `docs/engineering/CODE_BEHAVIOR_REMEDIATION_CHECKLIST.md` for current handoff state.

## Current Source Truth

- Active workspace: `c:\Projects\Active\ME_Inventory_App_Tauri_v2`
- App name: `ME Inventory`
- Display name: `ME Inventory v1.0.1`
- Version source: `package.json`, `backend\Cargo.toml`, and `backend\tauri.conf.json`
- Tauri identifier: `com.me.inventory`
- Install mode: current-user NSIS install
- Updater: signed Tauri updater with GitHub Releases static metadata
- Runtime database: local FeOxDB file named `inventory.feox`
- Shared sync: S-drive FeOx operation logs plus manifest/snapshot bootstrap
- Deprecated local `.db` files: quarantined once into app-data backups and never used as data sources

Version note: `1.0.1` is the current source truth for the signed updater target. `1.0.0` is the expected updater baseline for installed-machine smoke.

## Project Layout

```text
frontend/     React/Vite UI, frontend tests, UI assets, and Tauri bridge code
backend/      Tauri/Rust app, commands, storage, sync, export, import, and native helpers
docs/         Engineering runbooks, cleanup checklists, and performance baselines
scripts/      Smoke/manual automation scripts
```

## Doc Map

- `README.md`: current setup, runtime behavior, release checklist, and open work.
- `docs/engineering/CODE_BEHAVIOR_REMEDIATION_CHECKLIST.md`: active hardening status and remaining release gates.
- `docs/engineering/SYNC_RECOVERY_INVARIANTS.md`: local sync recovery rules.
- `docs/engineering/FEOXDB_SYNC_MIGRATION_PLAN.md`: FeOxDB shared-sync design and acceptance checklist.
- `docs/engineering/CODE_BEHAVIOR_AUDIT.md`: historical source audit that started the hardening pass.
- `docs/engineering/CLEANUP_CHECKLIST.md` and `DONE_CHECKLIST.md`: historical release evidence and cleanup logs.

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
- UNC image paths remain allowed on Windows when they are absolute, point to a supported image file, and pass the same size and magic-byte checks as drive-letter paths.
- Preview caching intentionally uses the selected path plus file metadata as the cache fingerprint and does not canonicalize the source path first. Windows reparse points and symlinks are handled by the operating system path lookup; the app still validates the resolved file metadata and image signature before copying into app cache.
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
- Shared sync can require operation, snapshot, and manifest HMAC authentication by setting `ME_INVENTORY_SYNC_HMAC_KEY` to the same 16+ byte secret on every trusted client. When this variable is set, unsigned or mismatched shared files are rejected. When it is not set, the S-drive remains a trusted-write boundary enforced by Windows/share ACLs.
- HMAC is optional hardening for the current trusted-lab release, not a replacement for share permissions. Make it required before release if IT cannot confirm that only trusted users and trusted machines can write to the shared root.
- HMAC keys are distributed and rotated outside this repo. Put the same current key on every trusted client, wait for all clients to converge, then rotate by changing every client to the new key during the same maintenance window. Mixed-key clients fail closed against each other's new files until the rotation is complete.
- Last-write-wins entry state uses `(mutation_ts_utc, op_id)` ordering.
- Remote timestamps must be valid RFC3339 UTC timestamps. Old and future UTC timestamps are accepted because offline machines may reconnect later; clock skew directly affects last-write-wins ordering and should be corrected at the workstation/domain level instead of hidden in sync code.
- Deletes create tombstones.
- Older operations are skipped and logged as conflicts.
- Newer upserts after a tombstone restore the entry.
- Repeated syncs are intended to be idempotent.
- The native watcher emits `inventory:shared-changed`; the frontend coalesces sync work so overlapping sync passes do not stack up.
- Open clients also run a 500ms fallback sync poll so S-drive changes still land quickly when the network filesystem watcher misses a remote change.
- The frontend status reports local readiness, shared-root availability, local-only pending state, mutation mode, revision, and last snapshot id.
- The 1.x storage/query target is modest lab inventory scale, up to 10,000 rows per query. Larger deployments should run the ignored backend/frontend performance baselines before release and move filtering/sorting closer to indexed storage if the baseline is not acceptable.

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
node scripts\run-bun.mjs install
```

Run the web UI:

```powershell
node scripts\run-bun.mjs run dev
```

Run the Tauri desktop app:

```powershell
node scripts\run-bun.mjs run dev:desktop
```

Build the frontend:

```powershell
node scripts\run-bun.mjs run build
```

Run frontend tests:

```powershell
node scripts\run-bun.mjs run test
```

Run dependency audits:

```powershell
node scripts\run-bun.mjs audit
cd backend; cargo audit
```

Run the one-machine shared-sync smoke:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\smoke-sync-one-machine.ps1
```

Build the Windows NSIS installer:

```powershell
node scripts\run-bun.mjs run build:desktop
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

The global Bun PowerShell shim can resolve to a stale wrapper on this workstation. Use the repo Bun launcher for release validation until the shim is fixed:

```powershell
node scripts\run-bun.mjs run lint
node scripts\run-bun.mjs run test
node scripts\run-bun.mjs run build
node scripts\run-bun.mjs audit

Push-Location backend
cargo fmt -- --check
cargo check
cargo test
cargo audit
Pop-Location
```

`cargo audit` requires `cargo install cargo-audit`. Clippy is also a release gate once installed with `rustup component add clippy`:

```powershell
Push-Location backend
cargo clippy --all-targets -- -D warnings
Pop-Location
```

For signed updater releases, build with the updater private key available outside the repo. The current local key path is `%USERPROFILE%\.tauri\me-inventory-updater.key`; keep that private key out of the repo.

```powershell
$env:PATH = "$env:USERPROFILE\.bun\bin;$env:PATH"
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content -LiteralPath "$env:USERPROFILE\.tauri\me-inventory-updater.key" -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
node scripts\run-bun.mjs run build:desktop
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

Shared-release staging uses `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases`. Before replacing release artifacts, back up any current shared metadata or installer folders that are still needed for rollback evidence.

Publish the generated NSIS installer, its `.sig` file, SHA-256 sums, and a GitHub Release asset named `latest.json`. If `gh` is unavailable on the workstation, upload the staged files manually to a non-draft, non-prerelease GitHub Release. Static updater metadata must include the Tauri updater fields for the Windows platform:

```json
{
  "version": "1.0.1",
  "notes": "Release notes",
  "pub_date": "2026-05-02T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "contents of the generated .sig file",
      "url": "https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/download/v1.0.1/ME%20Inventory_1.0.1_x64-setup.exe"
    }
  }
}
```

Fresh `1.0.1` manual smoke:

- Confirm `package.json`, `backend\Cargo.toml`, and `backend\tauri.conf.json` versions match.
- Confirm the app identifier is still `com.me.inventory`.
- Update an installed `1.0.0` machine to `1.0.1`.
- Launch from the installed shortcut.
- Confirm the visible name and version are `ME Inventory v1.0.1`.
- On clean app data, confirm startup hydrates from the S-drive FeOx snapshot and newer operation files.
- Close and reopen, then confirm row count stays stable.
- Add, edit, verify, archive, restore, and delete a disposable smoke entry.
- Save and open a safe `https://` link.
- Run `Search Online`.
- Select a local picture path with spaces, confirm preview, then open it.
- Confirm a missing picture path shows the missing state without crashing.
- Export Excel, cancel once, then save once to a path with spaces.
- Open the workbook and confirm exactly `Inventory` and `Archive` sheets.
- Upload the staged `1.0.1` GitHub Release assets, then from the installed `1.0.0` app confirm update check, download progress, install, and relaunch/update behavior.
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

- Run real shared-drive multi-machine `1.0.1` update smoke from installed `1.0.0`.
- Add conflict UI, locked-file smoke, and shared media storage.
- Decide whether entries should move from the current compatibility projection to future `inventory:item:*` and ledger keyspaces.
- Benchmark real inventory size for search, sort, startup, sync, and table rendering.
- Add FeOxDB schema versioning and a future migration path.
- Confirm UNC picture path behavior in a packaged smoke.
- Keep HTML export as an explicit placeholder unless it becomes a real requirement.
- Keep `com.me.inventory`; do not change the Tauri identifier without a migration plan.
