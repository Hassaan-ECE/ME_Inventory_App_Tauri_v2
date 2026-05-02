# Release Evidence Checklist

Last updated: 2026-05-02

This file is a historical release-evidence log for the 0.9.x to 1.0.0 smoke path. For current hardening status, use `docs/engineering/CODE_BEHAVIOR_REMEDIATION_CHECKLIST.md`. Read `docs/engineering/AGENT_RUNBOOK.md` before release work.

## Status Legend

- `[ ]` not started
- `[~]` in progress
- `[x]` done
- `[!]` blocked or needs decision

## Current 1.0.1 Release Blockers

- [x] Build release binary and produce `1.0.1` NSIS installer.
- [x] Manually sign the produced `1.0.1` installer with the Tauri updater key after the NSIS wrapper hit Windows error 1224.
- [x] Stage `1.0.1` assets locally under `release\v1.0.1\`.
- [x] Stage `1.0.1` assets on the shared release drive under `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\1.0.1\`.
- [ ] Upload `1.0.1` assets to GitHub Release `v1.0.1` and verify `releases/latest/download/latest.json`.
- [ ] Validate installed `1.0.0` updates to signed `1.0.1`.
- [ ] Run packaged `1.0.1` NSIS install smoke.
- [ ] Run real shared-drive multi-machine sync smoke.
- [ ] Confirm packaged CSP, image preview/open behavior, and signed updater behavior.
- [ ] Record tester, machine names, installer path, updater artifact path, GitHub release URL, SHA-256, commit, source version, result, and date.

## Current 1.0.1 Evidence

| Check | Date | Tester | Machine(s) | Artifact / URL | SHA-256 | Result | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `1.0.1` small validation | 2026-05-02 | Codex | Build machine | Source tree |  | Partial pass | Passed Bun version, lint, targeted frontend bridge/column tests, Rust fmt/check, shared-sync coordinator unit filter, and sync HMAC integration filter. `cargo test --test sync_core recovery` timed out after 5 minutes and was stopped; full test suites were not run. |
| `1.0.1` signed local/shared staging | 2026-05-02 | Codex | Build machine | `release\v1.0.1\`; `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\1.0.1\ME Inventory_1.0.1_x64-setup.exe` | `a7b133a87784cb28811b0541191faa08a869e85fd1ef421880a641c9cf920293` | Staged; GitHub upload pending | Tauri build compiled the release binary and produced the installer, but NSIS bundling exited with Windows error 1224. The installer was manually signed with `tauri signer sign`; `.sig`, `latest.json`, and `SHA256SUMS.txt` were staged locally and on the shared drive. |

## Historical 0.9.x To 1.0.0 Path

<details>
<summary>Completed and pending version-by-version release steps</summary>

- [x] Back up root shared `current.json` before the fresh `0.9.0` reset.
- [x] Move old shared release folders from `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\` into a timestamped backup folder.
- [x] Generate a fresh local updater signing key outside the repo.
- [x] Reset source version to `0.9.0`.
- [x] Run pre-package validation for the `0.9.0` baseline.
- [x] Build signed `0.9.0` NSIS installer.
- [x] Stage `0.9.0` installer under the fresh shared release folder.
- [x] Publish shared root `current.json` for `0.9.0`.
- [ ] User installs staged `0.9.0` package and runs packaged NSIS smoke.
- [x] Bump source version to `0.9.1`.
- [x] Build signed `0.9.1` updater package.
- [x] Stage local GitHub release assets in `release\v0.9.1\`.
- [x] Upload `0.9.1` assets to GitHub Release `v0.9.1`.
- [ ] Validate installed `0.9.0` updates to signed `0.9.1`.
- [x] Bump source version to `0.9.2`.
- [x] Build signed `0.9.2` updater package.
- [x] Stage local and shared `0.9.2` release assets.
- [x] Upload `0.9.2` assets to GitHub Release `v0.9.2`.
- [ ] Validate installed `0.9.1` updates to signed `0.9.2`.
- [x] Bump source version to `0.9.3`.
- [x] Build signed `0.9.3` updater package.
- [x] Stage local and shared `0.9.3` release assets.
- [x] Upload `0.9.3` assets to GitHub Release `v0.9.3`.
- [ ] Validate installed `0.9.2` updates to signed `0.9.3`.
- [x] Bump source version to `0.9.4`.
- [x] Build signed `0.9.4` updater package.
- [x] Stage local and shared `0.9.4` release assets.
- [x] Upload `0.9.4` assets to GitHub Release `v0.9.4`.
- [ ] Validate installed `0.9.3` updates to signed `0.9.4`.
- [x] Bump source version to `0.9.5`.
- [x] Build signed `0.9.5` updater package.
- [x] Stage local and shared `0.9.5` release assets.
- [x] Upload `0.9.5` assets to GitHub Release `v0.9.5`.
- [ ] Validate installed `0.9.4` updates to signed `0.9.5`.
- [x] Bump source version to `0.9.6`.
- [x] Build signed `0.9.6` updater package.
- [x] Stage local and shared `0.9.6` release assets.
- [x] Upload `0.9.6` assets to GitHub Release `v0.9.6`.
- [ ] Validate installed `0.9.5` updates to signed `0.9.6`.
- [x] Bump source version to `0.9.7`.
- [x] Build signed `0.9.7` updater package.
- [x] Stage local and shared `0.9.7` release assets.
- [x] Upload `0.9.7` assets to GitHub Release `v0.9.7`.
- [ ] Validate installed `0.9.6` updates to signed `0.9.7`.
- [x] Bump source version to `0.9.8`.
- [x] Build signed `0.9.8` updater package.
- [x] Stage local and shared `0.9.8` release assets.
- [x] Upload `0.9.8` assets to GitHub Release `v0.9.8`.
- [ ] Validate installed `0.9.7` updates to signed `0.9.8`.
- [x] Bump source version to `0.9.9`.
- [x] Build signed `0.9.9` updater package.
- [x] Stage local and shared `0.9.9` release assets.
- [x] Upload `0.9.9` assets to GitHub Release `v0.9.9`.
- [ ] Validate installed `0.9.8` updates to signed `0.9.9`.
- [x] Bump source version to `1.0.0`.
- [x] Remove legacy database runtime paths and bundled `.db` resources.
- [x] Add FeOxDB snapshot/manifest bootstrap and operation-log compaction.
- [x] Add one-time local deprecated `.db` quarantine.
- [x] Build signed `1.0.0` updater package.
- [x] Stage local and shared `1.0.0` release assets.
- [x] Upload `1.0.0` assets to GitHub Release `v1.0.0`.
- [ ] Validate installed `0.9.9` updates to signed `1.0.0`.
- [ ] Run real shared-drive multi-machine sync smoke.

</details>

## Historical Evidence

<details>
<summary>0.9.x and 1.0.0 build/upload evidence</summary>

| Check | Date | Tester | Machine(s) | Artifact / URL | SHA-256 | Result | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `0.9.0` packaged NSIS install smoke | 2026-04-30 | Syed Hassaan Shah | Pending manual install machine | `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.0\ME Inventory_0.9.0_x64-setup.exe` | `ab890778a9ae2a0fcc422d1d5667eb955ac0888b37fd9c4956bf60458b77bb9d` | Pending manual smoke | Signed NSIS package staged; install smoke still needs user run. |
| `0.9.1` signed package build/staging | 2026-04-30 | Syed Hassaan Shah | Build machine | `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.1\ME Inventory_0.9.1_x64-setup.exe` | `8f7a8d986b801280a7cce574a976444176f7020b6861eb17c3c53b3fbe1c2704` | Pass | Restores S-drive `shared\me_lab_shared.db` hydration and shared DB writes. |
| `0.9.1` GitHub Release asset upload | 2026-04-30 | Syed Hassaan Shah | GitHub | `https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/tag/v0.9.1` | `8f7a8d986b801280a7cce574a976444176f7020b6861eb17c3c53b3fbe1c2704` | Pass | Uploaded `latest.json`, installer, `.sig`, and `SHA256SUMS.txt`; `releases/latest/download/latest.json` resolves to `0.9.1`. |
| `0.9.1` signed GitHub updater smoke |  |  |  |  |  |  |  |
| `0.9.2` signed package build/staging | 2026-04-30 | Syed Hassaan Shah | Build machine | `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.2\ME Inventory_0.9.2_x64-setup.exe` | `6acb53a0cb79bee9bfb0f475e3086e52773bc8605d427b21f8eddd7d72f14240` | Pass | Watches the shared DB folder and uses a 10-second shared SQLite polling backup. |
| `0.9.2` GitHub Release asset upload | 2026-04-30 | Syed Hassaan Shah | GitHub | `https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/tag/v0.9.2` | `6acb53a0cb79bee9bfb0f475e3086e52773bc8605d427b21f8eddd7d72f14240` | Pass | Uploaded `latest.json`, installer, `.sig`, and `SHA256SUMS.txt`; `releases/latest/download/latest.json` resolves to `0.9.2`. |
| `0.9.2` signed GitHub updater smoke |  |  |  |  |  |  |  |
| `0.9.3` signed package build/staging | 2026-04-30 | Syed Hassaan Shah | Build machine | `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.3\ME Inventory_0.9.3_x64-setup.exe` | `1278cbb08c3ab44a9c1e893106f2112772b23db45a186b38ee54ca0e695c029a` | Pass | Fixes shared SQLite edit identity, adds open-app update checks, and switches updater install mode to quiet with in-app progress. |
| `0.9.3` GitHub Release asset upload | 2026-04-30 | Syed Hassaan Shah | GitHub | `https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/tag/v0.9.3` | `1278cbb08c3ab44a9c1e893106f2112772b23db45a186b38ee54ca0e695c029a` | Pass | Uploaded `latest.json`, installer, `.sig`, and `SHA256SUMS.txt`; `releases/latest/download/latest.json` resolves to `0.9.3`. |
| `0.9.3` signed GitHub updater smoke |  |  |  |  |  |  |  |
| `0.9.4` signed package build/staging | 2026-04-30 | Syed Hassaan Shah | Build machine | `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.4\ME Inventory_0.9.4_x64-setup.exe` | `b62df42f353af3df7247523a9b8ef1d88c6bd5f6b52454f9e870a3da78656978` | Pass | Adds FeOxDB operation-log field-level merge support for the planned SQLite exit. |
| `0.9.4` GitHub Release asset upload | 2026-04-30 | Syed Hassaan Shah | GitHub | `https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/tag/v0.9.4` | `b62df42f353af3df7247523a9b8ef1d88c6bd5f6b52454f9e870a3da78656978` | Pass | Uploaded `latest.json`, installer, `.sig`, and `SHA256SUMS.txt`; `releases/latest/download/latest.json` resolves to `0.9.4`. |
| `0.9.4` signed GitHub updater smoke |  |  |  |  |  |  |  |
| `0.9.5` signed package build/staging | 2026-04-30 | Syed Hassaan Shah | Build machine | `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.5\ME Inventory_0.9.5_x64-setup.exe` | `d5f565427b32c1af53e33adac0c5409b42b1908135b9da2f429fbbcd6caa5fe1` | Pass | Adds active shared SQLite field-level merging for stale two-machine edits and restores window geometry across updater relaunch. |
| `0.9.5` GitHub Release asset upload | 2026-04-30 | Syed Hassaan Shah | GitHub | `https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/tag/v0.9.5` | `d5f565427b32c1af53e33adac0c5409b42b1908135b9da2f429fbbcd6caa5fe1` | Pass | Uploaded `latest.json`, installer, `.sig`, and `SHA256SUMS.txt`; `releases/latest/download/latest.json` resolves to `0.9.5`. |
| `0.9.5` signed GitHub updater smoke |  |  |  |  |  |  |  |
| `0.9.6` signed package build/staging | 2026-04-30 | Syed Hassaan Shah | Build machine | `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.6\ME Inventory_0.9.6_x64-setup.exe` | `e061e8558a20c0431d11379b34b7c81c17fe9d92adb3f2c7361dc1b924ce2951` | Pass | Hotfix removes the close-request window-state save that could prevent closing in `0.9.5`. |
| `0.9.6` GitHub Release asset upload | 2026-04-30 | Syed Hassaan Shah | GitHub | `https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/tag/v0.9.6` | `e061e8558a20c0431d11379b34b7c81c17fe9d92adb3f2c7361dc1b924ce2951` | Pass | Uploaded `latest.json`, installer, `.sig`, and `SHA256SUMS.txt`; `releases/latest/download/latest.json` resolves to `0.9.6`. |
| `0.9.6` signed GitHub updater smoke |  |  |  |  |  |  |  |
| `0.9.7` signed package build/staging | 2026-05-01 | Syed Hassaan Shah | Build machine | `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.7\ME Inventory_0.9.7_x64-setup.exe` | `1bbdc04000c39f67e1cda3e53cb39300a0ec4eed6e9ac8fd80ce14dcb4468408` | Pass | Moves normal shared workflow to FeOxDB operation logs; shared SQLite is seed-only. Tauri NSIS wrapper reported Windows error 1224, so the produced installer was manually signed with the same updater key. |
| `0.9.7` GitHub Release asset upload | 2026-05-01 | Syed Hassaan Shah | GitHub | `https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/tag/v0.9.7` | `1bbdc04000c39f67e1cda3e53cb39300a0ec4eed6e9ac8fd80ce14dcb4468408` | Pass | Uploaded `latest.json`, installer, `.sig`, and `SHA256SUMS.txt`; `releases/latest/download/latest.json` resolves to `0.9.7`. |
| `0.9.7` signed GitHub updater smoke |  |  |  |  |  |  |  |
| `0.9.8` signed package build/staging | 2026-05-01 | Syed Hassaan Shah | Build machine | `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.8\ME Inventory_0.9.8_x64-setup.exe` | `91f9bf3c57ea8977589730ba9ccac5dd67277a9c70928721eabbd949365a788c` | Pass | Makes FeOxDB sync near-live with a short local mutation push delay, 1-second fallback polling, and watermarked operation scans. Tauri NSIS wrapper reported Windows error 1224 after producing the installer, so the installer was manually signed with the same updater key. |
| `0.9.8` GitHub Release asset upload | 2026-05-01 | Syed Hassaan Shah | GitHub | `https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/tag/v0.9.8` | `91f9bf3c57ea8977589730ba9ccac5dd67277a9c70928721eabbd949365a788c` | Pass | Uploaded `latest.json`, installer, `.sig`, and `SHA256SUMS.txt`; `releases/latest/download/latest.json` resolves to `0.9.8` and the installer asset returns HTTP 200. |
| `0.9.8` signed GitHub updater smoke |  |  |  |  |  |  |  |
| `0.9.9` signed package build/staging | 2026-05-01 | Syed Hassaan Shah | Build machine | `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.9\ME Inventory_0.9.9_x64-setup.exe` | `c2743f18f6a6e7fb1353a670fdbee77450357c4f751f8a8ce3c1a6ce2b651266` | Pass | Shows local FeOxDB rows before shared sync, publishes saved changes to S-drive from a backend background task, keeps frontend sync coalesced, and lowers fallback polling to 500ms. |
| `0.9.9` GitHub Release asset upload | 2026-05-01 | Syed Hassaan Shah | GitHub | `https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/tag/v0.9.9` | `c2743f18f6a6e7fb1353a670fdbee77450357c4f751f8a8ce3c1a6ce2b651266` | Pass | Uploaded `latest.json`, installer, `.sig`, and `SHA256SUMS.txt`; `releases/latest/download/latest.json` resolves to `0.9.9` and the installer asset returns HTTP 200. |
| `0.9.9` signed GitHub updater smoke |  |  |  |  |  |  |  |
| `1.0.0` FeOxDB-only cleanup implementation | 2026-05-01 | Codex | Build machine | Source tree |  | Pass | Removes active legacy database code, adds snapshot/manifest bootstrap, compaction, and local deprecated `.db` quarantine. |
| `1.0.0` signed package build/staging | 2026-05-01 | Codex | Build machine | `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\1.0.0\ME Inventory_1.0.0_x64-setup.exe` | `0846d4f24eca14da78d621836958972604126b54c2148f5ce5cba41e16034200` | Pass | Tauri NSIS wrapper reported Windows error 1224 after producing the installer, so the installer was manually signed with the same updater key. |
| `1.0.0` GitHub Release asset upload | 2026-05-01 | Codex | GitHub | `https://github.com/Hassaan-ECE/ME_Inventory_App_Tauri_v2/releases/tag/v1.0.0` | `0846d4f24eca14da78d621836958972604126b54c2148f5ce5cba41e16034200` | Pass | Uploaded `latest.json`, installer, `.sig`, and `SHA256SUMS.txt`; `releases/latest/download/latest.json` resolves to `1.0.0` and the installer asset returns HTTP 200. |
| `1.0.0` signed GitHub updater smoke |  |  |  |  |  |  |  |
| Real shared-drive multi-machine sync smoke |  |  |  |  |  |  |  |

</details>

## Notes

- `1.0.1` still needs GitHub release upload and installed updater smoke from `1.0.0`.
- Real two-machine smoke still needs to validate snapshot bootstrap, operation compaction, fast convergence, and field-level merge on the S-drive.

## Current Artifact Paths

- Old releases backup: `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\backups\old-releases-20260430-103822\`
- Previous `current.json` backup: `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\backups\current-before-fresh-0.9.0-20260430-103822.json`
- Fresh shared releases root: `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\`
- Local updater key path: `%USERPROFILE%\.tauri\me-inventory-updater.key`

## Validation Commands

```powershell
node scripts\run-bun.mjs run lint
node scripts\run-bun.mjs run test
node scripts\run-bun.mjs run build

Push-Location backend
cargo fmt -- --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo audit
Pop-Location
```
