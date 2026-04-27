# Manual Installed-App Smoke - Syed - 2026-04-26

Tester: Syed  
Install source: `S:\Manufacturing\Internal\_Syed_H_Shah\InventoryApps\ME\releases\0.9.7\ME Inventory Setup 0.9.7.exe`  
User-tested installer SHA-256: `34853747877A0904CABD6A6880DAFD51D33AEAFECB2C5830F4C4F43D1CEDDAB8`
Post-fix installer SHA-256: `3534DC6A581EF8E8827139CC49CAC941D3E87C6595ED6D05E3CB92789D6732F1`

## Passed

- Uninstalled the old version before installing the shared-drive `0.9.7` installer.
- Installed as the current Windows user.
- Finish-page desktop shortcut was created and showed the delta ME Inventory icon.
- App launched from the desktop shortcut as the installed desktop app.
- Version behavior matched `0.9.7`.
- Inventory rows were visible after first launch.
- Close/reopen did not duplicate the imported row count.
- Search for existing asset, serial, or model filtered correctly.
- Created a unique smoke entry.
- Close/reopen preserved the new smoke entry.
- Edited location, project, notes, or a similarly safe field.
- Close/reopen preserved the edit.
- Verified/status update saved.
- Archive moved the smoke entry out of the active inventory view or marked it archived.
- Restore returned the smoke entry.
- Delete with confirmation removed the smoke entry and it stayed deleted after reopen.
- Search Online opened a browser with the expected search.
- Excel export cancel completed with no error.
- Excel export saved to a path with spaces.
- Workbook opened and contained exported active and archived data.
- Updater scaffold stayed quiet/not configured and did not block app use.

## Failed Or Changed Expectations

- Clicking a saved `https://` link did not open the browser.
- After selecting a picture path with spaces, the picture preview stayed missing even though the path was saved and appeared in Excel export.
- The exported workbook had `Inventory`, `Import Issues`, and `Export Summary`, but the desired workbook shape is now exactly `Inventory` and `Archive`.

## Follow-Up Decisions

- Fix saved links so table links and context-menu `Open Saved Link` route through the native Tauri opener.
- Fix local picture preview by loading validated image files through a native preview-safe mechanism instead of raw `file://` URLs.
- Keep original local picture paths in the current data model for now, but track future shared media storage as a separate TODO.
- Change Excel export to two sheets: non-archived entries in `Inventory`, archived entries in `Archive`.

## Follow-Up Evidence

- The later post-picture-fix evidence log is `docs/post-picture-fix-smoke-2026-04-26.md`.
- The implemented picture preview fix uses a cache-backed Tauri asset path rather than a base64 data URL.
