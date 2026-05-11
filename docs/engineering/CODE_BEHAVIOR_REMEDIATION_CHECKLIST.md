# Code Behavior Remediation Checklist

Date: 2026-05-02

Source audit: `docs/engineering/CODE_BEHAVIOR_AUDIT.md`

This checklist tracks what is still left after the first audit-remediation pass. Work it from top to bottom unless a blocker forces a pivot.

## Status Legend

- [ ] Not started
- [~] In progress or partially implemented
- [x] Complete
- [!] Blocked or needs a decision

## Current Snapshot

- [x] CSP is no longer disabled.
- [x] Tauri capabilities are narrower than the original broad defaults.
- [x] Backend shared sync now has a coordinator that serializes mutation, publish, and manual sync entry points.
- [x] Remote operation timestamps are parsed and validated.
- [x] Backend input bounds exist for quantity, field lengths, links, and picture paths.
- [x] Frontend bridge guards exist for main inventory and mutation payloads.
- [x] Stored column visibility repairs an all-hidden data-column state.
- [x] Excel formula-like text has a regression test.
- [x] Picture previews sniff supported image signatures before caching.
- [x] Startup recovery invariants and failure-injection coverage now cover the known recoverable local and snapshot states.
- [x] Optional shared-sync HMAC has operation, snapshot, manifest, and two-client coverage plus operational docs.
- [~] Tooling gates are partly runnable: Bun-first frontend audit bypasses the stale shim, but `cargo-clippy` and `cargo-audit` still need local installation before release validation.

## One-At-A-Time Work Queue

### 0. Stabilize The Current Diff

- [x] Review the full current worktree diff for accidental scope creep.
- [x] Decide whether the audit remediation should be one commit or split commits.
  - Decision: keep changes grouped by remediation slice if committing manually, unless a single checkpoint commit is requested.
- [x] Make sure all new source/test/docs files are intentionally included:
  - `backend/src/runtime/shared_sync.rs`
  - `backend/src/sync/auth.rs`
  - `backend/src/sync/recovery.rs`
  - `backend/src/sync/timestamps.rs`
  - `frontend/src/integrations/tauri/bridgeGuards.ts`
  - `frontend/tests/columns.test.ts`
  - `frontend/tsconfig.tests.json`
  - `docs/engineering/CODE_BEHAVIOR_AUDIT.md`
  - `docs/engineering/CODE_BEHAVIOR_REMEDIATION_CHECKLIST.md`
  - `docs/engineering/SYNC_RECOVERY_INVARIANTS.md`
  - `scripts/run-bun.mjs`
- [!] Re-run baseline validation after any final cleanup:
  - `node scripts\run-bun.mjs run lint`
  - `node scripts\run-bun.mjs run test`
  - `node scripts\run-bun.mjs run build`
  - `cd backend; cargo fmt -- --check`
  - `cd backend; cargo test`
  - Status: partial validation only. Passed `node scripts\run-bun.mjs --version`, `node scripts\run-bun.mjs run lint`, `node scripts\run-bun.mjs run build`, `cd backend; cargo fmt -- --check`, and `cd backend; cargo check`. Skipped `node scripts\run-bun.mjs run test` and `cd backend; cargo test` by user request because tests are crashing VS Code.
  - 2026-05-02 release pass: also passed `node scripts\run-bun.mjs run test -- frontend/tests/tauri-inventory-bridge.test.ts frontend/tests/columns.test.ts`, `cd backend; cargo test --lib shared_sync -- --nocapture`, and `cd backend; cargo test --test sync_core hmac -- --nocapture`. `cd backend; cargo test --test sync_core recovery -- --nocapture` timed out after 5 minutes and was stopped. Full test suites still were not run.

Acceptance criteria:
- No unrelated files are mixed into the audit-remediation commit.
- New modules and tests are committed with the files that import them.
- Baseline validation results are recorded in the final handoff.

### 1. Finish Local Crash-Recovery Invariants

- [x] Write explicit recovery invariants for local mutation and sync metadata.
- [x] Cover create/update/archive/verify/delete local mutation invariants:
  - entry record
  - next entry id
  - outbox record
  - applied marker
  - client sequence marker
  - tombstone for deletes
  - entry state
  - sync revision
- [x] Cover snapshot apply invariants:
  - pending snapshot marker
  - entry replacement
  - tombstone replacement
  - entry-state replacement
  - watermark replacement
  - bootstrap marker
  - last snapshot id
  - sync revision
- [x] Decide whether recovery should repair, reject, or only report each inconsistent state.
- [x] Extend `recover_local_sync_state` for the chosen repair/report cases.
- [x] Surface recovery results clearly in shared status or startup status.

Acceptance criteria:
- Every multi-step local write path has a documented invariant.
- Recovery behavior is deterministic for known partial states.
- Recovery test code covers the rules; this pass did not run `cargo test` by user request.

### 2. Add Failure-Injection And Partial-State Tests

- [x] Add tests for outbox-only local operation recovery.
- [x] Add tests for applied-marker/client-sequence mismatch recovery.
- [x] Add tests for delete partial states:
  - entry deleted but tombstone missing
  - tombstone exists but entry still exists
  - tombstone exists but entry state missing
- [x] Add tests for update partial states:
  - entry exists but entry state missing or stale
  - outbox operation exists but applied marker missing
- [x] Add tests for interrupted snapshot apply:
  - pending marker present and last snapshot id missing
  - keyspaces partially replaced
  - pending marker is cleared only when the snapshot is fully applied or explicitly abandoned
- [x] Add a regression test that startup recovery does not rewrite a clean store.

Acceptance criteria:
- Tests simulate partial state directly without relying on process crashes.
- Recovery tests prove both repaired state and user-visible recovery message.
- Normal sync/conflict tests still need to be run by the user; they were skipped in this pass by request.

### 3. Prove Shared-Sync HMAC End To End

- [x] Add operation-file tests with HMAC configured:
  - signed operation reads successfully
  - unsigned operation is rejected
  - tampered operation with stale HMAC is rejected
  - wrong HMAC key is rejected
- [x] Add snapshot/manifest tests with HMAC enabled:
  - signed manifest and snapshot apply successfully
  - unsigned manifest is rejected
  - unsigned snapshot is rejected
  - tampered snapshot is rejected
- [x] Add a two-client sync test with HMAC enabled.
- [x] Add docs for the shared-root trust model:
  - ACL-only trusted-write mode when no HMAC key is set
  - HMAC-required mode when the key is set on every client
  - how the key is distributed and rotated outside the repo
- [x] Decide whether HMAC should be optional hardening or required before release.
  - Decision: optional hardening for the trusted-lab release; require it before release if shared-root ACL ownership cannot be verified.

Acceptance criteria:
- HMAC behavior is tested for operations, snapshots, manifests, and two-client sync.
- The app fails closed when HMAC is configured but shared files are unsigned or mismatched.
- README or an engineering doc explains the operational trust model.

### 4. Finish Backend Sync Concurrency Coverage

- [x] Add a command/runtime test for overlapping manual sync and background publish.
- [x] Add a test for mutation-triggered publish while a shared sync is already running.
- [x] Add a test that background publish failure is persisted or surfaced through later `load_inventory` / `query_inventory` status.
- [x] Decide whether the coordinator should serialize only sync/publish or also all inventory mutations.
  - Decision: serialize inventory mutations, manual sync, and background publish through one backend coordinator.
- [x] Document the intended concurrency invariant near the shared-sync coordinator.

Acceptance criteria:
- Tests prove only one backend shared-sync/publish/mutation critical section runs at a time, or the allowed overlap is documented.
- Background publish errors become visible to the UI without waiting for user guesswork.

### 5. Complete Timestamp And Clock-Skew Behavior

- [x] Add tests for old but valid remote timestamps.
- [x] Add tests for far-future remote timestamps.
- [x] Add tests that non-UTC offsets are rejected consistently for create/update/delete operations.
- [x] Document product behavior for clock skew between machines.
- [x] Decide whether future timestamps should be accepted, warned, or rejected beyond a threshold.
  - Decision: accept valid UTC future timestamps; clock skew is operationally visible through last-write-wins ordering and should be fixed at workstation/domain time sync.

Acceptance criteria:
- Malformed timestamps, non-UTC timestamps, old timestamps, future timestamps, and equal timestamp tie-breakers are all covered.
- Clock-skew behavior is documented as a product decision, not an accidental implementation detail.

### 6. Finish Frontend Runtime-Guard Coverage

- [x] Add tests for malformed `queryInventory` responses.
- [x] Add tests for malformed create/update/archive/verify/delete mutation responses.
- [x] Add tests for malformed shared status values.
- [x] Add tests that `InventoryShell` fails closed with a clear status message when bridge parsing rejects a payload.
- [x] Add or consciously defer a runtime guard for `exportExcel` results.
- [x] Confirm update-state normalization still handles malformed updater state safely.

Acceptance criteria:
- Malformed Tauri payloads cannot silently enter React state.
- The UI shows a recoverable error state instead of crashing.

### 7. Resolve Tooling And Release-Gate Gaps

- [x] Decide the canonical frontend package manager for release validation:
  - keep Bun-first and use `scripts/run-bun.mjs` to bypass the stale shim, or
  - switch to npm-first and add a committed `package-lock.json`.
  - Decision: keep Bun-first with committed `bun.lock`; use `scripts/run-bun.mjs` to bypass the stale PowerShell shim.
- [x] Fix or bypass the stale global `bun` PowerShell shim.
- [x] Make frontend dependency audit runnable for the chosen package manager.
- [ ] Install Clippy locally:
  - `rustup component add clippy`
- [ ] Run and fix:
  - `cd backend; cargo clippy --all-targets -- -D warnings`
- [ ] Install cargo-audit locally:
  - `cargo install cargo-audit`
- [ ] Run and triage:
  - `cd backend; cargo audit`
- [ ] Consider removing broad `#![allow(dead_code, unused_imports)]` suppressions once Clippy is available.
- [x] Keep `frontend/tsconfig.tests.json` in the root TypeScript build if tests are first-class TypeScript.

Acceptance criteria:
- Release validation commands are documented, runnable, and aligned with the committed lockfile.
- Clippy and dependency audits either pass or have explicit documented exceptions.
- The project no longer has a fake-green audit script.

### 8. Security And Release Posture Review

- [ ] Validate the new CSP in a packaged Tauri build.
- [ ] Confirm image previews still load through the app-cache asset protocol under the CSP.
- [ ] Confirm updater check/download/install still works under the narrowed capabilities.
- [ ] Review updater endpoint ownership.
- [ ] Review updater private-key storage, password policy, and rotation plan.
- [ ] Verify S-drive/shared-root ACLs with IT or the actual share owner.
- [ ] Record who can write to the shared root.
- [x] Decide whether HMAC is required if ACL verification is weak or unavailable.
  - Decision: require HMAC if ACL ownership/writer list cannot be verified before release.

Acceptance criteria:
- CSP and capabilities are validated in the packaged app, not only by source inspection.
- Updater signing and shared-root trust assumptions are recorded.
- Any manual security validation includes date, tester, and evidence path or notes.

### 9. Picture Path, UNC, And Reparse Policy

- [x] Decide whether UNC paths should remain allowed for picture open/preview.
  - Decision: UNC image paths remain allowed on Windows if they pass absolute-path, extension, size, file, and image-signature validation.
- [x] Decide whether filesystem canonicalization is required before preview caching.
  - Decision: no pre-cache canonicalization; cache fingerprint uses selected path plus metadata.
- [x] Decide how to handle symlinks/reparse points.
  - Decision: rely on OS path lookup and validate resolved file metadata plus image signature before caching.
- [~] Add tests or a packaged smoke for the chosen UNC behavior.
  - Source-level Windows UNC path tests exist; packaged UNC smoke still belongs in release smoke.
- [x] Document the picture-path policy in README or an engineering note.

Acceptance criteria:
- Picture path behavior is intentional for local paths, UNC paths, and reparse/symlink cases.
- Packaged smoke confirms the behavior on Windows.

### 10. Performance Baselines And Query Limits

- [ ] Run backend ignored performance baseline with realistic inventory size.
- [ ] Run frontend performance baseline with realistic inventory size.
- [ ] Record baseline outputs outside generated cache unless an evidence file is intentionally committed.
- [x] Decide whether the 10,000-row query target is still acceptable.
  - Decision: keep 10,000 rows as the 1.x lab-inventory target until performance baseline evidence says otherwise.
- [ ] If baseline fails the target, plan indexed storage or server-backed query work as a separate feature.

Acceptance criteria:
- Performance risk is backed by measurements.
- Query/index work is only started if the baseline proves it is needed.

### 11. Manual Smoke Before Release

- [ ] Run one-machine shared-sync smoke:
  - `powershell -ExecutionPolicy Bypass -File scripts\smoke-sync-one-machine.ps1`
- [ ] Build the packaged desktop app.
- [ ] Run packaged NSIS install smoke.
- [ ] Validate installed `1.0.3` updates to signed `1.0.4`, or adjust the version path to the current release plan.
- [ ] Run real shared-drive multi-machine sync smoke.
- [ ] Run real legacy SQLite import smoke if legacy import remains release-relevant.
- [ ] Confirm image preview/open behavior in the packaged app.
- [ ] Confirm signed updater behavior in the packaged app.
- [ ] Record smoke evidence:
  - commit
  - version
  - installer path
  - updater artifact path
  - tester
  - date
  - result

Acceptance criteria:
- Manual smoke covers install, update, sync, import, picture preview, and updater behavior.
- Evidence is recorded before release handoff.

## Already Addressed Enough For Final Verification

These do not need more implementation unless review finds a regression.

- [x] Add explicit Tauri CSP.
- [x] Replace broad Tauri capability defaults with narrower permissions.
- [x] Add backend shared-sync coordinator.
- [x] Validate remote operation timestamps.
- [x] Compare conflict timestamps by parsed time.
- [x] Repair all-hidden stored column visibility.
- [x] Add backend quantity and text-field bounds.
- [x] Add backend link and picture-path validation.
- [x] Add bridge-boundary runtime guards for inventory payloads.
- [x] Add Excel formula-like text regression test.
- [x] Add picture preview magic-byte checks.
- [x] Add frontend test TypeScript project.
- [x] Add dependency-audit script placeholders, pending the tooling decision above.

## Recommended Next Slice

Start with **manual validation and release evidence**.

Reason: the remaining unchecked work is mostly environment-bound release validation: packaged CSP/capability smoke, Clippy/cargo-audit installation, S-drive ACL confirmation, real UNC picture smoke, performance baselines, installer/update smoke, and shared-drive multi-machine evidence. Automated tests were intentionally not run in this pass because they are crashing VS Code on this workstation.
