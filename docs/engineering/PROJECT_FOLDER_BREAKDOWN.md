# Project Folder Breakdown

Last updated: 2026-05-11

This document maps the project files to their responsibilities. It covers tracked source, config, docs, scripts, and tests. Generated folders such as `node_modules/`, `backend/target/`, `dist/`, `.tmp/`, and release staging output are summarized instead of expanded.

## Top-Level Files

| Path | Handles |
| --- | --- |
| `.gitignore` | Git ignore rules for generated output, local state, build artifacts, and release staging files. |
| `README.md` | Main project entry point with architecture notes, release status, build commands, sync layout, and smoke-test guidance. |
| `bun.lock` | Locked frontend JavaScript dependency graph for Bun/npm-compatible installs. |
| `eslint.config.js` | ESLint configuration for frontend TypeScript and React code. |
| `package.json` | Workspace scripts, frontend dependencies, app version, and package manager declaration. |
| `tsconfig.json` | Root TypeScript project references for frontend app, tests, and Vite config. |

## Backend App Shell

| Path | Handles |
| --- | --- |
| `backend/Cargo.lock` | Locked Rust dependency graph for reproducible backend/Tauri builds. |
| `backend/Cargo.toml` | Rust crate metadata, dependencies, binary/lib targets, and package version. |
| `backend/build.rs` | Tauri build script hook used by Cargo during backend builds. |
| `backend/capabilities/default.json` | Tauri capability permissions exposed to the frontend window. |
| `backend/icons/icon.ico` | Windows application icon bundled into the desktop app. |
| `backend/tauri.conf.json` | Tauri product metadata, build config, bundle settings, updater config, and window config. |
| `backend/windows/nsis-hooks.nsh` | NSIS installer hook script used during Windows bundle generation. |
| `backend/src/main.rs` | Native binary entry point that calls the Tauri library runner. |
| `backend/src/lib.rs` | Tauri application setup: plugins, FeOxDB open, cleanup/recovery, managed state, commands, and shutdown flush. |

## Backend API Layer

| Path | Handles |
| --- | --- |
| `backend/src/api/mod.rs` | API module exports for commands and mutation helpers. |
| `backend/src/api/commands.rs` | Tauri command handlers for load/query/sync/mutations and background shared publish scheduling. |
| `backend/src/api/mutations.rs` | Local inventory create/update/verify/archive/delete behavior plus sync operation queueing and rollback-on-error. |

## Backend Domain Layer

| Path | Handles |
| --- | --- |
| `backend/src/domain/mod.rs` | Domain module exports for models, query logic, and entry change helpers. |
| `backend/src/domain/model.rs` | Inventory structs, command result types, validation, normalization, timestamp helpers, create/update transforms, and shared-status DTOs. |
| `backend/src/domain/query.rs` | Backend filtering, sorting, paging, search, archive/inventory scoping, and count calculation. |
| `backend/src/domain/entry_changes.rs` | Field-level edit context support: base version extraction, changed-field normalization, and partial update projection. |

## Backend Integrations

| Path | Handles |
| --- | --- |
| `backend/src/integrations/mod.rs` | Integration module exports for cleanup, export, and native desktop helpers. |
| `backend/src/integrations/deprecated_db_cleanup.rs` | One-time quarantine of legacy SQLite `.db` files into app-data backups. |
| `backend/src/integrations/native.rs` | Safe external URL opening, local path opening, picture preview cache generation, and picture picker integration. |
| `backend/src/integrations/export/mod.rs` | Excel export command plumbing, save dialog selection, result structs, and workbook write entry point. |
| `backend/src/integrations/export/workbook.rs` | Excel workbook construction for inventory/archive sheets, rows, cells, and export summary counts. |
| `backend/src/integrations/export/workbook/columns.rs` | Export column definitions and mapping from inventory entries to cell values. |
| `backend/src/integrations/export/workbook/formats.rs` | Excel workbook formatting, headers, status colors, alignments, and reusable cell styles. |

## Backend Runtime

| Path | Handles |
| --- | --- |
| `backend/src/runtime/mod.rs` | Runtime module exports for sync coordination and shared-drive watcher. |
| `backend/src/runtime/shared_sync.rs` | Shared sync coordinator mutex and last background status cache; serializes mutations, sync, and publish work. |
| `backend/src/runtime/shared_watcher.rs` | File watcher for shared operation logs and debounced `inventory:shared-changed` frontend events. |

## Backend FeOxDB Storage

| Path | Handles |
| --- | --- |
| `backend/src/storage/mod.rs` | `InventoryDb` wrapper around FeOxDB, app/test open helpers, flush behavior, and module wiring. |
| `backend/src/storage/codec.rs` | JSON serialization/deserialization helpers and primitive metadata decoding. |
| `backend/src/storage/entries.rs` | Entry key-value CRUD, UUID/numeric-id lookup, id index maintenance, scans, and snapshot entry replacement. |
| `backend/src/storage/keys.rs` | Canonical key prefixes, sync key formatting/parsing, path segment validation, and sequence formatting. |
| `backend/src/storage/metadata.rs` | App metadata accessors: next inventory id, sync schema, client/device identity, local sequence, revision, and snapshot id. |
| `backend/src/storage/sync_state.rs` | Sync keyspaces for outbox, applied markers, watermarks, tombstones, entry states, conflicts, corrupt files, and arbitrary sync metadata. |
| `backend/src/storage/tests.rs` | Unit tests for storage lookup, delete behavior, metadata stability, sync records, tombstones, and keyspace isolation. |

## Backend Shared Sync

| Path | Handles |
| --- | --- |
| `backend/src/sync/mod.rs` | Sync module wiring and selective exports for runtime commands and tests. |
| `backend/src/sync/types.rs` | Shared sync constants, default shared root, operation envelope types, tombstones, entry state, conflicts, and corrupt-file records. |
| `backend/src/sync/apply.rs` | Main shared sync loop: push local ops, apply snapshots, pull remote ops, merge disjoint field edits, apply deletes/upserts, and advance watermarks. |
| `backend/src/sync/auth.rs` | Optional HMAC signing/verification for operation, snapshot, and manifest trust. |
| `backend/src/sync/conflicts.rs` | Current entry-state lookup, last-write-wins comparison, stale conflict recording, and corrupt remote marker writing. |
| `backend/src/sync/identity.rs` | Persistent client/device identity and local sequence allocation helpers. |
| `backend/src/sync/queue.rs` | Local operation construction, outbox persistence, bootstrap operation creation, tombstone creation, and pending operation push. |
| `backend/src/sync/recovery.rs` | Startup repair for partial local sync writes, missing markers, stale tombstones, local sequence state, and interrupted snapshot apply markers. |
| `backend/src/sync/scanning.rs` | Shared operation-file scanning after watermarks, temp/unknown file skipping, validation, and deterministic operation ordering. |
| `backend/src/sync/shared_paths.rs` | Shared root resolution, shared folder creation, and user-facing shared status construction. |
| `backend/src/sync/timestamps.rs` | UTC RFC3339 timestamp validation and parsed timestamp comparison. |

## Backend Operation Files

| Path | Handles |
| --- | --- |
| `backend/src/sync/operation_file/mod.rs` | Atomic operation JSON writes, operation reads, checksum/auth/timestamp validation, and duplicate sequence detection. |
| `backend/src/sync/operation_file/canonical.rs` | Canonical JSON serialization, checksum generation, SHA-256 helpers, and operation signing bytes. |
| `backend/src/sync/operation_file/paths.rs` | Operation file path/name formatting, safe path segment validation, temp-file detection, and sequence parsing. |
| `backend/src/sync/operation_file/validation.rs` | Operation envelope and payload identity validation before writing or accepting remote operations. |

## Backend Snapshots

| Path | Handles |
| --- | --- |
| `backend/src/sync/snapshot/mod.rs` | Snapshot module wiring and public snapshot exports. |
| `backend/src/sync/snapshot/types.rs` | Snapshot/manifest schemas, snapshot watermarks, constants, and report structs. |
| `backend/src/sync/snapshot/apply.rs` | Safe snapshot apply logic, local keyspace replacement, manifest coverage checks, and pending apply marker handling. |
| `backend/src/sync/snapshot/io.rs` | Manifest/snapshot read/write, checksum/auth validation, atomic JSON writes, operation compaction, and snapshot pruning. |
| `backend/src/sync/snapshot/lock.rs` | Snapshot publish lock file acquisition, stale lock removal, and cleanup on drop. |
| `backend/src/sync/snapshot/publish.rs` | Snapshot publish decision, snapshot assembly, manifest creation, signing, compaction, and pruning. |

## Backend Tests

| Path | Handles |
| --- | --- |
| `backend/tests/performance_baseline.rs` | Ignored backend performance baseline for load/query/count/sync status operations on current and synthetic datasets. |
| `backend/tests/shared_sync_flow.rs` | End-to-end shared sync tests for bootstrap, snapshots, HMAC, create/update/delete, watermarks, revision, and smoke workflow behavior. |
| `backend/tests/sync_conflict_flow.rs` | Conflict behavior tests for duplicate operation files, tombstones, last-write-wins, field merge, delete/restore, and stale operations. |
| `backend/tests/sync_core.rs` | Sync core tests for shared root resolution, identity, checksum, operation files, HMAC, timestamp validation, recovery, watermarks, and builders. |
| `backend/tests/support/backend.rs` | Test module bridge exposing backend domain, storage, and sync modules to integration tests. |
| `backend/tests/support/core_backend.rs` | Narrower test module bridge for core domain/query/storage testing. |
| `backend/tests/support/sync_fixtures.rs` | Shared sync test fixtures for temp dirs, sample entries, remote operations, corrupt snapshots, counts, and file helpers. |

## Docs

| Path | Handles |
| --- | --- |
| `docs/engineering/AGENT_RUNBOOK.md` | Project-specific runbook for release, cleanup, build traps, shared drive staging, and recurring fixes. |
| `docs/engineering/CLEANUP_CHECKLIST.md` | Historical release evidence checklist and current release validation records. |
| `docs/engineering/CODE_BEHAVIOR_AUDIT.md` | Audit notes for code behavior risks and observations. |
| `docs/engineering/CODE_BEHAVIOR_REMEDIATION_CHECKLIST.md` | Remediation tracker for sync, recovery, HMAC, validation, and release quality gates. |
| `docs/engineering/DONE_CHECKLIST.md` | Historical completion log of major implementation and release tasks. |
| `docs/engineering/FEOXDB_SYNC_MIGRATION_PLAN.md` | Compact design note for the FeOxDB operation-log sync architecture and shared layout. |
| `docs/engineering/SYNC_RECOVERY_INVARIANTS.md` | Explicit recovery invariants for local outbox, markers, tombstones, snapshots, and sequence state. |
| `docs/engineering/PROJECT_FOLDER_BREAKDOWN.md` | This file: annotated map of project folders and tracked files. |
| `docs/performance/PERFORMANCE_BASELINE.md` | Recorded performance baseline notes and guidance for rerunning baseline tests. |

## Frontend App Shell

| Path | Handles |
| --- | --- |
| `frontend/index.html` | Vite HTML entry point for the React app. |
| `frontend/public/favicon.ico` | Browser/dev favicon asset. |
| `frontend/src/app/App.tsx` | Top-level React app component that renders the inventory shell. |
| `frontend/src/app/branding.ts` | App name, version, author, credit, and display-name constants. |
| `frontend/src/app/index.css` | Global CSS, Tailwind v4 imports/theme tokens, dark mode variables, and base page styling. |
| `frontend/src/app/main.tsx` | React DOM bootstrap and Tauri bridge import side effect. |

## Frontend Inventory Components

| Path | Handles |
| --- | --- |
| `frontend/src/features/inventory/types.ts` | Frontend inventory types, status/result DTOs, update types, filters, columns, and option constants. |
| `frontend/src/features/inventory/data/mockInventory.ts` | Mock inventory rows used outside the desktop bridge. |
| `frontend/src/features/inventory/components/InventoryShell.tsx` | Main inventory UI composition, state wiring, filters, table/dialog/menu orchestration, and action routing. |
| `frontend/src/features/inventory/components/InventoryHeader.tsx` | Header actions for scope counts, add/export/update/theme controls. |
| `frontend/src/features/inventory/components/SearchCard.tsx` | Search, filter toggle, color-row toggle, result label, and column menu shell. |
| `frontend/src/features/inventory/components/FilterPanel.tsx` | Field-specific filter inputs and clear-filter action. |
| `frontend/src/features/inventory/components/InventoryTable.tsx` | Memoized table wrapper with scroll state and virtualized body/header composition. |
| `frontend/src/features/inventory/components/ColumnMenu.tsx` | Column visibility menu and per-column toggle controls. |
| `frontend/src/features/inventory/components/EmptyResults.tsx` | Empty-state UI for no matching inventory/archive results. |
| `frontend/src/features/inventory/components/EntryContextMenu.tsx` | Row context menu actions for open, link, search, archive/restore, and delete. |
| `frontend/src/features/inventory/components/EntryDialog.tsx` | Add/edit entry dialog, form fields, metadata sidebar, preview panel, and save/cancel behavior. |
| `frontend/src/features/inventory/components/StatusStrip.tsx` | Bottom status message strip. |

## Frontend Entry Dialog

| Path | Handles |
| --- | --- |
| `frontend/src/features/inventory/components/entry-dialog/PicturePreviewPanel.tsx` | Picture preview panel wrapper for compact/sidebar and inline layouts. |
| `frontend/src/features/inventory/components/entry-dialog/components.tsx` | Reusable dialog pieces: picture preview card, placeholder, metadata row, and dialog actions. |
| `frontend/src/features/inventory/components/entry-dialog/editContext.ts` | Builds edit context with base version and changed fields for backend merge safety. |
| `frontend/src/features/inventory/components/entry-dialog/fieldMetadata.ts` | Field metadata arrays for text inputs, selects, booleans, and database context rows. |
| `frontend/src/features/inventory/components/entry-dialog/form.ts` | Form state building, input validation, field-diff calculation, field updates, and option label formatting. |
| `frontend/src/features/inventory/components/entry-dialog/picturePreview.ts` | Picture preview source classification, native preview decision, and keyboard handling for previews. |
| `frontend/src/features/inventory/components/entry-dialog/useEntryDialogLayout.ts` | Responsive dialog layout decisions for sidebar actions and preview placement. |
| `frontend/src/features/inventory/components/entry-dialog/useEntryDialogSubmit.ts` | Submit state, form-to-input conversion, edit context attachment, and error handling. |
| `frontend/src/features/inventory/components/entry-dialog/useEntryPicturePreview.ts` | Picture preview loading state, native preview calls, picker handling, and path change behavior. |
| `frontend/src/features/inventory/components/entry-dialog/useMediaQuery.ts` | React-safe media query subscription hook. |
| `frontend/src/features/inventory/components/entry-dialog/useMountedRef.ts` | Mounted-state ref helper for safe async state updates. |

## Frontend Header Components

| Path | Handles |
| --- | --- |
| `frontend/src/features/inventory/components/header/ExportMenu.tsx` | Export menu for Excel and HTML export actions. |
| `frontend/src/features/inventory/components/header/ScopeToggle.tsx` | Inventory/archive segmented scope toggle with counts. |
| `frontend/src/features/inventory/components/header/UpdateActionButton.tsx` | Updater action button labels, icons, disabled states, and progress display. |

## Frontend Shell Hooks

| Path | Handles |
| --- | --- |
| `frontend/src/features/inventory/components/shell/DeleteConfirmationDialog.tsx` | Styled confirmation modal for destructive delete actions. |
| `frontend/src/features/inventory/components/shell/helpers.ts` | Shell constants and helpers for shared status, local mock mutations, preferences, and status text. |
| `frontend/src/features/inventory/components/shell/useDesktopInventory.ts` | Desktop inventory loading, initial sync, polling, shared-change subscriptions, and local entry state. |
| `frontend/src/features/inventory/components/shell/useDesktopUpdates.ts` | Updater state management and check/download/install action flow. |
| `frontend/src/features/inventory/components/shell/useInventoryEntryMutations.ts` | UI mutation handlers for add/edit/verify/archive/delete across desktop and mock modes. |
| `frontend/src/features/inventory/components/shell/useInventoryExportActions.ts` | Excel and HTML export action handling plus status announcements. |
| `frontend/src/features/inventory/components/shell/useInventoryExternalActions.ts` | Saved-link opening, arbitrary external URL opening, and online search action handling. |
| `frontend/src/features/inventory/components/shell/useInventoryPreferences.ts` | Theme, row color, and column visibility preference persistence. |
| `frontend/src/features/inventory/components/shell/useInventoryViewModel.ts` | Derived view state: filtered/sorted rows, counts, visible columns, result labels, and row map. |
| `frontend/src/features/inventory/components/shell/useStatusAnnouncer.ts` | Temporary status override announcer with timeout cleanup. |

## Frontend Table

| Path | Handles |
| --- | --- |
| `frontend/src/features/inventory/components/table/InventoryTableBody.tsx` | Virtualized table rows, cells, row tones, link buttons, verify toggle, and spacer rows. |
| `frontend/src/features/inventory/components/table/InventoryTableHeader.tsx` | Column group widths and sortable table header rendering. |
| `frontend/src/features/inventory/components/table/columnStyles.ts` | Stable per-column width and style definitions. |
| `frontend/src/features/inventory/components/table/virtualization.ts` | Row height, overscan, visible-range calculation, and scroll clamping. |

## Frontend Inventory Libraries

| Path | Handles |
| --- | --- |
| `frontend/src/features/inventory/lib/index.ts` | Barrel exports for inventory library helpers. |
| `frontend/src/features/inventory/lib/columns.ts` | Column visibility defaults, visibility merge, visible column derivation, and link label formatting. |
| `frontend/src/features/inventory/lib/counts.ts` | Inventory/archive/verified/total count calculation. |
| `frontend/src/features/inventory/lib/filtering.ts` | Filter defaults, global search field definitions, filter matching, and query matching. |
| `frontend/src/features/inventory/lib/resultLabels.ts` | Human-readable result label construction for current scope/filter/query state. |
| `frontend/src/features/inventory/lib/sorting.ts` | Sort comparator logic for inventory columns, including blank handling. |

## Frontend Tauri Integration

| Path | Handles |
| --- | --- |
| `frontend/src/integrations/tauri/bridgeGuards.ts` | Runtime validation and normalization for backend/Tauri payloads crossing into TypeScript. |
| `frontend/src/integrations/tauri/desktop-bridge.d.ts` | Global `window.inventoryDesktop` bridge type declarations. |
| `frontend/src/integrations/tauri/tauriInventoryBridge.ts` | Tauri `invoke` bridge installation, updater flow, event listeners, desktop commands, and picture/export/native bindings. |
| `frontend/src/integrations/tauri/windowState.ts` | Window geometry persistence, monitor visibility checks, debounced save, and restore validation. |

## Frontend Shared UI And Utilities

| Path | Handles |
| --- | --- |
| `frontend/src/shared/assets/hero.png` | Shared raster image asset retained with the frontend bundle. |
| `frontend/src/shared/components/ui/badge.tsx` | Badge component variants for status labels. |
| `frontend/src/shared/components/ui/button.tsx` | Button component variants and shared button styling. |
| `frontend/src/shared/components/ui/empty.tsx` | Empty-state layout primitives. |
| `frontend/src/shared/components/ui/input.tsx` | Styled input component wrapper. |
| `frontend/src/shared/components/ui/textarea.tsx` | Styled textarea component wrapper. |
| `frontend/src/shared/components/ui/toggle.tsx` | Toggle button component variants. |
| `frontend/src/shared/lib/externalUrl.ts` | Frontend URL safety checks and implicit HTTPS normalization. |
| `frontend/src/shared/lib/utils.ts` | Class-name merge helper and relative timestamp formatter. |

## Frontend Tests

| Path | Handles |
| --- | --- |
| `frontend/tests/setup.ts` | Vitest setup, test environment globals, and jest-dom registration. |
| `frontend/tests/columns.test.ts` | Column visibility, visible columns, and data-column count coverage. |
| `frontend/tests/entry-dialog.test.tsx` | Entry dialog behavior, changed field edit context, picture path/preview behavior, and dialog layout coverage. |
| `frontend/tests/external-url.test.ts` | External URL safety and implicit HTTPS normalization tests. |
| `frontend/tests/inventory-entry-actions.test.tsx` | Entry action workflow tests for add/edit/archive/restore/delete/open-link behavior. |
| `frontend/tests/inventory-filtering.test.ts` | Inventory filtering and global search tests. |
| `frontend/tests/inventory-shell.test.tsx` | Core inventory shell rendering and high-level behavior tests. |
| `frontend/tests/inventory-shell-mutations.test.tsx` | Shell mutation tests for local/desktop add, save, verify, archive, and shared status feedback. |
| `frontend/tests/inventory-shell-sync.test.tsx` | Shared sync polling, startup sync loading, watcher event, and mutation-triggered sync tests. |
| `frontend/tests/inventory-shell-updates.test.tsx` | Desktop updater check/download/install UI and state tests. |
| `frontend/tests/inventory-shell-views-export.test.tsx` | View, filter, scope, column, theme, and export behavior tests. |
| `frontend/tests/inventory-table.test.tsx` | Table rendering, sorting, virtualization, row actions, and responsive table behavior tests. |
| `frontend/tests/performance-baseline.test.tsx` | Frontend performance baseline for large synthetic table/filter/render operations. |
| `frontend/tests/tauri-inventory-bridge.test.ts` | Tauri bridge parsing, command invocation, event handling, updater flow, and window-state persistence tests. |
| `frontend/tests/window-state.test.ts` | Window-state parsing, bounds validation, monitor visibility, and clamp logic tests. |
| `frontend/tests/inventory-shell/helpers.tsx` | Shared frontend test helpers for entries, bridge mocks, shared status, counts, and deferred promises. |

## Frontend Tooling

| Path | Handles |
| --- | --- |
| `frontend/tsconfig.app.json` | TypeScript config for frontend application source. |
| `frontend/tsconfig.node.json` | TypeScript config for Vite/node-side config files. |
| `frontend/tsconfig.tests.json` | TypeScript config for frontend tests. |
| `frontend/vite.config.ts` | Vite config for React, Tailwind, aliases, build, and Vitest environment. |

## Scripts

| Path | Handles |
| --- | --- |
| `scripts/run-bun.mjs` | Bun launcher shim that avoids stale PATH shims and forwards commands to the real Bun executable. |
| `scripts/smoke-sync-one-machine.ps1` | PowerShell smoke test for one-machine shared sync behavior using an isolated shared root. |

## Generated Or Local-Only Folders

| Path | Handles |
| --- | --- |
| `.git/` | Git repository metadata; never edit manually for app changes. |
| `.tmp/` | Temporary working area; non-canonical unless a task explicitly points there. |
| `backend/target/` | Rust build output, test binaries, and incremental compiler artifacts. |
| `dist/` | Built frontend output. |
| `node_modules/` | Installed JavaScript dependencies. |
| `release/` | Local release staging artifacts such as installers, signatures, and checksums. |
| `resources/data/` | Local resource data folder present in the workspace; no tracked files are currently listed under it. |

