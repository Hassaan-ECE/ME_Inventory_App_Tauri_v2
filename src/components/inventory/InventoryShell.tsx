import { useCallback, useDeferredValue, useEffect, useMemo, useRef, useState } from "react";

import { APP_DISPLAY_NAME, APP_VERSION } from "@/branding";
import { ColumnMenu } from "@/components/inventory/ColumnMenu";
import { EmptyResults } from "@/components/inventory/EmptyResults";
import { FilterPanel } from "@/components/inventory/FilterPanel";
import { InventoryHeader } from "@/components/inventory/InventoryHeader";
import { EntryContextMenu, type EntryContextAction } from "@/components/inventory/EntryContextMenu";
import { EntryDialog } from "@/components/inventory/EntryDialog";
import { InventoryTable } from "@/components/inventory/InventoryTable";
import { SearchCard } from "@/components/inventory/SearchCard";
import { StatusStrip } from "@/components/inventory/StatusStrip";
import { Button } from "@/components/ui/button";
import { MOCK_INVENTORY } from "@/data/mockInventory";
import { toSafeExternalUrl } from "@/lib/externalUrl";
import {
  DEFAULT_FILTERS,
  buildDefaultColumnVisibility,
  buildResultsLabel,
  filterEntries,
  getInventoryCounts,
  getVisibleColumns,
  getVisibleDataColumnCount,
  mergeColumnVisibility,
  sortEntries,
} from "@/lib/inventory";
import { INVENTORY_COLUMNS } from "@/types/inventory";
import type {
  ColumnKey,
  FilterState,
  InventoryCounts,
  InventoryEntry,
  InventoryEntryInput,
  InventorySharedStatus,
  InventoryScope,
  SortState,
  ThemeMode,
  UpdateState,
} from "@/types/inventory";

const THEME_STORAGE_KEY = "meInventory.theme";
const COLOR_ROWS_STORAGE_KEY = "meInventory.colorRows";
const COLUMN_VISIBILITY_STORAGE_KEY = "meInventory.columnVisibility";
const DEFAULT_SHARED_SYNC_INTERVAL_MS = 10_000;
const MOCK_SHARED_STATUS: InventorySharedStatus = {
  available: true,
  canModify: true,
  enabled: false,
  message: "",
  mutationMode: "shared",
};
const DESKTOP_SHARED_PENDING_STATUS: InventorySharedStatus = {
  available: false,
  canModify: false,
  enabled: true,
  message: "Checking shared workspace...",
  mutationMode: "local",
  syncIntervalMs: DEFAULT_SHARED_SYNC_INTERVAL_MS,
};
const EMPTY_COUNTS: InventoryCounts = {
  archive: 0,
  inventory: 0,
  total: 0,
  verified: 0,
};
const DESKTOP_QUERY_LIMIT = 100_000;

interface DialogState {
  mode: "add" | "edit";
  entryId?: string;
}

interface ContextMenuState {
  entryId: string;
  x: number;
  y: number;
}

export function InventoryShell() {
  const [entries, setEntries] = useState<InventoryEntry[]>(() => (hasDesktopBridge() ? [] : MOCK_INVENTORY));
  const [dataSource, setDataSource] = useState<"desktop" | "mock">(() => (hasDesktopBridge() ? "desktop" : "mock"));
  const [scope, setScope] = useState<InventoryScope>("inventory");
  const [theme, setTheme] = useState<ThemeMode>(() => readTheme());
  const [query, setQuery] = useState("");
  const [filters, setFilters] = useState<FilterState>(DEFAULT_FILTERS);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [colorRows, setColorRows] = useState<boolean>(() => readColorRows());
  const [columnVisibility, setColumnVisibility] = useState<Record<ColumnKey, boolean>>(() => readColumnVisibility());
  const [sortState, setSortState] = useState<SortState>({ column: "manufacturer", direction: "asc" });
  const [isLoading, setIsLoading] = useState<boolean>(() => hasDesktopBridge());
  const [desktopCounts, setDesktopCounts] = useState<InventoryCounts>(EMPTY_COUNTS);
  const [totalFiltered, setTotalFiltered] = useState(0);
  const [sharedStatus, setSharedStatus] = useState<InventorySharedStatus>(() =>
    hasDesktopBridge() ? DESKTOP_SHARED_PENDING_STATUS : MOCK_SHARED_STATUS,
  );
  const [statusOverride, setStatusOverride] = useState<string | null>(null);
  const [dialogState, setDialogState] = useState<DialogState | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [pendingDeleteEntryId, setPendingDeleteEntryId] = useState<string | null>(null);
  const [updateState, setUpdateState] = useState<UpdateState>(() => buildIdleUpdateState());
  const statusTimeoutRef = useRef<number | null>(null);
  const syncInFlightRef = useRef(false);
  const initialSyncStartedRef = useRef(false);
  const queryRequestRef = useRef(0);

  const markSharedUnavailable = useCallback((message = "Shared workspace unavailable. Saving changes locally."): void => {
    setSharedStatus((current) => ({
      ...current,
      available: false,
      canModify: true,
      enabled: true,
      hasLocalOnlyChanges: current.hasLocalOnlyChanges,
      message,
      mutationMode: "local",
      syncIntervalMs: current.syncIntervalMs ?? DEFAULT_SHARED_SYNC_INTERVAL_MS,
    }));
  }, []);

  const deferredQuery = useDeferredValue(query);
  const deferredFilters = useDeferredValue(filters);

  const refreshDesktopQuery = useCallback(
    async ({ applyResult = true, showLoading = false }: { applyResult?: boolean; showLoading?: boolean } = {}): Promise<void> => {
      if (!window.inventoryDesktop?.queryInventory && !window.inventoryDesktop?.loadInventory) {
        return;
      }

      const requestId = queryRequestRef.current + 1;
      queryRequestRef.current = requestId;
      if (showLoading) {
        setIsLoading(true);
      }
      try {
        const payload = window.inventoryDesktop.queryInventory
          ? await window.inventoryDesktop.queryInventory({
              filters: deferredFilters,
              limit: DESKTOP_QUERY_LIMIT,
              offset: 0,
              query: deferredQuery,
              scope,
              sort: sortState,
            })
          : await loadDesktopInventoryFallback(scope, deferredQuery, deferredFilters, sortState);
        if (!applyResult || requestId !== queryRequestRef.current) {
          return;
        }
        setEntries(payload.entries);
        setDesktopCounts(payload.counts);
        setTotalFiltered(payload.totalFiltered);
        setDataSource("desktop");
        setSharedStatus((current) => (sharedStatusesMatch(current, payload.shared) ? current : payload.shared));
      } catch {
        if (applyResult) {
          setEntries(MOCK_INVENTORY);
          setDesktopCounts(EMPTY_COUNTS);
          setTotalFiltered(MOCK_INVENTORY.length);
          setDataSource("mock");
          setSharedStatus(MOCK_SHARED_STATUS);
          announceStatus("Database unavailable. Falling back to bundled mock data.");
        }
      } finally {
        if (applyResult && requestId === queryRequestRef.current) {
          setIsLoading(false);
        }
      }
    },
    [deferredFilters, deferredQuery, scope, sortState],
  );

  const syncEntriesFromDesktop = useCallback(
    async ({ applyResult = true }: { applyResult?: boolean } = {}): Promise<void> => {
      if (!window.inventoryDesktop?.syncInventory) {
        return;
      }
      if (syncInFlightRef.current) {
        return;
      }

      try {
        syncInFlightRef.current = true;
        const payload = await window.inventoryDesktop.syncInventory();
        if (!applyResult) {
          return;
        }
        setSharedStatus((current) => (sharedStatusesMatch(current, payload.shared) ? current : payload.shared));
        await refreshDesktopQuery({ applyResult });
      } catch {
        if (applyResult) {
          markSharedUnavailable();
          await refreshDesktopQuery({ applyResult });
        }
      } finally {
        syncInFlightRef.current = false;
      }
    },
    [markSharedUnavailable, refreshDesktopQuery],
  );

  useEffect(() => {
    document.title = APP_DISPLAY_NAME;
  }, []);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  }, [theme]);

  useEffect(() => {
    localStorage.setItem(COLOR_ROWS_STORAGE_KEY, JSON.stringify(colorRows));
  }, [colorRows]);

  useEffect(() => {
    localStorage.setItem(COLUMN_VISIBILITY_STORAGE_KEY, JSON.stringify(columnVisibility));
  }, [columnVisibility]);

  useEffect(() => {
    return () => {
      if (statusTimeoutRef.current !== null) {
        window.clearTimeout(statusTimeoutRef.current);
      }
    };
  }, []);

  useEffect(() => {
    let active = true;

    async function loadEntriesFromDesktop(): Promise<void> {
      if (!window.inventoryDesktop?.queryInventory && !window.inventoryDesktop?.loadInventory) {
        return;
      }

      await refreshDesktopQuery({ applyResult: active, showLoading: true });

      if (active && !initialSyncStartedRef.current) {
        initialSyncStartedRef.current = true;
        void syncEntriesFromDesktop();
      }
    }

    void loadEntriesFromDesktop();

    return () => {
      active = false;
    };
  }, [refreshDesktopQuery, syncEntriesFromDesktop]);

  useEffect(() => {
    if (!window.inventoryDesktop?.checkForUpdate) {
      return undefined;
    }

    let active = true;
    const unsubscribe = window.inventoryDesktop.onUpdateStateChanged?.((state) => {
      if (active) {
        setUpdateState(state);
      }
    });

    void window.inventoryDesktop
      .checkForUpdate()
      .then((state) => {
        if (active) {
          setUpdateState((current) => chooseFreshUpdateState(current, state));
        }
      })
      .catch(() => {
        if (active) {
          setUpdateState(buildIdleUpdateState());
        }
      });

    return () => {
      active = false;
      unsubscribe?.();
    };
  }, []);

  useEffect(() => {
    if (!window.inventoryDesktop?.syncInventory) {
      return undefined;
    }

    let active = true;
    const syncIntervalMs = sharedStatus.syncIntervalMs ?? DEFAULT_SHARED_SYNC_INTERVAL_MS;
    const intervalId = window.setInterval(() => {
      void syncEntriesFromDesktop({ applyResult: active });
    }, syncIntervalMs);
    const unsubscribe = window.inventoryDesktop.onSharedInventoryChanged?.(() => {
      void syncEntriesFromDesktop({ applyResult: active });
    });

    return () => {
      active = false;
      window.clearInterval(intervalId);
      unsubscribe?.();
    };
  }, [sharedStatus.syncIntervalMs, syncEntriesFromDesktop]);

  const filteredEntries = useMemo(
    () => filterEntries(entries, scope, deferredQuery, deferredFilters),
    [deferredFilters, deferredQuery, entries, scope],
  );
  const sortedEntries = useMemo(() => sortEntries(filteredEntries, sortState), [filteredEntries, sortState]);
  const mockCounts = useMemo(() => getInventoryCounts(entries), [entries]);
  const counts = dataSource === "desktop" ? desktopCounts : mockCounts;
  const displayEntries = dataSource === "desktop" ? entries : sortedEntries;
  const displayCount = dataSource === "desktop" ? totalFiltered : sortedEntries.length;
  const canModifyEntries = dataSource !== "desktop" || sharedStatus.canModify;
  const resultsLabel = isLoading
    ? "Loading inventory entries..."
    : buildResultsLabel(displayCount, scope, deferredQuery, deferredFilters);
  const visibleColumns = useMemo(() => getVisibleColumns(columnVisibility), [columnVisibility]);
  const entriesById = useMemo(() => new Map(displayEntries.map((entry) => [entry.id, entry])), [displayEntries]);
  const statusMessage = isLoading
    ? "Loading ME inventory database..."
    : statusOverride ?? buildDefaultStatusMessage(counts.total, counts.verified, dataSource, sharedStatus);
  const dialogEntry = dialogState?.mode === "edit" ? entriesById.get(dialogState.entryId ?? "") ?? null : null;
  const contextEntry = contextMenu ? entriesById.get(contextMenu.entryId) ?? null : null;
  const pendingDeleteEntry = pendingDeleteEntryId ? entriesById.get(pendingDeleteEntryId) ?? null : null;

  function announceStatus(message: string): void {
    setStatusOverride(message);

    if (statusTimeoutRef.current !== null) {
      window.clearTimeout(statusTimeoutRef.current);
    }

    statusTimeoutRef.current = window.setTimeout(() => {
      setStatusOverride(null);
    }, 2400);
  }

  async function runDesktopMutation<Result>(operation: () => Promise<Result>): Promise<Result> {
    return operation();
  }

  function applyDesktopMutationFeedback(result: { message: string; shared?: InventorySharedStatus }): void {
    if (result.shared) {
      setSharedStatus(result.shared);
    }
    announceStatus(result.message);
  }

  function handleThemeToggle(): void {
    setTheme((current) => (current === "light" ? "dark" : "light"));
  }

  function handleFilterChange(field: keyof FilterState, value: string): void {
    setFilters((current) => ({ ...current, [field]: value }));
  }

  function handleClearFilters(): void {
    setFilters(DEFAULT_FILTERS);
  }

  function handleSortChange(column: ColumnKey): void {
    setSortState((current) => ({
      column,
      direction: current.column === column && current.direction === "asc" ? "desc" : "asc",
    }));
  }

  function handleAddEntry(): void {
    if (!canModifyEntries) {
      announceStatus(sharedStatus.message || "Shared workspace unavailable. Saving changes locally.");
      return;
    }
    setContextMenu(null);
    setDialogState({ mode: "add" });
  }

  function handleOpenEntry(entryId: string): void {
    setContextMenu(null);
    setDialogState({ mode: "edit", entryId });
  }

  function handleOpenContextMenu(entryId: string, clientX: number, clientY: number): void {
    const menuWidth = 240;
    const entry = entriesById.get(entryId);
    const menuHeight = entry?.links.trim() ? 212 : 172;
    const maxX = typeof window === "undefined" ? clientX : Math.max(12, window.innerWidth - menuWidth - 12);
    const maxY = typeof window === "undefined" ? clientY : Math.max(12, window.innerHeight - menuHeight - 12);

    setContextMenu({
      entryId,
      x: Math.min(clientX, maxX),
      y: Math.min(clientY, maxY),
    });
  }

  async function handleToggleVerified(entryId: string): Promise<void> {
    if (dataSource === "desktop" && !canModifyEntries) {
      announceStatus(sharedStatus.message || "Shared workspace unavailable. Saving changes locally.");
      return;
    }

    const nextVerified = !entriesById.get(entryId)?.verifiedInSurvey;

    if (dataSource === "desktop" && window.inventoryDesktop?.toggleVerifiedEntry) {
      try {
        const result = await runDesktopMutation(() => window.inventoryDesktop!.toggleVerifiedEntry(entryId, nextVerified));
        setEntries((current) =>
          current.map((entry) => (entry.id === entryId ? result.entry : entry)),
        );
        applyDesktopMutationFeedback(result);
        void refreshDesktopQuery();
        return;
      } catch {
        announceStatus("Could not update the ME Inventory database.");
        return;
      }
    }

    setEntries((current) =>
      current.map((entry) =>
        entry.id === entryId ? { ...entry, verifiedInSurvey: !entry.verifiedInSurvey } : entry,
      ),
    );
    announceStatus("Verified state updated locally.");
  }

  async function handleSaveEntry(input: InventoryEntryInput): Promise<void> {
    if (dataSource === "desktop" && !canModifyEntries) {
      throw new Error(sharedStatus.message || "Shared workspace unavailable. Saving changes locally.");
    }

    if (dialogState?.mode === "edit" && dialogState.entryId) {
      const existingEntry = entriesById.get(dialogState.entryId);
      if (!existingEntry) {
        throw new Error("The selected entry could not be found.");
      }

      if (dataSource === "desktop" && window.inventoryDesktop?.updateEntry) {
        const result = await runDesktopMutation(() => window.inventoryDesktop!.updateEntry(dialogState.entryId!, input));
        setEntries((current) => current.map((entry) => (entry.id === result.entry.id ? result.entry : entry)));
        applyDesktopMutationFeedback(result);
        void refreshDesktopQuery();
      } else {
        const updatedEntry = buildLocalUpdatedEntry(existingEntry, input);
        setEntries((current) => current.map((entry) => (entry.id === updatedEntry.id ? updatedEntry : entry)));
        announceStatus("Entry updated locally.");
      }

      setDialogState(null);
      return;
    }

    if (dataSource === "desktop" && window.inventoryDesktop?.createEntry) {
      const result = await runDesktopMutation(() => window.inventoryDesktop!.createEntry(input));
      setEntries((current) => [result.entry, ...current.filter((entry) => entry.id !== result.entry.id)]);
      applyDesktopMutationFeedback(result);
      void refreshDesktopQuery();
    } else {
      const createdEntry = buildLocalCreatedEntry(input);
      setEntries((current) => [createdEntry, ...current]);
      announceStatus("Entry added locally.");
    }

    setDialogState(null);
  }

  async function handleArchiveChange(entryId: string, archived: boolean): Promise<void> {
    if (dataSource === "desktop" && !canModifyEntries) {
      announceStatus(sharedStatus.message || "Shared workspace unavailable. Saving changes locally.");
      return;
    }

    const entry = entriesById.get(entryId);
    if (!entry || entry.archived === archived) {
      return;
    }

    if (dataSource === "desktop" && window.inventoryDesktop?.setArchivedEntry) {
      try {
        const result = await runDesktopMutation(() => window.inventoryDesktop!.setArchivedEntry(entryId, archived));
        setEntries((current) => current.map((entry) => (entry.id === result.entry.id ? result.entry : entry)));
        applyDesktopMutationFeedback(result);
        void refreshDesktopQuery();
      } catch {
        announceStatus("Could not update the shared inventory database.");
        return;
      }
    } else {
      setEntries((current) =>
        current.map((entry) => (entry.id === entryId ? { ...entry, archived, updatedAt: new Date().toISOString() } : entry)),
      );
      announceStatus(archived ? "Entry moved to the archive." : "Entry restored to inventory.");
    }
  }

  function handleRequestDeleteEntry(entryId: string): void {
    if (dataSource === "desktop" && !canModifyEntries) {
      announceStatus(sharedStatus.message || "Shared workspace unavailable. Saving changes locally.");
      return;
    }

    const entry = entriesById.get(entryId);
    if (!entry) {
      return;
    }

    setPendingDeleteEntryId(entryId);
  }

  async function handleConfirmDeleteEntry(entryId: string): Promise<void> {
    setPendingDeleteEntryId(null);

    if (dataSource === "desktop" && window.inventoryDesktop?.deleteEntry) {
      try {
        const result = await runDesktopMutation(() => window.inventoryDesktop!.deleteEntry(entryId));
        setEntries((current) => current.filter((entry) => entry.id !== entryId));
        applyDesktopMutationFeedback(result);
        void refreshDesktopQuery();
        return;
      } catch {
        announceStatus("Could not delete from the shared inventory database.");
        return;
      }
    }

    setEntries((current) => current.filter((entry) => entry.id !== entryId));
    announceStatus("Entry deleted.");
  }

  async function handleExportExcel(): Promise<void> {
    if (!window.inventoryDesktop?.exportExcel) {
      announceStatus("Excel export is only available in the desktop app.");
      return;
    }

    try {
      const result = await window.inventoryDesktop.exportExcel();
      if (result.canceled) {
        return;
      }
      if (result.error) {
        announceStatus("Excel export failed.");
        return;
      }

      announceStatus("Excel export completed.");
    } catch {
      announceStatus("Excel export failed.");
    }
  }

  async function handleUpdateAction(): Promise<void> {
    if (!window.inventoryDesktop) {
      return;
    }

    try {
      if (updateState.status === "ready") {
        const nextState = await window.inventoryDesktop.installUpdate?.();
        if (nextState) {
          setUpdateState(nextState);
          if (nextState.status === "error" && nextState.error) {
            announceStatus(nextState.error);
          }
        }
        return;
      }

      if (updateState.status === "downloading" || updateState.status === "checking" || updateState.status === "installing") {
        return;
      }

      if (!window.inventoryDesktop.downloadUpdate) {
        announceStatus("Update download is only available in the desktop app.");
        return;
      }

      if (updateState.available) {
        setUpdateState((current) => ({ ...current, status: "downloading" }));
      }
      const nextState = await window.inventoryDesktop.downloadUpdate();
      setUpdateState((current) => chooseFreshUpdateState(current, nextState));
      if (nextState.status === "error" && nextState.error) {
        announceStatus(nextState.error);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Update failed.";
      setUpdateState((current) => ({ ...current, error: message, status: "error" }));
      announceStatus(message);
    }
  }

  function handleExportHtml(): void {
    announceStatus("HTML export is not implemented yet.");
  }

  async function handleOpenEntryLink(entryId: string): Promise<void> {
    const entry = entriesById.get(entryId);
    if (!entry) {
      return;
    }

    const linkText = entry.links.trim();
    if (!linkText) {
      announceStatus("No link is saved for this entry.");
      return;
    }

    const externalUrl = toSafeExternalUrl(linkText);
    if (!externalUrl) {
      announceStatus("This link is not in a valid format.");
      return;
    }

    const opened = await openExternalUrl(externalUrl);
    if (!opened) {
      announceStatus("Could not open the saved link.");
      return;
    }

    announceStatus(`Opened link: ${linkText}`);
  }

  async function handleSearchOnline(entryId: string): Promise<void> {
    const entry = entriesById.get(entryId);
    if (!entry) {
      return;
    }

    const queryText = [entry.manufacturer, entry.model, entry.description].filter((value) => value.trim()).join(" ").trim();
    if (!queryText) {
      announceStatus("No searchable entry details were found.");
      return;
    }

    const searchUrl = toSafeExternalUrl(`https://www.google.com/search?q=${encodeURIComponent(queryText)}`, {
      allowImplicitHttps: false,
    });
    if (!searchUrl) {
      announceStatus("Could not build a safe browser search URL.");
      return;
    }
    const opened = await openExternalUrl(searchUrl);
    if (!opened) {
      announceStatus("Could not open the browser for this search.");
      return;
    }

    announceStatus(`Opened web search: ${queryText}`);
  }

  async function handleContextAction(action: EntryContextAction): Promise<void> {
    const entryId = contextMenu?.entryId;
    setContextMenu(null);

    if (!entryId) {
      return;
    }

    switch (action) {
      case "open":
        handleOpenEntry(entryId);
        return;
      case "open-link":
        await handleOpenEntryLink(entryId);
        return;
      case "search-online":
        await handleSearchOnline(entryId);
        return;
      case "archive-toggle": {
        const entry = entriesById.get(entryId);
        if (!entry) {
          return;
        }
        await handleArchiveChange(entryId, !entry.archived);
        return;
      }
      case "delete":
        handleRequestDeleteEntry(entryId);
        return;
    }
  }

  function handleToggleColumn(columnKey: ColumnKey): void {
    setColumnVisibility((current) => {
      const nextValue = !current[columnKey];
      const visibleDataColumns = getVisibleDataColumnCount(current);

      if (!nextValue && columnKey !== "verified" && visibleDataColumns === 1) {
        return current;
      }

      return { ...current, [columnKey]: nextValue };
    });
  }

  return (
    <div className="h-screen overflow-hidden bg-background text-foreground">
      <main className="flex h-full min-h-0 flex-col overflow-hidden">
        <InventoryHeader
          archiveCount={counts.archive}
          canModifyEntries={canModifyEntries}
          inventoryCount={counts.inventory}
          onAddEntry={handleAddEntry}
          onExportExcel={() => {
            void handleExportExcel();
          }}
          onExportHtml={handleExportHtml}
          onScopeChange={setScope}
          onThemeToggle={handleThemeToggle}
          scope={scope}
          theme={theme}
          updateState={updateState}
          onUpdateAction={() => {
            void handleUpdateAction();
          }}
        />

        <div className="flex min-h-0 flex-1 overflow-hidden px-3 py-4 sm:px-5">
          <div className="flex min-h-0 w-full flex-1 flex-col gap-4 overflow-hidden">
            <SearchCard
              colorRows={colorRows}
              columnMenu={
                <ColumnMenu columns={INVENTORY_COLUMNS} onToggleColumn={handleToggleColumn} visibility={columnVisibility} />
              }
              filtersOpen={filtersOpen}
              onColorRowsChange={setColorRows}
              onFiltersToggle={() => setFiltersOpen((current) => !current)}
              onQueryChange={setQuery}
              query={query}
              resultsLabel={resultsLabel}
              scope={scope}
            />

            {filtersOpen ? <FilterPanel filters={filters} onChange={handleFilterChange} onClear={handleClearFilters} /> : null}

            <div className="min-h-0 flex-1 overflow-hidden">
              {isLoading ? (
                <section className="flex h-full min-h-0 flex-1 items-center justify-center rounded-3xl border border-border/70 bg-card/80 shadow-sm">
                  <div className="text-sm text-muted-foreground">Loading ME inventory database...</div>
                </section>
              ) : displayEntries.length > 0 ? (
                <InventoryTable
                  activeEntryId={contextMenu?.entryId ?? dialogEntry?.id ?? null}
                  canModifyEntries={canModifyEntries}
                  colorRows={colorRows}
                  columns={visibleColumns}
                  onOpenContextMenu={handleOpenContextMenu}
                  onOpenEntry={handleOpenEntry}
                  onSortChange={handleSortChange}
                  onToggleVerified={(entryId) => {
                    void handleToggleVerified(entryId);
                  }}
                  entries={displayEntries}
                  sortState={sortState}
                />
              ) : (
                <EmptyResults query={query} scope={scope} onAddEntry={handleAddEntry} />
              )}
            </div>
          </div>
        </div>

        <StatusStrip message={statusMessage} />
      </main>

      {contextMenu && contextEntry ? (
        <EntryContextMenu
          canModifyEntries={canModifyEntries}
          position={{ x: contextMenu.x, y: contextMenu.y }}
          entry={contextEntry}
          scope={scope}
          onAction={(action) => {
            void handleContextAction(action);
          }}
          onClose={() => setContextMenu(null)}
        />
      ) : null}

      {dialogState ? (
        <EntryDialog
          key={`${dialogState.mode}-${dialogState.entryId ?? scope}`}
          defaultArchived={scope === "archive"}
          mode={dialogState.mode}
          readOnly={dataSource === "desktop" && !canModifyEntries}
          entry={dialogEntry}
          onClose={() => setDialogState(null)}
          onSave={handleSaveEntry}
        />
      ) : null}

      {pendingDeleteEntry ? (
        <DeleteConfirmationDialog
          entry={pendingDeleteEntry}
          onCancel={() => setPendingDeleteEntryId(null)}
          onConfirm={() => {
            void handleConfirmDeleteEntry(pendingDeleteEntry.id);
          }}
        />
      ) : null}
    </div>
  );
}

interface DeleteConfirmationDialogProps {
  entry: InventoryEntry;
  onCancel: () => void;
  onConfirm: () => void;
}

function DeleteConfirmationDialog({ entry, onCancel, onConfirm }: DeleteConfirmationDialogProps) {
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape") {
        onCancel();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onCancel]);

  const title = entry.description || entry.manufacturer || entry.model || `ID ${entry.id}`;

  return (
    <div
      aria-modal="true"
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/45 p-4"
      role="dialog"
      onClick={(event) => {
        if (event.target === event.currentTarget) {
          onCancel();
        }
      }}
    >
      <section className="w-full max-w-md rounded-2xl border border-border/70 bg-card p-5 text-card-foreground shadow-2xl">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-destructive-foreground">Delete Entry</p>
          <h2 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Delete this entry?</h2>
          <p className="mt-3 text-sm text-muted-foreground">
            This removes the entry from the inventory database.
          </p>
        </div>

        <div className="mt-4 rounded-xl border border-border/70 bg-background/70 px-4 py-3">
          <p className="text-sm font-medium text-foreground">{title}</p>
          {entry.assetNumber || entry.location ? (
            <p className="mt-1 text-xs text-muted-foreground">
              {[entry.assetNumber, entry.location].filter(Boolean).join(" | ")}
            </p>
          ) : null}
        </div>

        <div className="mt-5 flex justify-end gap-2">
          <Button variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
          <Button className="border-destructive bg-destructive text-white hover:bg-destructive/90" onClick={onConfirm}>
            Delete Entry
          </Button>
        </div>
      </section>
    </div>
  );
}

function buildIdleUpdateState(): UpdateState {
  return {
    available: false,
    currentVersion: APP_VERSION,
    status: "idle",
  };
}

function chooseFreshUpdateState(current: UpdateState, next: UpdateState): UpdateState {
  if (current.latestVersion && current.latestVersion === next.latestVersion) {
    return getUpdateStatusRank(current.status) > getUpdateStatusRank(next.status) ? current : next;
  }

  return next;
}

function getUpdateStatusRank(status: UpdateState["status"]): number {
  switch (status) {
    case "idle":
      return 0;
    case "checking":
      return 1;
    case "not-available":
      return 2;
    case "available":
      return 3;
    case "downloading":
      return 4;
    case "ready":
      return 5;
    case "installing":
      return 6;
    case "error":
      return 7;
    default:
      return 0;
  }
}

function sharedStatusesMatch(left: InventorySharedStatus, right: InventorySharedStatus): boolean {
  return (
    left.available === right.available &&
    left.canModify === right.canModify &&
    left.enabled === right.enabled &&
    left.hasLocalOnlyChanges === right.hasLocalOnlyChanges &&
    left.message === right.message &&
    left.mutationMode === right.mutationMode &&
    left.revision === right.revision &&
    left.sharedDbPath === right.sharedDbPath &&
    left.sharedRootPath === right.sharedRootPath &&
    left.syncIntervalMs === right.syncIntervalMs
  );
}

function hasDesktopBridge(): boolean {
  return typeof window !== "undefined" && Boolean(window.inventoryDesktop?.isDesktop);
}

async function loadDesktopInventoryFallback(
  scope: InventoryScope,
  query: string,
  filters: FilterState,
  sortState: SortState,
) {
  const payload = await window.inventoryDesktop!.loadInventory();
  const filtered = filterEntries(payload.entries, scope, query, filters);
  const sorted = sortEntries(filtered, sortState);

  return {
    counts: getInventoryCounts(payload.entries),
    dbPath: payload.dbPath,
    entries: sorted,
    shared: payload.shared,
    totalFiltered: sorted.length,
  };
}

function buildDefaultStatusMessage(
  totalCount: number,
  verifiedCount: number,
  dataSource: "desktop" | "mock",
  sharedStatus: InventorySharedStatus,
): string {
  const summary = `Total: ${totalCount} | Verified: ${verifiedCount}/${totalCount}`;
  if (dataSource !== "desktop" || !sharedStatus.message) {
    return summary;
  }
  return `${summary} | ${sharedStatus.message}`;
}

async function openExternalUrl(url: string): Promise<boolean> {
  const externalUrl = toSafeExternalUrl(url, { allowImplicitHttps: false });
  if (!externalUrl) {
    return false;
  }

  if (window.inventoryDesktop?.openExternal) {
    return window.inventoryDesktop.openExternal(externalUrl);
  }

  window.open(externalUrl, "_blank", "noopener,noreferrer");
  return true;
}

function buildLocalCreatedEntry(input: InventoryEntryInput): InventoryEntry {
  const timestamp = new Date().toISOString();

  return {
    id: `local-${Date.now()}`,
    entryUuid: "",
    assetNumber: input.assetNumber,
    serialNumber: input.serialNumber,
    qty: input.qty,
    manufacturer: input.manufacturer,
    model: input.model,
    description: input.description,
    projectName: input.projectName,
    location: input.location,
    assignedTo: input.assignedTo,
    links: input.links,
    notes: input.notes,
    lifecycleStatus: input.lifecycleStatus,
    workingStatus: input.workingStatus,
    condition: input.condition,
    verifiedInSurvey: input.verifiedInSurvey,
    archived: input.archived,
    manualEntry: true,
    picturePath: input.picturePath ?? "",
    createdAt: timestamp,
    updatedAt: timestamp,
  };
}

function buildLocalUpdatedEntry(existingEntry: InventoryEntry, input: InventoryEntryInput): InventoryEntry {
  return {
    ...existingEntry,
    assetNumber: input.assetNumber,
    serialNumber: input.serialNumber,
    qty: input.qty,
    manufacturer: input.manufacturer,
    model: input.model,
    description: input.description,
    projectName: input.projectName,
    location: input.location,
    assignedTo: input.assignedTo,
    links: input.links,
    notes: input.notes,
    lifecycleStatus: input.lifecycleStatus,
    workingStatus: input.workingStatus,
    condition: input.condition,
    verifiedInSurvey: input.verifiedInSurvey,
    archived: input.archived,
    picturePath: input.picturePath ?? "",
    updatedAt: new Date().toISOString(),
  };
}

function readTheme(): ThemeMode {
  if (typeof window === "undefined") {
    return "light";
  }

  const storedTheme = window.localStorage.getItem(THEME_STORAGE_KEY);
  return storedTheme === "dark" ? "dark" : "light";
}

function readColorRows(): boolean {
  if (typeof window === "undefined") {
    return true;
  }

  const storedValue = window.localStorage.getItem(COLOR_ROWS_STORAGE_KEY);
  return storedValue == null ? true : storedValue === "true";
}

function readColumnVisibility(): Record<ColumnKey, boolean> {
  if (typeof window === "undefined") {
    return buildDefaultColumnVisibility();
  }

  const storedValue = window.localStorage.getItem(COLUMN_VISIBILITY_STORAGE_KEY);
  if (!storedValue) {
    return buildDefaultColumnVisibility();
  }

  try {
    return mergeColumnVisibility(JSON.parse(storedValue) as Partial<Record<ColumnKey, boolean>>);
  } catch {
    return buildDefaultColumnVisibility();
  }
}
