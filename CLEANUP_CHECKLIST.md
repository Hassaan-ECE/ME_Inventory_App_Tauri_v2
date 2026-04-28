# Cleanup Checklist

Last updated: 2026-04-28

Read `AGENT_RUNBOOK.md` before starting cleanup work. The runbook records command pivots, known blockers, worker rules, and troubleshooting notes. Update it whenever an agent hits a new trap or finds a better route.

## Status Legend

- `[ ]` not started
- `[~]` in progress
- `[x]` done
- `[!]` blocked or needs decision

## Current Snapshot

- [x] README/doc consolidation is intentional.
- [x] Frontend shell/dialog cleanup is complete.
- [x] Frontend validation passed after integration.
- [x] New extracted folders and coordination docs are staged with their importing files.
- [x] Rust formatting, check, and test baselines were captured after `rustfmt` became available.
- [!] The Bun PowerShell shim still resolves to a stale npm wrapper; keep using the direct Bun binary.

### Current Worktree Baseline

```text
 D PORTING_TODO.md
 M README.md
 D docs/MEMORY_LEAK_AUDIT.md
 D docs/desktop-smoke-checklist.md
 D docs/installer-smoke-2026-04-26-worker-a.md
 D docs/manual-smoke-2026-04-26-syed.md
 D docs/post-picture-fix-smoke-2026-04-26.md
 D docs/release-qa-2026-04-26-manager.md
 D docs/sync-architecture-next-slice.md
 D guidelines.md
 M src/components/inventory/EntryDialog.tsx
 M src/components/inventory/InventoryShell.tsx
?? AGENT_RUNBOOK.md
?? CLEANUP_CHECKLIST.md
?? src/components/inventory/entry-dialog/
?? src/components/inventory/shell/
```

## Current Constraints

- [x] Preserve current behavior unless the user approves a behavior change.
- [x] Preserve existing README/doc consolidation changes.
- [x] Keep this checklist current before work starts, after each worker finishes, after validation, and before final handoff.
- [x] Use direct Bun binary until the PowerShell shim is fixed: `C:\Users\Syed.H.Shah\.bun\bin\bun.exe`.
- [x] `cargo fmt -- --check` now passes with `rustfmt` installed.
- [x] Record failed attempts and pivots in `AGENT_RUNBOOK.md`.

## Phase 0: Coordination

- [x] Create this checklist.
- [x] Create `AGENT_RUNBOOK.md`.
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

- [x] Worker A / Ohm owns `src/components/inventory/InventoryShell.tsx`.
- [x] Worker A owns new shell-only helper/component files under `src/components/inventory/shell/`.
- [x] Worker A must not touch `src/components/inventory/EntryDialog.tsx`.

## Phase 2: Entry Dialog Cleanup

- [x] Extract entry form helpers from `EntryDialog.tsx`.
- [x] Extract picture preview/open helpers from `EntryDialog.tsx`.
- [x] Extract picture preview card, dialog actions, and metadata row components.
- [x] Keep public `EntryDialog` props and behavior unchanged.
- [x] Run targeted dialog tests.

### Worker B Scope

- [x] Worker B / Carver owns `src/components/inventory/EntryDialog.tsx`.
- [x] Worker B owns new dialog-only helper/component files under `src/components/inventory/entry-dialog/`.
- [x] Worker B must not touch `src/components/inventory/InventoryShell.tsx`.

## Phase 3: Integration And Review

- [x] Review worker diffs for scope, behavior drift, and naming consistency.
- [x] Resolve import/path conflicts.
- [x] Run lint, targeted tests, full frontend tests, and build.
- [x] Run `git diff --check`.
- [x] Update checklist with completed work, validation results, blockers, and next recommended slice.

## Phase 4: Checkpoint Current Work

- [~] Stage or commit README/doc consolidation.
- [~] Stage or commit frontend cleanup and extracted folders.
- [~] Include `AGENT_RUNBOOK.md` and `CLEANUP_CHECKLIST.md` when staging.
- [x] Re-run frontend validation after staging.
- [x] Confirm generated artifacts are not staged.
- [x] Confirm imported files and extracted modules are staged together.

## Phase 5: Tooling Cleanup

- [x] Document the broken Bun PowerShell shim and confirm it still resolves before the real Bun install.
- [x] Keep the direct Bun binary command as fallback until the shim is fixed.
- [x] Install or confirm `rustfmt`.
- [x] Take Rust validation baseline: `cargo fmt -- --check`, `cargo check`, and `cargo test`.
- [x] Record expected Rust command runtimes in `AGENT_RUNBOOK.md`.
- [x] Consider wrapper scripts for frontend and Rust validation commands; deferred to avoid adding automation before the current checkpoint lands.

## Phase 6: Dead Code And Deferred Features

- [ ] Inventory placeholders, scaffolds, disabled paths, and deferred features.
- [ ] Review HTML export placeholder and decide whether it stays or is removed until needed.
- [ ] Review updater no-op/event scaffolding.
- [ ] Review mock/browser fallback paths and keep only intentional dev/demo behavior.
- [ ] Review stale release/version references after the README consolidation.
- [ ] Remove dead code only when tests prove behavior stayed stable.

## Phase 7: Remaining Frontend Restructure

- [ ] Review `InventoryTable.tsx` for smaller helpers or components.
- [ ] Review `InventoryHeader.tsx` for menu/action extraction.
- [ ] Review `src/lib/inventory.ts` for testable helper boundaries.
- [ ] Split oversized frontend tests if they slow future edits, especially `inventory-shell.test.tsx`.
- [ ] Preserve visible UI behavior, public props, localStorage keys, and Tauri bridge calls.
- [ ] Run targeted frontend tests after each slice.

## Phase 8: Rust Backend Restructure

- [ ] Split `sync.rs` by identity, operation files, scanning, apply/merge, conflicts/tombstones, and status.
- [ ] Split `store.rs` by entry CRUD, metadata, indexes, sync state, and test helpers.
- [ ] Keep FeOxDB key names stable.
- [ ] Keep Tauri command contracts stable.
- [ ] Keep shared operation file format stable.
- [ ] Run Rust validation after each backend slice.

## Phase 9: Native, Export, Import, And Updater Cleanup

- [ ] Review `export.rs` for workbook-format/helper extraction.
- [ ] Review `legacy_import.rs` for schema detection and mapping boundaries.
- [ ] Review `native.rs` for URL/path/picture preview helper boundaries.
- [ ] Review `updater.rs` for manifest, cache, hash, and install boundaries.
- [ ] Preserve Excel workbook sheet contract.
- [ ] Preserve URL/path safety behavior.
- [ ] Preserve legacy SQLite import behavior.
- [ ] Keep custom shared-drive updater cleanup separate from signed Tauri updater decisions.

## Phase 10: Tests And Smoke

- [ ] Split oversized sync tests after Rust module boundaries settle.
- [ ] Keep one-machine sync smoke documented and runnable.
- [ ] Add or retain packaged smoke checklist before release.
- [ ] Track validation commands and results after every cleanup slice.
- [ ] Record any skipped validation with the reason.

## Phase 11: Architecture And Release Decisions

- [x] Reconcile source version for the `0.9.7` release checkpoint.
- [ ] Decide whether to keep the custom shared-drive updater or move to signed Tauri updater.
- [ ] Decide when to add sync snapshots, manifest compaction, conflict UI, and shared media storage.
- [ ] Decide whether and when to move from compatibility `InventoryEntry` records to future ledger/item keyspaces.
- [ ] Decide whether the Tauri identifier should ever change.

## Worker Policy

- [x] Use GPT-5.5 xhigh workers for independent cleanup slices when requested.
- [x] Assign strict file ownership before spawning workers.
- [x] Manager owns checklist updates, integration, validation, and final review.
- [x] Spawn a read-only reviewer for large combined diffs.
- [ ] Record each future worker assignment and result here.

## Worker Log

- [x] Worker A / Ohm: Inventory shell cleanup, GPT-5.5 xhigh, completed. Reported targeted shell tests pass, 1 file / 23 tests, and lint pass.
- [x] Worker B / Carver: Entry dialog cleanup, GPT-5.5 xhigh, completed. Reported targeted dialog tests pass, 1 file / 6 tests, lint pass, and scoped diff check pass.
- [x] Worker C / Locke: Read-only integration review, GPT-5.5 xhigh, completed. No code findings; noted that new extracted folders must be included when committing.
- [~] Worker D / Nash: Phase 4 checkpoint audit, GPT-5.5 xhigh, running.
- [~] Worker E / Archimedes: Phase 5 tooling investigation, GPT-5.5 xhigh, running.

## Validation Results

- Baseline `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Baseline `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- src/test/inventory-shell.test.tsx src/test/entry-dialog.test.tsx`: pass, 2 files / 29 tests.
- Baseline `& "$env:USERPROFILE\.bun\bin\bun.exe" run test`: pass, 6 files / 55 tests.
- Baseline `& "$env:USERPROFILE\.bun\bin\bun.exe" run build`: pass.
- Integrated `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Integrated `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- src/test/inventory-shell.test.tsx src/test/entry-dialog.test.tsx`: pass, 2 files / 29 tests.
- Integrated `& "$env:USERPROFILE\.bun\bin\bun.exe" run test`: pass, 6 files / 55 tests.
- Integrated `& "$env:USERPROFILE\.bun\bin\bun.exe" run build`: pass.
- Integrated `git diff --check`: pass, with CRLF normalization warnings only.
- Phase 4 checkpoint `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Phase 4 checkpoint `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- src/test/inventory-shell.test.tsx src/test/entry-dialog.test.tsx`: pass, 2 files / 29 tests.
- Phase 4 checkpoint `& "$env:USERPROFILE\.bun\bin\bun.exe" run test`: pass, 6 files / 55 tests.
- Phase 4 checkpoint `& "$env:USERPROFILE\.bun\bin\bun.exe" run build`: pass, with Vite plugin timing warning only.
- Phase 4 checkpoint `git diff --cached --check`: pass.
- Phase 4 checkpoint `git diff --check`: pass, with CRLF normalization warnings only for unstaged Rust updater files.
- Phase 5 `cargo fmt -- --check`: pass, 0.73s.
- Phase 5 `cargo check`: pass, 13.54s.
- Phase 5 `cargo test`: pass, 182.76s, with four `dead_code` warnings in the `updater_scaffold` test target.
- Release `0.9.7` `& "$env:USERPROFILE\.bun\bin\bun.exe" run lint`: pass.
- Release `0.9.7` `& "$env:USERPROFILE\.bun\bin\bun.exe" run test -- src/test/inventory-shell.test.tsx src/test/entry-dialog.test.tsx`: pass, 2 files / 29 tests.
- Release `0.9.7` `& "$env:USERPROFILE\.bun\bin\bun.exe" run test`: pass, 6 files / 55 tests.
- Release `0.9.7` `& "$env:USERPROFILE\.bun\bin\bun.exe" run build`: pass, with Vite plugin timing warning only.
- Release `0.9.7` `cargo fmt -- --check`: pass, 1.45s.
- Release `0.9.7` `cargo check`: pass, 33.67s.
- Release `0.9.7` `cargo test`: pass, 247.44s, with four `dead_code` warnings in the `updater_scaffold` test target.
- Release `0.9.7` `bun tauri build --bundles nsis`: first attempt hit Windows os error 1224 during NSIS bundling; retry after deleting only the generated `0.9.7` installer passed in 103.88s.
- Release `0.9.7` shared updater manifest: published `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\current.json` pointing to `releases/0.9.7/ME Inventory Setup 0.9.7.exe` with SHA-256 `31ccbe6a1a86e12bdb35834bd6e3d900afd4a6e945d0ec2f816220257e22583a`.
- Release `0.9.7` `git diff --cached --check`: pass.
- Release `0.9.7` `git diff --check`: pass, with CRLF normalization warnings only.

## Next Recommended Slice

- Keep the staged checkpoint intact until it is committed or intentionally split.
- Decide whether to fix the stale Bun shim globally or continue using the direct binary fallback.
- Start Phase 6 or Phase 7 for another low-risk frontend cleanup slice, or start Rust cleanup now that the Rust baseline is available.
- Use `AGENT_RUNBOOK.md` to avoid repeating known command/tooling traps.
