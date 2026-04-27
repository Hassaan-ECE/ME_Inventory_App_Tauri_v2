# Release QA Result - Manager

Date: 2026-04-26
Tester: Manager
Repo: `D:\Projects\Active\ME_Inventory_App_Tauri_v2`
Installer: `D:\Projects\Active\ME_Inventory_App_Tauri_v2\src-tauri\target\release\bundle\nsis\ME Inventory_0.9.7_x64-setup.exe`
Installer SHA-256: `CE94ECA075DF3F88484BA58FD7FB3763E3676FBF95F66309ACDC03FB3B75BE59`

## Final Automated Checks

All checks below were run after the reviewer fixes for native path scope and local FeOxDB flush behavior:

- `bun run lint`: pass
- `bun run build`: pass
- `bun run test`: pass, 6 files / 43 tests
- `cargo fmt -- --check`: pass
- `cargo check`: pass
- `cargo test`: pass, 24 Rust unit tests plus 5 updater scaffold integration tests
- `bun tauri build --bundles nsis`: pass

Final bundle output:

- Path: `D:\Projects\Active\ME_Inventory_App_Tauri_v2\src-tauri\target\release\bundle\nsis\ME Inventory_0.9.7_x64-setup.exe`
- Size: `3692375` bytes
- Last write: `2026-04-26 16:43:14`
- Signature state: `NotSigned`

## Final Installer Smoke

The rebuilt installer was installed silently with `/S` after closing any running `me-inventory` process.

Before the clean first-run import smoke, existing app data at `C:\Users\syedh\AppData\Roaming\com.me.inventory` was moved to:

- `C:\Users\syedh\AppData\Roaming\com.me.inventory.backup-final-smoke-20260426-164409`

The rebuilt app was launched from:

- `C:\Users\syedh\AppData\Local\ME Inventory\me-inventory.exe`

Installed executable metadata:

- Product name: `ME Inventory`
- Product version: `0.9.7`
- File version: `0.9.7`
- Installed exe SHA-256: `68B0EF2AACFDB8917D2C6C69E7D9BC42CAD45AFE02B1CD756529C78AC8861F8D`

Clean first-run import evidence after closing the app:

- FeOxDB `entryUuid` count: `146`
- FeOxDB legacy import marker count: `1`

The clean final-smoke app data was preserved at:

- `C:\Users\syedh\AppData\Roaming\com.me.inventory.final-smoke-data-20260426-164409`

The previous app data was restored to:

- `C:\Users\syedh\AppData\Roaming\com.me.inventory`

## Notes

- One smoke script attempted to read `inventory.feox` while the Tauri app still had the file open and received a Windows file-lock error. The app was then closed, the FeOxDB evidence was read successfully, the clean-smoke data was preserved, and the previous app data was restored.
- GUI-only smoke remains unverified: visual imported row count/header, UI add/edit persistence across reopen, installer UI prompts, finish-page launch, shortcut icon appearance, native save/open dialogs and OS handlers, and opening the exported workbook in Excel.
- Shared sync and real signed updater behavior remain intentionally out of scope for this wave.
