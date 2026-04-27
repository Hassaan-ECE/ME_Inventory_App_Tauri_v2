# Installer Smoke Result - Worker A

Date: 2026-04-26
Tester: Worker A - Installer Smoke
Git commit: `7f59922b7a1b17e927bc17430f13a5c043d6ca2b`
Repo: `D:\Projects\Active\ME_Inventory_App_Tauri_v2`
Installer: `D:\Projects\Active\ME_Inventory_App_Tauri_v2\src-tauri\target\release\bundle\nsis\ME Inventory_0.9.7_x64-setup.exe`
Installer SHA-256: `D6C79DC9E3D6F301A91B4354A823B7A4088100B90854F84632C5085F4D3D2233`
Full bundle build run: no, used supplied release artifact

## Backups

- Pre-install app data check: `C:\Users\syedh\AppData\Roaming\com.me.inventory` did not exist, so no pre-install backup was needed.
- Before uninstall/reinstall smoke, the newly created app data was backed up to `C:\Users\syedh\AppData\Roaming\com.me.inventory.backup-installer-smoke-20260426-162955`.

## Actions Run

- Read `guidelines.md`, `PORTING_TODO.md`, `README.md`, and `docs\desktop-smoke-checklist.md`.
- Confirmed installer metadata and hash with `Get-Item`, `Get-FileHash -Algorithm SHA256`, and `VersionInfo`.
- Checked pre-install app data with `Test-Path -LiteralPath 'C:\Users\syedh\AppData\Roaming\com.me.inventory'`.
- Checked pre-install app process state with `Get-Process`.
- Ran installer silently with:

```powershell
Start-Process -FilePath 'D:\Projects\Active\ME_Inventory_App_Tauri_v2\src-tauri\target\release\bundle\nsis\ME Inventory_0.9.7_x64-setup.exe' -ArgumentList '/S' -Wait -PassThru
```

- Inspected installed directory, Start Menu shortcut, and HKCU uninstall registry metadata.
- Compared bundled SQLite resource hashes between repo `data\*.db` and installed `_up_\data\*.db`.
- Launched installed app from the executable and from the Start Menu shortcut with `Start-Process`.
- Verified process state with `Get-Process`, including `Path`, `MainWindowTitle`, and `Responding`.
- Verified SQLite seed row counts with Python stdlib `sqlite3` in read-only mode.
- Verified FeOxDB imported entry evidence with:

```powershell
rg -a -o '"entryUuid"' 'C:\Users\syedh\AppData\Roaming\com.me.inventory\inventory.feox' | Measure-Object -Line
rg -a -o '__meta:legacy_import_path' 'C:\Users\syedh\AppData\Roaming\com.me.inventory\inventory.feox' | Measure-Object -Line
```

- Closed the app via `CloseMainWindow()` and confirmed the process exited.
- Backed up smoke-created app data with `Copy-Item -Recurse -Force`.
- Ran uninstaller silently with:

```powershell
Start-Process -FilePath 'C:\Users\syedh\AppData\Local\ME Inventory\uninstall.exe' -ArgumentList '/S' -Wait -PassThru
```

- Reinstalled silently with the same installer, relaunched from the Start Menu shortcut, rechecked persisted FeOxDB entry evidence, and closed the app.
- Checked Authenticode signature state with `Get-AuthenticodeSignature`.

## Results

| Item | Result | Evidence |
| --- | --- | --- |
| Pre-install app data backup | Pass | `com.me.inventory` did not exist before install; no pre-install backup needed. |
| Installer execution | Pass | Silent NSIS install `/S` returned exit code 0. |
| Current-user install location | Pass | Installed to `C:\Users\syedh\AppData\Local\ME Inventory`; HKCU uninstall key `ME Inventory` created. |
| Installed version metadata | Pass | Installer and installed `me-inventory.exe` report product/file version `0.9.7` and product name `ME Inventory`. |
| App identifier behavior | Pass | Config identifier is `com.me.inventory`; first launch created app data under `C:\Users\syedh\AppData\Roaming\com.me.inventory`. |
| Shortcut metadata | Pass | Start Menu shortcut targets `C:\Users\syedh\AppData\Local\ME Inventory\me-inventory.exe` with working directory `C:\Users\syedh\AppData\Local\ME Inventory`. |
| Launch from installed exe | Pass | Process `me-inventory` started, main window title `ME Inventory`, responding `True`. |
| Launch from shortcut | Pass | Shortcut launch started `me-inventory`, main window title `ME Inventory`, responding `True`. |
| Bundled SQLite resources | Pass | Installed `_up_\data\me_inventory.db` and `_up_\data\me_lab_inventory.db` exist and match repo SHA-256 hashes. |
| First-run import | Pass | Seed SQLite has 146 `entries`; first launch created `inventory.feox` with 146 serialized `entryUuid` occurrences and one legacy import marker. |
| Close/reopen persistence | Partial | After close/reopen, FeOxDB still had 146 `entryUuid` occurrences and one import marker. GUI add/edit persistence was not automated. |
| Uninstall preserves app data | Pass | Silent uninstall removed install dir, shortcut, and registry key; app data remained with 146 `entryUuid` occurrences and one import marker. |
| Reinstall reuses app data | Pass | Reinstall exit code 0; launch after reinstall reused existing app data with 146 `entryUuid` occurrences and one import marker. |
| Code signing | Partial | Installer and installed exe are `NotSigned`; this may be expected for this artifact but is a release distribution risk. |

## Limitations And Evidence Needed

- GUI-visible imported row count and header version were not captured from the Tauri window. Evidence needed: screenshot or manual visual confirmation that the UI shows `ME Inventory v0.9.7` and imported inventory rows.
- GUI add/edit persistence was not executed. Evidence needed: add a uniquely named smoke entry through the UI, close/reopen, edit it, close/reopen again, and confirm it remains changed.
- Installer UI prompts, finish-page launch behavior, and SmartScreen/enterprise policy behavior were not checked because the installer was run silently.
- Shortcut icon appearance was not visually verified.
- Shared sync and real updater behavior were not tested because they are intentionally scaffolded/deferred in the current port.
