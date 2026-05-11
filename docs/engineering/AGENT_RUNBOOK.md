# Agent Runbook

Last updated: 2026-05-11

Use this file before release, cleanup, or bug-fix work in this repo. It records project-specific traps, pivots that worked, and fixes still worth doing. Current release state lives in `README.md`; old release/audit evidence lives in `docs/engineering/archive/`.

## Agent Rules

- Run `git status --short` before editing.
- Treat `README.md` as the current entry point and use dated engineering docs as historical evidence unless they explicitly say they are active.
- Preserve user-authored changes. Do not revert files outside your assigned scope.
- Keep importing files and extracted folders together. If `InventoryShell.tsx` imports `frontend/src/features/inventory/components/shell/*`, that folder must be included when staging or committing.
- Use GPT-5.5 xhigh workers for independent cleanup slices when the user asks for worker support.
- Give each worker strict file ownership before it starts.
- The manager owns checklist updates, integration, validation, and final review.
- Record durable release blockers or repeatable pivots here before handing off. One-off command failures can stay in the final handoff.

## Known Hiccups And Pivots

### Broken Bun PowerShell Shim

- Symptom: `bun run ...` calls `C:\Users\Syed.H.Shah\AppData\Roaming\npm\node_modules\bun\bin\bun.exe`, and that file does not exist.
- `bun --version` can print this missing-executable error while still returning exit code `0`, so do not treat plain `bun` as validated just because `$LASTEXITCODE` is zero.
- Diagnose with:

```powershell
Get-Command bun -All
where.exe bun
```

- Working pivot:

```powershell
node scripts\run-bun.mjs run <script>
```

- Known good version from this workspace: `1.3.13`.
- Proper fix later: repair PATH, remove the stale npm shim, or reinstall Bun so `bun` resolves to `C:\Users\Syed.H.Shah\.bun\bin\bun.exe`.
- Until fixed, use `scripts\run-bun.mjs`; it resolves the real Bun binary or `BUN_EXE`.

### Missing Rustfmt

- Symptom: `cargo fmt -- --check` fails with `cargo-fmt.exe is not installed for the toolchain stable-x86_64-pc-windows-msvc`.
- Current status: resolved on 2026-04-28; `rustfmt-x86_64-pc-windows-msvc` is installed and `cargo fmt -- --check` passes.
- Proper fix:

```powershell
rustup component add rustfmt
```

- Do not claim Rust formatting passed until this is fixed and `cargo fmt -- --check` runs successfully.

### Cargo Check Timeout

- Symptom: `cargo check` timed out under a short planning timeout.
- Pivot: rerun with a longer timeout after Rust tooling is fixed.
- Current status: resolved on 2026-04-28; `cargo check` passed in 13.54s.
- Proper fix: use a timeout of at least several minutes for Rust checks.

### CRLF Normalization Warnings

- Symptom: `git diff --check` prints warnings such as `LF will be replaced by CRLF`.
- Pivot: treat as non-blocking only when `git diff --check` exits with code `0`.
- Proper fix only if needed: normalize line endings in a separate, explicit cleanup.

### NSIS User-Mapped Section Lock

- Symptom: `bun tauri build --bundles nsis` compiles the release executable, then fails during the NSIS bundle step with `The requested operation cannot be performed on a file with a user-mapped section open. (os error 1224)`.
- Pivot: confirm no app/build processes are running, remove only the generated installer for the target version under `backend\target\release\bundle\nsis\`, then rerun the bundle command.
- Historical cases are recorded under `docs/engineering/archive/`. The `1.0.4` NSIS build completed cleanly, but keep this pivot available if the file-lock error returns.
- Treat the installer from a failed bundle command as untrusted even if an `.exe` file exists.

### Signed Tauri Updater Key

- Current updater migration uses the official Tauri updater plugin with GitHub Releases static metadata.
- Public updater config lives in `backend\tauri.conf.json`.
- Private updater signing key was generated outside the repo at `%USERPROFILE%\.tauri\me-inventory-updater.key`.
- Public key sidecar was generated at `%USERPROFILE%\.tauri\me-inventory-updater.key.pub`.
- Release-smoke work rotated the updater key on 2026-04-30 because the previously documented key files were missing locally.
- Do not commit private key material, passwords, generated `.sig` files, release bundles, or `latest.json` drafts unless the user explicitly asks for release asset staging.
- The current key was generated without a password after the CLI rejected the empty-password flag form. Rotate the key before broad distribution if policy requires password-protected signing keys.
- For the current Tauri CLI, signing expects `TAURI_SIGNING_PRIVATE_KEY` to contain the private key text. `TAURI_SIGNING_PRIVATE_KEY_PATH` was not accepted during earlier package builds.
- Because the current key is unpassworded, set `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` to an explicit empty string for non-interactive builds; otherwise the signer can wait for password input after producing the NSIS installer.

### GitHub Release Upload

- `gh` is installed and authenticated on this workstation as of the `1.0.4` release.
- Stage installer, `.sig`, SHA-256 sums, and `latest.json` locally under an ignored `release\vX.Y.Z\` folder.
- Upload those assets to a non-draft, non-prerelease GitHub Release tagged `vX.Y.Z`, then validate the app updater against the real GitHub metadata URL.
- Use `gh release view vX.Y.Z --json tagName,isDraft,isPrerelease,url,targetCommitish,assets` after upload to verify asset names, hashes, and target commit.

### Shared Drive Staging

- Current shared root: `S:\Engineering\Public\Syed_Hassaan_Shah\InventoryApps\ME`.
- Keep the shared root obvious for installers: put only the current NSIS installer `.exe` at the root.
- Put updater metadata, `.sig` files, SHA-256 sums, previous installers, and other support material under folders such as `release-support\vX.Y.Z\` or `archive\`.
- Shared sync state belongs under `shared\inventory\`, with manifest, operation logs, snapshots, locks, and backups below that folder.
- The app fallback shared root is hardcoded in `backend/src/sync/types.rs`; update that constant if the shared root moves again.

### Untracked Source Files

- Symptom: imports pass locally, but new source files show as `??` in `git status --short`.
- Proper fix: never commit an importing file without the new module files it imports. Check `git status --short` before and after validation.

### Worker Fork Model Override

- Symptom: spawning a full-history fork while overriding `model` or `reasoning_effort` fails.
- Pivot: spawn without `fork_context: true` and include the needed repo/task context in the worker prompt.
- Proper fix: for GPT-5.5 xhigh worker requests, either omit fork context or omit model overrides.

## Validation Commands

### Frontend

Use the repo launcher until the shim is fixed:

```powershell
node scripts\run-bun.mjs run lint
node scripts\run-bun.mjs run test
node scripts\run-bun.mjs run build
node scripts\run-bun.mjs audit
```

### Git

```powershell
git diff --check
git status --short
```

### Rust

`rustfmt` is installed. Clippy and `cargo-audit` still need local installation before they can be release gates on this workstation.

Run:

```powershell
Push-Location backend
cargo fmt -- --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo audit
Pop-Location
```

## Worker Pattern

- Worker A style: one subsystem, one ownership boundary, no cross-file reversions.
- Worker B style: another independent subsystem with a disjoint write set.
- Reviewer worker: read-only unless the manager assigns a specific fix.
- Each worker final response must list changed files and commands run.
- The manager updates `README.md` for current handoff state. Historical evidence can be appended under `docs/engineering/archive/` when it is useful for audit trail, but do not make archived docs the source of current instructions.

## Troubleshooting Log Template

```text
Date:
Agent/worker:
Attempted command/action:
Error or symptom:
Root cause:
Pivot used:
Proper fix:
Status: resolved/deferred/blocked
```

## Troubleshooting Log

```text
Date: 2026-04-28
Agent/worker: Manager
Attempted command/action: bun run lint / bun run build
Error or symptom: PowerShell resolved bun to a stale npm shim that pointed at a missing bun.exe.
Root cause: PATH found C:\Users\Syed.H.Shah\AppData\Roaming\npm\bun.ps1 before the real Bun install.
Pivot used: Ran C:\Users\Syed.H.Shah\.bun\bin\bun.exe directly.
Proper fix: Repair PATH or remove/reinstall the stale npm shim.
Status: deferred
```

```text
Date: 2026-04-28
Agent/worker: Manager
Attempted command/action: cargo fmt -- --check
Error or symptom: cargo-fmt.exe is not installed for stable-x86_64-pc-windows-msvc.
Root cause: rustfmt component missing.
Pivot used: Marked Rust format validation blocked and limited the first cleanup slice to frontend.
Proper fix: rustup component add rustfmt.
Status: resolved; rustfmt is now installed and cargo fmt -- --check passes.
```

```text
Date: 2026-04-28
Agent/worker: Manager
Attempted command/action: cargo check
Error or symptom: Command timed out under the planning timeout.
Root cause: Unknown; likely needed more time or was waiting on build work.
Pivot used: Deferred Rust baseline until tooling cleanup.
Proper fix: Rerun with a longer timeout after rustfmt is installed.
Status: resolved; cargo check passed in 13.54s with a longer timeout.
```

```text
Date: 2026-04-28
Agent/worker: Manager
Attempted command/action: Spawn GPT-5.5 xhigh workers with fork_context=true.
Error or symptom: Runtime rejected model/reasoning override with full-history fork.
Root cause: Full-history forked agents inherit parent model and reasoning effort.
Pivot used: Spawned workers without fork_context and included task context in the prompt.
Proper fix: Use this spawn pattern for future GPT-5.5 xhigh workers.
Status: resolved
```

```text
Date: 2026-04-28
Agent/worker: Manager
Attempted command/action: bun --version
Error or symptom: PowerShell printed the stale npm shim missing-executable error but returned exit code 0.
Root cause: The stale npm bun.ps1 wrapper still resolves before C:\Users\Syed.H.Shah\.bun\bin\bun.exe.
Pivot used: Continue using C:\Users\Syed.H.Shah\.bun\bin\bun.exe directly.
Proper fix: Repair PATH, remove the stale npm shim, or reinstall Bun.
Status: deferred
```

```text
Date: 2026-04-28
Agent/worker: Manager
Attempted command/action: cargo fmt -- --check; cargo check; cargo test
Error or symptom: No failure. cargo test emitted four dead_code warnings in the updater_scaffold test target.
Root cause: The updater scaffold test imports updater functions that are not all used by that test binary.
Pivot used: None needed; validation passed.
Proper fix: Removed the custom updater scaffold during the signed Tauri updater migration.
Status: resolved on 2026-04-29
```

```text
Date: 2026-04-29
Agent/worker: Manager
Attempted command/action: bun tauri signer generate --ci --password "" --write-keys "$env:USERPROFILE\.tauri\me-inventory-updater.key"
Error or symptom: The Tauri CLI rejected the empty password flag form as a missing value.
Root cause: Empty string argument was not accepted for `--password` in this PowerShell invocation.
Pivot used: Generated keys with `bun tauri signer generate --ci --write-keys "$env:USERPROFILE\.tauri\me-inventory-updater.key"`.
Proper fix: For protected release keys, rerun key generation interactively or pass a real secret from a secure local store.
Status: resolved; current key is unpassworded and stored outside the repo.
```

```text
Date: 2026-04-28
Agent/worker: Manager
Attempted command/action: bun tauri build --bundles nsis for a historical release
Error or symptom: NSIS bundling failed with os error 1224, user-mapped section open.
Root cause: Windows held a mapping to the generated installer output during the bundle step.
Pivot used: Removed only the generated installer under backend\target\release\bundle\nsis\ and reran the bundle command with the real Bun binary first on PATH.
Proper fix: Retry after clearing the generated installer output; investigate file locking only if the error repeats.
Status: resolved
```
