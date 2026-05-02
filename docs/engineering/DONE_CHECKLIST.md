# Done Checklist

Last updated: 2026-04-30

Historical archive of completed cleanup work, old release notes, worker logs, validation baselines, and deferred roadmap captured before the fresh `0.9.x` release-smoke reset. Commands, versions, and paths in this file are evidence from past work, not current instructions. Current release blockers live in `docs/engineering/CLEANUP_CHECKLIST.md`; current hardening status lives in `docs/engineering/CODE_BEHAVIOR_REMEDIATION_CHECKLIST.md`.

Read `docs/engineering/AGENT_RUNBOOK.md` before starting cleanup work. The runbook records command pivots, known blockers, worker rules, and troubleshooting notes. Update it whenever an agent hits a new trap or finds a better route.

## Status Legend

- `[ ]` not started
- `[~]` in progress
- `[x]` done
- `[!]` blocked or needs decision

## Archived Snapshot

- [x] README/doc consolidation is intentional.
- [x] Frontend shell/dialog cleanup is complete.
- [x] Frontend validation passed after integration.
- [x] New extracted folders and coordination docs are staged with their importing files.
- [x] Rust formatting, check, and test baselines were captured after `rustfmt` became available.
- [!] At this checkpoint, the Bun PowerShell shim still resolved to a stale npm wrapper; the current runbook now points at `scripts\run-bun.mjs`.

### Archived Worktree Baseline

```text
Branch: codex/cleanup-checklist-completion
Committed cleanup slices:
- performance checklist phase
- inventory table component split
- inventory helper module split and broader search parity
- native helper split
- Excel export workbook split
- legacy import helper split
- unused layout component removal
Pending worktree at this checkpoint:
- clean after final validation and closeout commit
```

## Archived Constraints

- [x] Preserve current behavior unless the user approves a behavior change.
- [x] Preserve existing README/doc consolidation changes.
- [x] Keep this checklist updated during the archived cleanup slice.
- [x] Use the direct Bun binary until the PowerShell shim is fixed: `C:\Users\Syed.H.Shah\.bun\bin\bun.exe`.
- [x] `cargo fmt -- --check` now passes with `rustfmt` installed.
- [x] Record failed attempts and pivots in `docs/engineering/AGENT_RUNBOOK.md`.

## Phase 0: Coordination

- [x] Create this checklist.
- [x] Create `docs/engineering/AGENT_RUNBOOK.md`.
- [x] Record current `git status --short`.
- [x] Record validation baseline.
- [x] Spawn GPT-5.5 xhigh workers with disjoint file ownership.
- [x] Use a read-only reviewer worker for the combined frontend diff.

### Validation Baseline

- [x] Direct Bun binary available: `1.3.13`.
- [x] `lint` baseline recorded: pass.
- [x] Targeted shell/dialog tests baseline recorded: pass, 2 files / 29 tests.
- [x] Full frontend test baseline recorded: pass, 6 files / 55 tests.
- [x] Frontend build baseline recorded: pass.

## Phase 1: Frontend Shell Cleanup

- [x] Extract pure shell helpers/constants from `InventoryShell.tsx`.
- [x] Extract delete confirmation dialog from `InventoryShell.tsx`.
- [x] Keep visible UI, localStorage keys, desktop bridge behavior, sync polling, update behavior, and tests unchanged.
- [x] Run targeted shell tests.

### Worker A Scope

- [x] Worker A / Ohm owns `frontend/src/features/inventory/components/InventoryShell.tsx`.
- [x] Worker A owns new shell-only helper/component files under `frontend/src/features/inventory/components/shell/`.
- [x] Worker A must not touch `frontend/src/features/inventory/components/EntryDialog.tsx`.

## Phase 2: Entry Dialog Cleanup

- [x] Extract entry form helpers from `EntryDialog.tsx`.
- [x] Extract picture preview/open helpers from `EntryDialog.tsx`.
- [x] Extract picture preview card, dialog actions, and metadata row components.
- [x] Keep public `EntryDialog` props and behavior unchanged.
- [x] Run targeted dialog tests.

### Worker B Scope

- [x] Worker B / Carver owns `frontend/src/features/inventory/components/EntryDialog.tsx`.
- [x] Worker B owns new dialog-only helper/component files under `frontend/src/features/inventory/components/entry-dialog/`.
- [x] Worker B must not touch `frontend/src/features/inventory/components/InventoryShell.tsx`.

## Phase 3: Integration And Review

- [x] Review worker diffs for scope, behavior drift, and naming consistency.
- [x] Resolve import/path conflicts.
- [x] Run lint, targeted tests, full frontend tests, and build.
- [x] Run `git diff --check`.
- [x] Update checklist with completed work, validation results, blockers, and next recommended slice.

## Phase 4: Checkpoint Current Work

- [x] Stage or commit README/doc consolidation.
- [x] Stage or commit frontend cleanup and extracted folders.
- [x] Include `docs/engineering/AGENT_RUNBOOK.md` and `docs/engineering/CLEANUP_CHECKLIST.md` when staging.
- [x] Re-run frontend validation after staging.
- [x] Confirm generated artifacts are not staged.
- [x] Confirm imported files and extracted modules are staged together.

## Phase 5: Tooling Cleanup

- [x] Document the broken Bun PowerShell shim and confirm it still resolves before the real Bun install.
- [x] Keep the direct Bun binary command as fallback until the shim is fixed.
- [x] Install or confirm `rustfmt`.
- [x] Take Rust validation baseline: `cargo fmt -- --check`, `cargo check`, and `cargo test`.
- [x] Record expected Rust command runtimes in `docs/engineering/AGENT_RUNBOOK.md`.
- [x] Consider wrapper scripts for frontend and Rust validation commands; deferred to avoid adding automation before the current checkpoint lands.

## Phase 6: Dead Code And Deferred Features

- [x] Inventory placeholders, scaffolds, disabled paths, and deferred features.
- [x] Keep HTML export visible as an explicit placeholder with test coverage.
- [x] Replace updater no-op/event scaffolding with signed Tauri updater progress events.
- [x] Review mock/browser fallback paths and keep only intentional dev/demo behavior.
- [x] Review stale release/version references touched by the updater migration.
- [x] Remove dead code only when tests prove behavior stayed stable.

## Phase 7: Remaining Frontend Restructure

- [x] Review `InventoryTable.tsx` for smaller helpers or components.
- [x] Review `InventoryHeader.tsx` for menu/action extraction.
- [x] Review `frontend/src/features/inventory/lib/index.ts` for testable helper boundaries.
- [x] Split oversized frontend tests if they slow future edits, especially `inventory-shell.test.tsx`.
- [x] Preserve visible UI behavior, public props, localStorage keys, and Tauri bridge calls.
- [x] Run targeted frontend tests after each completed slice.

## Phase 8: Performance Optimization

- [x] Capture baseline measurements before changing behavior: `load_inventory`, `query_inventory`, search/filter/sort latency, table scrolling, and memory with realistic inventory data.
- [x] Decide whether the desktop UI should use server-backed `queryInventory` for large datasets before treating FeOxDB query indexes as user-visible performance work.
- [x] Defer cached normalized search text because 10k measured search/filter/sort stayed below threshold.
- [x] Defer app-managed FeOxDB secondary query indexes because measured 10k load/search stayed below threshold.
- [x] Defer index maintenance and rebuild/backfill work until indexes are justified by measurements.
- [x] Keep the current scan path; do not replace it without a future indexed parity/rebuild plan.
- [x] Profile `InventoryTable.tsx` baseline behavior; keep current virtualization and defer broad memoization because render/scroll stayed below threshold.
- [x] Defer `cargo flamegraph` until a Rust hot command crosses baseline thresholds.
- [x] Fall back to timing instrumentation for this baseline; no flamegraph artifact generated.
- [x] Add Rust query parity tests for broader global search and blank sort behavior.
- [x] Defer frontend tests for server-backed query behavior because the UI did not switch from local filtering.
- [x] Record baseline performance notes and run targeted validation.

## Phase 9: Rust Backend Restructure

- [x] Split `sync.rs` by identity, operation files, scanning, apply/merge, conflicts/tombstones, and status.
- [x] Split `store.rs` by entry CRUD, metadata, indexes, sync state, and test helpers.
- [x] Keep FeOxDB key names stable.
- [x] Keep Tauri command contracts stable.
- [x] Keep shared operation file format stable.
- [x] Run Rust validation after each backend slice.

## Phase 10: Native, Export, Import, And Updater Cleanup

- [x] Review `export.rs` for workbook-format/helper extraction.
- [x] Review `legacy_import.rs` for schema detection and mapping boundaries.
- [x] Review `native.rs` for URL/path/picture preview helper boundaries.
- [x] Replace custom shared-drive `updater.rs` with official signed Tauri updater integration.
- [x] Preserve Excel workbook sheet contract.
- [x] Preserve URL/path safety behavior.
- [x] Preserve legacy SQLite import behavior.
- [x] Document that the custom shared-drive updater path was replaced by signed GitHub Releases metadata.

## Phase 11: Tests And Smoke

- [x] Split oversized sync tests after Rust module boundaries settle.
- [x] Keep one-machine sync smoke documented and runnable.
- [x] Add or retain packaged smoke checklist before release.
- [x] Track validation commands and results after every cleanup slice.
- [x] Record any skipped validation with the reason.

## Phase 12: Architecture And Release Decisions

- [x] Reconcile source version for the `0.9.7` release checkpoint.
- [x] Move from the custom shared-drive updater to the official signed Tauri updater.
- [x] Defer sync snapshots, manifest compaction, conflict UI, and shared media storage to separate specs.
- [x] Defer any move from compatibility `InventoryEntry` records to future ledger/item keyspaces to a separate spec.
- [x] Keep `com.me.inventory`; no identifier migration in this cleanup branch.

## Worker Policy

- [x] Use GPT-5.5 xhigh workers for independent cleanup slices when requested.
- [x] Assign strict file ownership before spawning workers.
- [x] Manager owns checklist updates, integration, validation, and final review.
- [x] Spawn a read-only reviewer for large combined diffs.
- [x] Record each future worker assignment and result here.

## Worker Log

- [x] Worker A / Ohm: Inventory shell cleanup, GPT-5.5 xhigh, completed. Reported targeted shell tests pass, 1 file / 23 tests, and lint pass.
- [x] Worker B / Carver: Entry dialog cleanup, GPT-5.5 xhigh, completed. Reported targeted dialog tests pass, 1 file / 6 tests, lint pass, and scoped diff check pass.
- [x] Worker C / Locke: Read-only integration review, GPT-5.5 xhigh, completed. No code findings; noted that new extracted folders must be included when committing.
- [x] Worker D / Nash: Phase 4 checkpoint audit, GPT-5.5 xhigh, completed.
- [x] Worker E / Archimedes: Phase 5 tooling investigation, GPT-5.5 xhigh, completed.
- [x] Worker Schrodinger: Phase 7 table restructure, completed and committed.
- [x] Worker Gibbs: Phase 7 inventory helper split and broader global search parity prep, completed and committed.
- [x] Worker Bernoulli: Phase 6 unused layout cleanup, completed and committed.
- [x] Worker Euler: Phase 10 Excel export cleanup, completed and committed.
- [x] Worker Poincare: Phase 10 legacy import cleanup, completed and committed.
- [x] Worker Lovelace: Phase 10 native helper cleanup, completed and committed.
- [x] Worker Hegel: Phase 9 store split, completed and integrated with sync split.
- [x] Worker Lagrange: Phase 9 sync split, completed and integrated with store split.
- [x] Worker Carver: Phase 11 sync test split, completed and committed.

## Validation Results

- Baseline `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Baseline `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- frontend/tests/inventory-shell.test.tsx frontend/tests/entry-dialog.test.tsx`: pass, 2 files / 29 tests.
- Baseline `& "$env:USERPROFILE\.bun\bin\bun.exe" run test`: pass, 6 files / 55 tests.
- Baseline `& "$env:USERPROFILE\.bun\bin\bun.exe" run build`: pass.
- Integrated `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Integrated `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- frontend/tests/inventory-shell.test.tsx frontend/tests/entry-dialog.test.tsx`: pass, 2 files / 29 tests.
- Integrated `& "$env:USERPROFILE\.bun\bin\bun.exe" run test`: pass, 6 files / 55 tests.
- Integrated `& "$env:USERPROFILE\.bun\bin\bun.exe" run build`: pass.
- Integrated `git diff --check`: pass, with CRLF normalization warnings only.
- Phase 4 checkpoint `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Phase 4 checkpoint `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- frontend/tests/inventory-shell.test.tsx frontend/tests/entry-dialog.test.tsx`: pass, 2 files / 29 tests.
- Phase 4 checkpoint `& "$env:USERPROFILE\.bun\bin\bun.exe" run test`: pass, 6 files / 55 tests.
- Phase 4 checkpoint `& "$env:USERPROFILE\.bun\bin\bun.exe" run build`: pass, with Vite plugin timing warning only.
- Phase 4 checkpoint `git diff --cached --check`: pass.
- Phase 4 checkpoint `git diff --check`: pass, with CRLF normalization warnings only for unstaged Rust updater files.
- Phase 5 `cargo fmt -- --check`: pass, 0.73s.
- Phase 5 `cargo check`: pass, 13.54s.
- Phase 5 `cargo test`: pass, 182.76s, with four `dead_code` warnings in the now-removed custom updater scaffold test target.
- Release `0.9.7` `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Release `0.9.7` `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- frontend/tests/inventory-shell.test.tsx frontend/tests/entry-dialog.test.tsx`: pass, 2 files / 29 tests.
- Release `0.9.7` `& "$env:USERPROFILE\.bun\bin\bun.exe" run test`: pass, 6 files / 55 tests.
- Release `0.9.7` `& "$env:USERPROFILE\.bun\bin\bun.exe" run build`: pass, with Vite plugin timing warning only.
- Release `0.9.7` `cargo fmt -- --check`: pass, 1.45s.
- Release `0.9.7` `cargo check`: pass, 33.67s.
- Release `0.9.7` `cargo test`: pass, 247.44s, with four `dead_code` warnings in the now-removed custom updater scaffold test target.
- Release `0.9.7` `bun tauri build --bundles nsis`: first attempt hit Windows os error 1224 during NSIS bundling; retry after deleting only the generated `0.9.7` installer passed in 103.88s.
- Release `0.9.7` legacy shared updater manifest: published `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\current.json` pointing to `releases/0.9.7/ME Inventory Setup 0.9.7.exe` with SHA-256 `31ccbe6a1a86e12bdb35834bd6e3d900afd4a6e945d0ec2f816220257e22583a`; superseded for future releases by signed Tauri updater metadata on GitHub Releases.
- Release `0.9.7` `git diff --cached --check`: pass.
- Release `0.9.7` `git diff --check`: pass, with CRLF normalization warnings only.
- Release `0.9.8` `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Release `0.9.8` `& "$env:USERPROFILE\.bun\bin\bun.exe" run test`: pass, 7 files / 60 tests, with 1 skipped file / 1 skipped test.
- Release `0.9.8` `& "$env:USERPROFILE\.bun\bin\bun.exe" run build`: pass.
- Release `0.9.8` `cargo fmt -- --check`: pass.
- Release `0.9.8` `cargo check`: pass.
- Release `0.9.8` `cargo test`: pass, 36 unit tests plus integration suites, with the benchmark harness ignored as expected.
- Release `0.9.8` `bun tauri build --bundles nsis`: pass after setting `TAURI_SIGNING_PRIVATE_KEY` to the key contents and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` to an explicit empty string; produced `ME Inventory_0.9.8_x64-setup.exe` and `.sig`.
- Release `0.9.8` local staging: created ignored `release\v0.9.8\` with GitHub `latest.json`, installer, signature, SHA-256 sums, and transitional legacy shared-updater manifest.
- Release `0.9.8` legacy shared updater manifest: backed up the previous shared `current.json`, then published `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\current.json` pointing to `releases/0.9.8/ME Inventory Setup 0.9.8.exe` with SHA-256 `2001b2426fea8f2eb6d3c84f794491e7ff1deff62a1f5c12a23ba95413fcbcd3`.
- Phase 7 table split `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- frontend/tests/inventory-table.test.tsx frontend/tests/inventory-filtering.test.ts`: pass, 2 files / 15 tests.
- Phase 7 helper split `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- frontend/tests/inventory-table.test.tsx frontend/tests/inventory-filtering.test.ts`: pass, 2 files / 15 tests.
- Phase 10 native split `cargo test native::tests`: pass, 8 tests.
- Phase 10 export split `cargo test export::tests`: pass, 2 tests.
- Phase 10 legacy import split `cargo test legacy_import::tests`: pass, 6 tests.
- Signed updater migration `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- frontend/tests/inventory-shell.test.tsx`: pass, 1 file / 24 tests.
- Signed updater migration `cargo check`: pass.
- Signed updater migration `git diff --check`: pass, with CRLF normalization warnings only.
- Phase 8 backend baseline `cargo test --test performance_baseline -- --ignored --nocapture`: pass. 10k median `load_entries` 141.043 ms; 10k median query equivalent 216.612 ms; 10k in-memory query 55.854 ms.
- Phase 8 frontend baseline `RUN_PERF_BASELINE=1` `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- frontend/tests/performance-baseline.test.tsx`: pass. 10k local filter/sort median 3.953 ms; table render median 7.172 ms; scroll update median 4.946 ms.
- Phase 7 header split `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- frontend/tests/inventory-shell.test.tsx`: pass, 1 file / 24 tests.
- Phase 7 header split `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Phase 7 shell test split `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- frontend/tests/inventory-shell.test.tsx frontend/tests/tauri-inventory-bridge.test.ts`: pass, 2 files / 24 tests.
- Phase 7 shell test split `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Phase 6 desktop fallback fix `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- frontend/tests/inventory-shell.test.tsx`: pass, 1 file / 21 tests.
- Phase 6 desktop fallback fix `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Phase 9 store/sync split `cargo fmt -- --check`: pass.
- Phase 9 store/sync split `cargo test store::tests`: pass, 5 tests.
- Phase 9 store/sync split `cargo test --test sync_core`: pass, 15 tests.
- Phase 9 store/sync split `cargo test --test shared_sync_flow`: pass, 21 tests.
- Phase 11 sync test split `cargo test --test shared_sync_flow`: pass, 13 tests.
- Phase 11 sync test split `cargo test --test sync_conflict_flow`: pass, 15 tests.
- Final `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Final `& "$env:USERPROFILE\.bun\bin\bun.exe" run test`: pass, 7 files passed / 1 skipped, 60 tests passed / 1 skipped.
- Final `& "$env:USERPROFILE\.bun\bin\bun.exe" run build`: pass.
- Final `cargo check`: pass.
- Final `cargo test`: pass, 36 unit tests, 12 performance harness tests plus 1 ignored benchmark, 13 shared sync tests, 15 sync conflict tests, 15 sync core tests, and doc tests.
- Final `scripts\smoke-sync-one-machine.ps1`: pass; clients converged, stale update logged as conflict, delete and newer restore succeeded.

## Skipped / Manual Validation

- Packaged NSIS install smoke was not rerun in this cleanup branch; keep it as a manual release checkpoint.
- Real signed GitHub Releases updater install was not run because it requires publishing a newer signed release asset.
- Real shared-drive multi-machine sync and real legacy DB import remain manual release checkpoints.

## Next Recommended Slice

### Release Smoke

- [ ] Run packaged NSIS install smoke before the next release.
- [ ] Validate the signed GitHub Releases updater path with a real newer signed release asset.
- [ ] Run real shared-drive multi-machine sync smoke.
- [ ] Run real legacy SQLite import smoke.

### Environment / Tooling

- [ ] Fix the stale global Bun PowerShell shim outside the repo when convenient.

### Next Release Prep

- [ ] Bump or reconcile the source version when preparing the next release after `0.9.8`.
- [ ] Confirm release assets include the NSIS installer, generated `.sig`, SHA-256 sums, and GitHub `latest.json`.
- [ ] Record manual smoke evidence: installer path, updater `.sig` path, commit, source version, tester, and date.

### Sync Roadmap

- [ ] Finish shared sync snapshots.
- [ ] Add shared sync manifest validation.
- [ ] Add single-writer sync compaction.
- [ ] Add conflict UI.
- [ ] Run locked-file shared sync smoke.
- [ ] Add shared media storage.

### Data / Import Roadmap

- [ ] Add import issue tracking for legacy SQLite import.
- [ ] Add clearer unknown-schema errors for legacy SQLite import.
- [ ] Add FeOxDB schema versioning and a future migration path.

### Performance / Manual QA

- [ ] Benchmark real inventory size for search, sort, startup, sync, and table rendering.
- [ ] Confirm UNC picture path behavior in a packaged smoke.

### Deferred Decisions

- [ ] Decide whether entries should move from the current compatibility projection to future `inventory:item:*` and ledger keyspaces.
- [ ] Keep HTML export as an explicit placeholder unless it becomes a real requirement.
