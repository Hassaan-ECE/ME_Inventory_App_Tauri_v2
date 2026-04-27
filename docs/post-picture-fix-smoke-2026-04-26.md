# Post Picture Fix Release Evidence - 2026-04-26

Tester: Syed for GUI picture-preview confirmation; Codex for installer/hash/config evidence  
Installer path: `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.7\ME Inventory Setup 0.9.7.exe`  
Installer SHA-256: `A8C211589A5612C5F9FE3AAFC353B2AA4F77A4664C4380E0E855CDCDC588BDFE`  
App identifier: `com.me.inventory`  
Version: `0.9.7`

## Evidence Collected

- Shared release installer exists at the expected path.
- `Get-FileHash` confirmed the installer SHA-256 as `A8C211589A5612C5F9FE3AAFC353B2AA4F77A4664C4380E0E855CDCDC588BDFE`.
- `package.json`, `src-tauri\Cargo.toml`, and `src-tauri\tauri.conf.json` all report version `0.9.7`.
- `src-tauri\tauri.conf.json` still reports identifier `com.me.inventory`.
- `Get-AuthenticodeSignature` reports the shared installer as `NotSigned`.
- Syed confirmed in the working thread that the post-fix picture preview now works after the cache-backed native preview change.
- Syed confirmed right-click `Open Saved Link` opens a saved safe link.
- Syed confirmed picture preview opens in the default Windows image viewer.
- Syed confirmed missing-picture state works for new entries.
- Syed confirmed exported Excel workbooks now contain the expected two-sheet structure.
- Syed confirmed the installed app shows local-only status text: `Total: 147 | Verified: 3/147 | FeOxDB local store ready.`
- Syed confirmed the updater quiet-state check passes: no update popup, no `Update available` action, no download/install update button, and no update-related error.

## Passed

- Latest shared release installer hash is recorded and matches the planned release candidate.
- Current source configuration still uses product `ME Inventory`, version `0.9.7`, and identifier `com.me.inventory`.
- Local picture preview after selecting a picture path is no longer stuck in the missing state.
- Right-click `Open Saved Link` works for a saved safe link.
- Picture preview opens in the default image viewer.
- Missing-picture state works for new entries.
- Excel export has exactly the intended `Inventory` and `Archive` sheet structure.
- Local-only sync status is visible and does not claim remote/shared sync.
- Updater scaffold stays quiet and does not block inventory use.

## Still Needs Manual GUI Evidence

- Install/reinstall of the exact `A8C211...` shared installer as the current Windows user.
- Desktop launch from the installed shortcut after reinstall.
## Not Manually Tested

- Unsafe saved link rejection was not manually tested in the installed app. Automated URL/path validation still covers unsafe protocol rejection, and users are expected to verify links before saving/opening them.

## Updater Quiet-State Check

For the current scaffold, "updater quiet" means the app shows no update prompt, no `Update available` action, no download/install update button, and no update-related error that blocks inventory use. If the installed app launches and remains usable without any update UI, this check passes for the scaffolded updater.

## Open Release Risks

- Installer and executable remain unsigned, so Windows SmartScreen or enterprise policy prompts may still appear.
- Real updater signing, endpoint configuration, and release hosting remain out of scope.
- Shared sync remains local-only.
- Shared media storage remains out of scope; entries still store the selected local picture path.
