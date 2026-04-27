# Desktop Packaging And Smoke Checklist

This checklist records the packaging assumptions for the current Tauri parity wave and the manual smoke path to run before a release candidate is handed to users.

Do not change the app identifier during this wave. The configured identifier is `com.me.inventory`.

## Current Packaging Assumptions

Validated by read-only inspection on 2026-04-26:

- `src-tauri/tauri.conf.json` sets `productName` to `ME Inventory`, `version` to `0.9.8`, `identifier` to `com.me.inventory`, and bundles only the `nsis` target.
- `src-tauri/tauri.conf.json` sets `bundle.windows.nsis.installMode` to `currentUser`.
- `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` all report version `0.9.8`; `src/branding.ts` derives the displayed version from `package.json`.
- The bundle resources list includes `../data/me_inventory.db` and `../data/me_lab_inventory.db`; both files exist under `data/` in this checkout.
- The configured Windows icon is `src-tauri/icons/icon.ico`; the file exists in this checkout.
- `src-tauri/capabilities/default.json` grants `core:default` to the `main` window. The current frontend uses custom Tauri commands for inventory, native open, and picture picking rather than direct frontend calls to the dialog or opener plugins.
- Runtime storage is a local FeOxDB file named `inventory.feox` under Tauri's app data directory for this identifier.
- First-run import searches `ME_INVENTORY_LEGACY_SQLITE`, local `data/` SQLite files, bundled resources, and current-directory `data/` SQLite files.
- Shared sync has an operation-log foundation. `sync_inventory` bootstraps local entries, pushes pending outbox operations, pulls remote operation files, and reports shared-root availability/pending local state.
- Updater commands are scaffolded in Rust and return safe not-configured states until real Tauri updater signing and endpoints are configured.
- Excel export is wired into the Tauri desktop bridge for the current Tauri entry fields.

## Planned NSIS Current-User Installer Flow

1. From the repo root, build the installer:

   ```powershell
   bun tauri build --bundles nsis
   ```

2. Use the installer under:

   ```powershell
   src-tauri\target\release\bundle\nsis\
   ```

3. Run the installer as a normal Windows user. Expected flow for `installMode = currentUser`:

   - no machine-wide install requirement
   - no app identifier change
   - app files installed for the current Windows user
   - user data kept outside the install directory in Tauri app data
   - uninstall removes installed app files but should not unexpectedly erase the local FeOxDB user data unless installer settings are later changed to do that

4. Launch the app from the installer finish action or installed shortcut/start entry, then run the smoke checklist below.

## Preflight Before A Packaged Smoke

- [ ] Confirm the app identifier is still `com.me.inventory`.
- [ ] Confirm `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` versions match.
- [ ] Confirm `data\me_inventory.db`, `data\me_lab_inventory.db`, and `src-tauri\icons\icon.ico` exist.
- [ ] Use a disposable Windows profile, VM snapshot, or backed-up app data directory. Do not delete production app data during smoke.
- [ ] Record the installer filename, build date, git commit, and tester.
- [ ] Confirm whether the full NSIS build was run for this smoke.

## Fresh Install And First Run

- [ ] Uninstall any prior test build for this disposable profile.
- [ ] Install the new NSIS package as the current user.
- [x] Verify the installer does not require an admin or all-users install path.
- [x] Launch the app.
- [x] Verify the window title/header shows `ME Inventory` and version `0.9.7`.
- [x] Verify the app opens without falling back to browser mock data.
- [x] On a clean app data directory, verify first-run import loads entries from the bundled SQLite resource.
- [ ] If the imported count is zero, record whether the app showed `Legacy SQLite database was not found...`; that maps to bundled resource/import resolution.
- [x] Close and reopen the app.
- [x] Verify the entry count is stable and the legacy import did not duplicate entries.

## Persistence Across Reopen

- [x] Add a unique smoke entry with a timestamp in the description, for example `Smoke QA 2026-04-26 <tester>`.
- [x] Close the app completely.
- [x] Reopen the app from the installed shortcut/start entry.
- [x] Verify the smoke entry is still present.
- [x] Edit the smoke entry and close/reopen again.
- [x] Verify the edit persisted.
- [ ] Uninstall and reinstall the same build in the disposable profile.
- [ ] Verify the smoke entry still exists after reinstall unless the installer behavior is intentionally changed and documented.

## Entry CRUD And Table Behavior

- [ ] Add an entry with manufacturer, model, description, project, location, quantity, lifecycle status, working status, notes, link, and picture path.
- [ ] Edit every populated field and save.
- [x] Toggle verified from the table.
- [x] Search for the smoke entry.
- [ ] Use column filters for manufacturer, model, description, location, and asset number.
- [ ] Sort at least manufacturer, model, description, quantity, location, and verified.
- [ ] Toggle column visibility and confirm at least one data column must remain visible.
- [x] Archive the entry and verify it moves to Archive.
- [x] Restore the entry and verify it returns to Inventory.
- [x] Delete the entry and verify confirmation is required.

## Native Link, Path, And Picture Flow

- [x] Save an `https://` link and use right-click `Open Saved Link`; verify the default browser opens the link.
- [ ] Save a `mailto:` link if the environment has a mail handler; verify the OS handler opens or the app fails gracefully.
- [ ] Try an unsafe link such as `javascript:alert(1)`; verify it does not open.
- [x] Use right-click `Search Online`; verify the default browser opens a search for the entry details.
- [x] Use `Browse` in the picture panel to select a local image with spaces in the path.
- [x] Verify the selected absolute path is saved in the entry.
- [x] Verify the picture preview loads.
- [x] Double-click or press Enter/Space on the preview; verify the default image viewer opens it.
- [ ] If a UNC test share is available, repeat browse/open with a UNC image path.
- [x] Enter a non-existent absolute image path; verify the preview shows the missing state and the app stays usable.

## Excel Export

Current expected state: native export implemented, packaged smoke still required.

- [x] Open `Export > Excel`.
- [x] Verify the native save dialog opens with default filename `ME_Inventory_Export.xlsx`.
- [x] Cancel the dialog and verify no status error appears.
- [x] Save to a local path with spaces.
- [x] Open the workbook in Excel.
- [x] Verify exactly `Inventory` and `Archive` sheets exist.
- [x] Verify active entries are included in the `Inventory` sheet.
- [x] Verify archived entries are included in the `Archive` sheet.

## Updater Scaffold State

Current expected state: safe not-configured scaffold.

- [x] Launch the packaged app with no real Tauri updater signing/endpoint configured.
- [x] Verify no update action button appears when `UpdateState.available` is false.
- [x] Confirm the app remains usable when the updater is not configured.
- [x] On the exact latest shared installer, confirm there is no update prompt, no `Update available` action, and no update-related error blocking inventory use.
- [ ] After real updater configuration, extend this smoke to cover signed metadata, newer version detection, download progress, signature rejection, ready state, and install behavior.

## Shared Sync Foundation

Current expected state: operation-log foundation, not full snapshot/compactor sync.

- [x] Verify inventory CRUD works without any shared drive or shared workspace configuration.
- [x] Verify status communicates local FeOxDB readiness and shared-root state.
- [ ] Confirm remote/shared changes are pulled after another client writes operation files.
- [ ] Simulate shared workspace unavailable by running with no shared path configured; expected result is local editing with pending local operations.
- [ ] Simulate reconnect after pending local operations exist; expected result is operation files written under `<shared root>\shared\inventory\ops\{client_id}`.
- [x] Confirm no UI claims snapshot/manifest/compactor sync was completed.

## Latest Smoke Result

Automated installer smoke was run on 2026-04-26 and logged in `docs/installer-smoke-2026-04-26-worker-a.md`. Final manager QA after reviewer fixes is logged in `docs/release-qa-2026-04-26-manager.md`. Post-picture-fix release evidence is logged in `docs/post-picture-fix-smoke-2026-04-26.md`.

Passed with evidence:

- Silent current-user NSIS install completed with exit code 0.
- Installed app launched from the executable and Start Menu shortcut.
- Installed metadata reported product name `ME Inventory` and version `0.9.7`.
- First launch created app data under `C:\Users\syedh\AppData\Roaming\com.me.inventory`.
- Installed bundled SQLite resources matched repo resource hashes.
- First-run import loaded 146 seed entries and wrote one legacy import marker.
- Silent uninstall removed installed app files but preserved app data.
- Silent reinstall reused the existing app data.
- Final rebuilt installer SHA-256 `CE94ECA075DF3F88484BA58FD7FB3763E3676FBF95F66309ACDC03FB3B75BE59` installed and launched successfully.
- Final rebuilt installer clean first-run import created 146 `entryUuid` records and one legacy import marker.
- Previous app data was restored after final clean-import smoke.
- Syed manual installed-app smoke on 2026-04-26 used shared installer SHA-256 `34853747877A0904CABD6A6880DAFD51D33AEAFECB2C5830F4C4F43D1CEDDAB8`.
- Post-fix installer SHA-256 `3534DC6A581EF8E8827139CC49CAC941D3E87C6595ED6D05E3CB92789D6732F1` includes native table/context link opening, native local picture preview support, and two-sheet Excel export.
- Latest shared release installer SHA-256 `A8C211589A5612C5F9FE3AAFC353B2AA4F77A4664C4380E0E855CDCDC588BDFE` was verified at `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.7\ME Inventory Setup 0.9.7.exe`.
- Visible installer flow, finish-page desktop shortcut creation, delta shortcut icon, installed app launch, and version `0.9.7` passed.
- Visible inventory rows, stable row count after close/reopen, search, create/edit persistence, verify, archive, restore, delete, Search Online, Excel cancel/save/open, active plus archived export data, and quiet updater scaffold passed.
- Syed confirmed the cache-backed picture preview fix works after the latest picture-preview change.
- Syed confirmed post-fix `Open Saved Link`, picture open in the default viewer, missing-picture state for new entries, installed Excel two-sheet shape, and local-only status text `Total: 147 | Verified: 3/147 | FeOxDB local store ready.`.
- Syed confirmed updater quiet state on the latest shared installer: no update popup, no `Update available` action, no download/install update button, and no update-related error.

Partially verified or still manual:

- Unsafe saved link rejection was not manually tested in the installed app; automated URL/path validation covers unsafe protocol rejection.
- SmartScreen behavior remains unverified.
- Installer and installed executable were `NotSigned`.

## Open Packaging Risks

- Full NSIS bundle build passed for the current tree on 2026-04-26.
- Excel export is implemented and the installed GUI smoke now confirms the two-sheet `Inventory` and `Archive` contract.
- Real updater behavior is a release-flow blocker because signing, endpoints, artifact generation, download, and install are not configured.
- Shared sync now has the first operation-log foundation only. Snapshots, manifest compaction, conflict UI, shared media storage, and multi-machine installed smoke remain open.
- The installer and installed executable are unsigned. Windows SmartScreen or enterprise policy prompts may occur unless signing is added outside this repo config.
- Changing `identifier` after installation would move or strand app data. Keep `com.me.inventory` stable unless a separate migration plan is created.

## Result Log Template

```text
Date:
Tester:
Git commit:
Installer path:
Full bundle build run: yes/no
Installed version:
App identifier confirmed: com.me.inventory yes/no
Fresh install: pass/fail
First-run import: pass/fail
Persistence reopen: pass/fail
CRUD: pass/fail
Native link/path/picture: pass/fail
Excel export packaged smoke: pass/fail
Updater scaffold: pass/fail
Sync local-only: pass/fail
Blockers found:
Notes:
```
