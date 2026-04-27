# Memory Leak Audit Runbook

This runbook is the checklist for auditing memory growth in the Tauri v2 inventory app. It is documentation-only: do not change source code during the audit pass, and do not delete generated artifacts unless the manager explicitly assigns cleanup.

## Scope

- React inventory shell, dialogs, table virtualization, menus, and desktop bridge subscriptions.
- Tauri command handlers, FeOxDB access, legacy SQLite import, native file/picture helpers, Excel export, and updater scaffold.
- Validation commands and repeatable manual exercises that can show listener, timer, observer, process, file, or retained-data leaks.
- Generated artifact handling so profiler output, screenshots, logs, and temporary build/test data stay out of source control.

## Audit Flow

1. Record the starting worktree with `git status --short`; note any files owned by other workers and leave them untouched.
2. Run the static source sweep from the validation mapping to find listeners, timers, observers, async subscriptions, large clones, and native file reads.
3. Run targeted automated validation for the source areas under review.
4. Run the manual leak exercise against a disposable app data directory or backed-up test profile.
5. Record baseline, peak, and post-close memory evidence. Keep raw generated artifacts under ignored artifact paths.
6. File findings with exact source paths, reproduction steps, memory evidence, and the validation commands that were run.
7. Hand cleanup candidates to the manager. Workers should not delete app data, build output, screenshots, traces, heap snapshots, or generated test files on their own.

## Validation Command Mapping

| ID | Goal | Evidence to record |
| --- | --- | --- |
| `baseline` | Worktree and ownership baseline. | Modified/untracked files before audit and which ones are outside this worker's ownership. |
| `source-sweep` | Cleanup-sensitive source sweep. | Source rows that need explicit cleanup or lifecycle review. |
| `rust-sweep` | Rust retention and file IO sweep. | Native code paths that can retain large collections, handles, or generated files. |
| `lint` | Frontend lint and hook hygiene. | ESLint result, especially React hook dependency and unused cleanup issues. |
| `vitest-all` | Frontend unit/regression coverage. | Vitest result for React, filtering, URL, bridge, and dialog behavior. |
| `vitest-dialog` | Entry dialog leak-prone paths. | Picture browse/preview/open, media query subscription, and dialog close behavior. |
| `vitest-shell` | Inventory shell leak-prone paths. | Desktop sync interval, updater callback, status timeout, CRUD refresh, and fallback behavior. |
| `vitest-table` | Virtualized table behavior. | Table rendering and row interaction coverage after scroll/sort changes. |
| `build` | Type and production build validation. | TypeScript project build and Vite production build result. |
| `cargo-test` | Rust command/unit coverage. | Command, query, import, native helper, export, and updater scaffold test result. |
| `tauri-dev` | Desktop runtime exercise. | Manual leak exercise result, app process IDs, and memory samples. |
| `tauri-build` | Packaged runtime exercise when release risk is in scope. | Build result, installer path, and whether packaged app memory behavior matches dev runtime. |
| `process-sample` | App process memory sample. | Baseline, post-exercise, idle-after-exercise, and post-close process memory. |
| `webview-sample` | WebView2 child process sample. | Whether WebView2 children keep growing or remain alive after app close. |
| `artifact-inventory` | Generated artifact inventory without deletion. | Paths to hand to the manager for cleanup decisions. |

### Command Details

```powershell
# baseline
git status --short
```

```powershell
# source-sweep
rg -n "useEffect|addEventListener|removeEventListener|setInterval|clearInterval|setTimeout|clearTimeout|ResizeObserver|URL\\.createObjectURL|invoke\\(|listen\\(|unlisten|on[A-Z].*Changed" src src-tauri/src src-tauri/tests
```

```powershell
# rust-sweep
rg -n "Arc|Mutex|RwLock|static|thread|spawn|channel|range_query|collect::<|Vec<|fs::read|File::|Workbook|Connection::open" src-tauri/src src-tauri/tests
```

```powershell
# lint
bun run lint
```

```powershell
# vitest-all
bun run test
```

```powershell
# vitest-dialog
bun run test -- src/test/entry-dialog.test.tsx
```

```powershell
# vitest-shell
bun run test -- src/test/inventory-shell.test.tsx
```

```powershell
# vitest-table
bun run test -- src/test/inventory-table.test.tsx
```

```powershell
# build
bun run build
```

```powershell
# cargo-test
Push-Location src-tauri
cargo test
Pop-Location
```

```powershell
# tauri-dev
bun tauri dev
```

```powershell
# tauri-build
bun tauri build --bundles nsis
```

```powershell
# process-sample
Get-Process me-inventory -ErrorAction SilentlyContinue |
  Select-Object Id,ProcessName,
    @{Name='WorkingSetMB';Expression={[math]::Round($_.WorkingSet64 / 1MB, 1)}},
    @{Name='PrivateMB';Expression={[math]::Round($_.PrivateMemorySize64 / 1MB, 1)}}
```

```powershell
# webview-sample
Get-Process msedgewebview2 -ErrorAction SilentlyContinue |
  Sort-Object WorkingSet64 -Descending |
  Select-Object -First 10 Id,ProcessName,
    @{Name='WorkingSetMB';Expression={[math]::Round($_.WorkingSet64 / 1MB, 1)}},
    @{Name='PrivateMB';Expression={[math]::Round($_.PrivateMemorySize64 / 1MB, 1)}}
```

```powershell
# artifact-inventory
Get-ChildItem -Force .tmp,coverage,test-results,playwright-report,blob-report,src-tauri\target\tmp -ErrorAction SilentlyContinue
```

## Manual Leak Exercise

Use a disposable profile, VM snapshot, or backed-up app data directory. Record the app data path before starting.

1. Start the app and let it idle for 30 seconds. Record `me-inventory` and `msedgewebview2` memory samples.
2. Run 30 search/filter/sort cycles across Inventory and Archive. Return to an empty search state and sample memory.
3. Open and close the Add Entry dialog 20 times. Include Escape, backdrop click, Cancel, and Save validation failure paths.
4. In Edit Entry, change picture paths rapidly between empty, missing, valid small image, and oversized image values. Confirm preview state settles and sample memory.
5. Open and close the column menu, export menu, context menu, and delete confirmation 20 times each. Confirm document listeners do not multiply.
6. Toggle verified, archive, restore, and delete on disposable entries. Confirm refreshes complete and sample memory.
7. Let the app idle for at least two sync intervals. Confirm only one sync interval is active by behavior and memory trend.
8. Export Excel to an ignored temporary path five times. Do not commit the workbooks; list them for manager cleanup.
9. Close the app. Confirm `me-inventory` exits and WebView2 child processes tied to the app do not remain.

## Source Audit Checklist

| Source area | Leak or retention risk | Checklist | Validation mapping |
| --- | --- | --- | --- |
| `src/components/inventory/InventoryShell.tsx` | Status timeout, sync interval, updater/shared callbacks, in-flight async query state, repeated large entry arrays. | Confirm every timeout/interval/subscription has cleanup, stale async results are ignored after unmount, sync cannot overlap indefinitely, and refresh paths do not retain old entry arrays. | `bun run test -- src/test/inventory-shell.test.tsx`, app process memory sample, WebView2 sample. |
| `src/components/inventory/EntryDialog.tsx` | Document keydown listener, media query listener, async picture preview loads, base64 data URL state, repeated open/close cycles. | Confirm keydown and media listeners unregister, preview async results are ignored after close/path change, no object URLs are created without revocation, and large previews are bounded. | `bun run test -- src/test/entry-dialog.test.tsx`, manual picture preview cycle. |
| `src/components/inventory/InventoryTable.tsx` | `ResizeObserver`, scroll state churn, virtual row slicing, large table renders. | Confirm observer disconnects, visible row range stays bounded, scrolling does not append retained row state, and column changes do not force persistent duplicate arrays. | `bun run test -- src/test/inventory-table.test.tsx`, manual search/filter/sort cycle. |
| `src/components/inventory/EntryContextMenu.tsx`, `src/components/inventory/ColumnMenu.tsx`, `src/components/inventory/InventoryHeader.tsx` | Document mousedown/keydown listeners for transient menus. | Confirm listeners are registered only while menu UI is mounted and always removed on close/unmount. | Cleanup-sensitive source sweep, menu open/close manual cycle. |
| `src/lib/tauriInventoryBridge.ts` and `src/types/desktop-bridge.d.ts` | Desktop callback contract can leak if future Tauri `listen` calls do not return/unregister unlisten handlers. | Confirm each bridge event helper returns a cleanup function and future real listeners call the Tauri unlisten handle. | Cleanup-sensitive source sweep, `bun run test`. |
| `src/lib/inventory.ts` and `src/lib/externalUrl.ts` | Pure transforms can allocate large temporary arrays or strings during repeated filter/sort/link operations. | Confirm helpers remain pure, bounded by current query limits, and do not close over growing state. | `bun run test`, manual search/filter/sort cycle. |
| `src-tauri/src/lib.rs` | App lifecycle and managed `InventoryDb` state can leave resources unflushed on exit. | Confirm managed state is created once, command handlers share that state, and exit flush remains wired. | `Push-Location src-tauri; cargo test; Pop-Location`, post-close process sample. |
| `src-tauri/src/store.rs` | FeOxDB store is held in `Arc`; range scans can load all entries; flush behavior affects retained/native state. | Confirm store is singleton app state, load paths are bounded by audit expectations, and no additional long-lived clones are introduced. | Rust retention sweep, `cargo test`, manual query cycle. |
| `src-tauri/src/query.rs` | Filtering clones entries into temporary vectors and sorts in memory. | Confirm `MAX_QUERY_LIMIT` is enforced, offsets do not retain skipped entries beyond the query, and growth is acceptable for current dataset size. | `cargo test`, manual search/filter/sort cycle. |
| `src-tauri/src/commands.rs` | CRUD paths load/find entries and flush after mutation; sync scaffold can become a future background source. | Confirm no background task is spawned, mutation results do not retain old vectors, and sync remains non-overlapping from the frontend. | `cargo test`, manual CRUD and idle sync cycle. |
| `src-tauri/src/legacy_import.rs` | SQLite connection, prepared statements, and imported row iteration can retain source rows or handles. | Confirm import streams rows through the iterator, does not collect the full legacy DB unnecessarily, and idempotent marker prevents repeat imports. | `cargo test`, first-run import smoke if in scope. |
| `src-tauri/src/native.rs` | Picture preview validates source files and copies preview files into the app cache for asset-protocol display. | Confirm extension/path validation, max preview byte guard, missing/oversized file handling, and no native file handles remain open. | `cargo test`, manual picture preview cycle. |
| `src-tauri/src/export.rs` | Workbook generation creates large workbook structures and temporary output files. | Confirm workbook is scoped per export, files are saved only to selected/ignored paths, and repeated export does not leave process memory climbing. | `cargo test`, repeated Excel export cycle. |
| `src-tauri/src/updater.rs` and `src-tauri/tests/updater_scaffold.rs` | Future updater downloads/installers can retain progress state, temp artifacts, or child processes. | Confirm current scaffold does not spawn, download, or keep installer paths; future real updater work must add cleanup tests. | `cargo test`, updater scaffold smoke. |
| `docs/sync-architecture-next-slice.md` future sync paths | Planned shared ops, snapshots, locks, backups, and temp files can accumulate generated artifacts. | Confirm future sync work writes temp files to ignored/generated paths until promoted, and cleanup is manager-owned. | Generated artifact inventory, sync implementation tests when added. |

## Cleanup Policy

- Do not delete files during the audit unless the manager explicitly delegates cleanup.
- Place new audit output under `.tmp/memory-leak-audit/` or `docs/memory-leak-audit/artifacts/`; both are ignored generated-artifact locations.
- Treat heap snapshots, CPU profiles, screenshots, trace files, exported workbooks, FeOxDB files, SQLite copies, and app data backups as generated artifacts unless a manager asks to promote a specific evidence file.
- If an artifact may contain inventory data, paths, user names, screenshots, or database records, mark it sensitive in the audit notes and keep it out of commits by default.
- When cleanup is needed, provide the manager a list of paths, why each was generated, and whether it is safe to remove. Do not remove production app data or installer output without explicit confirmation.
- Do not edit source code as part of a docs-only audit pass. Findings should be documented with reproduction steps and handed back for a scoped implementation task.

## Generated Artifact Exclusions

The repo should ignore generated audit/test outputs, including:

- `.tmp/`
- `coverage/`, `.nyc_output/`, `.vitest/`
- `test-results/`, `playwright-report/`, `blob-report/`
- `*.heapsnapshot`, `*.heapprofile`, `*.cpuprofile`, `*.trace`, `*.speedscope.json`
- `docs/memory-leak-audit/artifacts/`
- generated GUI smoke screenshots and active-state text files

Keep durable Markdown findings in `docs/` when they are meant to be reviewed or committed.
