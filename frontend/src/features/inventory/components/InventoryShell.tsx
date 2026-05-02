import { useEffect, useState } from "react";

import { APP_DISPLAY_NAME } from "@/app/branding";
import { ColumnMenu } from "@/features/inventory/components/ColumnMenu";
import { DeleteConfirmationDialog } from "@/features/inventory/components/shell/DeleteConfirmationDialog";
import { EmptyResults } from "@/features/inventory/components/EmptyResults";
import { FilterPanel } from "@/features/inventory/components/FilterPanel";
import { InventoryHeader } from "@/features/inventory/components/InventoryHeader";
import { EntryContextMenu, type EntryContextAction } from "@/features/inventory/components/EntryContextMenu";
import { EntryDialog } from "@/features/inventory/components/EntryDialog";
import { InventoryTable } from "@/features/inventory/components/InventoryTable";
import { SearchCard } from "@/features/inventory/components/SearchCard";
import { StatusStrip } from "@/features/inventory/components/StatusStrip";
import {
  buildDefaultStatusMessage,
  buildLocalCreatedEntry,
  buildLocalUpdatedEntry,
  normalizeSharedStatus,
  sharedStatusesMatch,
} from "@/features/inventory/components/shell/helpers";
import { useDesktopInventory } from "@/features/inventory/components/shell/useDesktopInventory";
import { useDesktopUpdates } from "@/features/inventory/components/shell/useDesktopUpdates";
import { useInventoryPreferences } from "@/features/inventory/components/shell/useInventoryPreferences";
import { useInventoryViewModel } from "@/features/inventory/components/shell/useInventoryViewModel";
import { useStatusAnnouncer } from "@/features/inventory/components/shell/useStatusAnnouncer";
import { toSafeExternalUrl } from "@/shared/lib/externalUrl";
import { DEFAULT_FILTERS, getVisibleDataColumnCount } from "@/features/inventory/lib";
import { INVENTORY_COLUMNS } from "@/features/inventory/types";
import type {
  ColumnKey,
  FilterState,
  InventoryEntryEditContext,
  InventoryEntryInput,
  InventorySharedStatus,
  InventoryScope,
  SortState,
} from "@/features/inventory/types";

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
  const { announceStatus, statusOverride } = useStatusAnnouncer();
  const {
    dataSource,
    entries,
    isLoading,
    scheduleDesktopSync,
    setEntries,
    setSharedStatus,
    sharedStatus,
  } = useDesktopInventory({ announceStatus });
  const { handleUpdateAction, updateState } = useDesktopUpdates({ announceStatus });
  const {
    colorRows,
    columnVisibility,
    handleThemeToggle,
    setColorRows,
    setColumnVisibility,
    theme,
  } = useInventoryPreferences();
  const [scope, setScope] = useState<InventoryScope>("inventory");
  const [query, setQuery] = useState("");
  const [filters, setFilters] = useState<FilterState>(DEFAULT_FILTERS);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [sortState, setSortState] = useState<SortState>({ column: "manufacturer", direction: "asc" });
  const [dialogState, setDialogState] = useState<DialogState | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [pendingDeleteEntryId, setPendingDeleteEntryId] = useState<string | null>(null);
  const {
    counts,
    displayEntries,
    entriesById,
    resultsLabel,
    visibleColumns,
  } = useInventoryViewModel({
    columnVisibility,
    entries,
    filters,
    isLoading,
    query,
    scope,
    sortState,
  });
  const canModifyEntries = dataSource !== "desktop" || sharedStatus.canModify;
  const statusMessage = isLoading
    ? "Loading ME inventory database..."
    : statusOverride ?? buildDefaultStatusMessage(counts.total, counts.verified, dataSource, sharedStatus);
  const dialogEntry = dialogState?.mode === "edit" ? entriesById.get(dialogState.entryId ?? "") ?? null : null;
  const contextEntry = contextMenu ? entriesById.get(contextMenu.entryId) ?? null : null;
  const pendingDeleteEntry = pendingDeleteEntryId ? entriesById.get(pendingDeleteEntryId) ?? null : null;

  useEffect(() => {
    document.title = APP_DISPLAY_NAME;
  }, []);

  async function runDesktopMutation<Result>(operation: () => Promise<Result>): Promise<Result> {
    return operation();
  }

  function applyDesktopMutationFeedback(result: { message: string; shared?: InventorySharedStatus }): void {
    if (result.shared) {
      const shared = normalizeSharedStatus(result.shared);
      setSharedStatus((current) => (sharedStatusesMatch(current, shared) ? current : shared));
    }
    announceStatus(result.message);
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
        setEntries((current) => current.map((entry) => (entry.id === entryId ? result.entry : entry)));
        applyDesktopMutationFeedback(result);
        scheduleDesktopSync();
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

  async function handleSaveEntry(input: InventoryEntryInput, editContext?: InventoryEntryEditContext): Promise<void> {
    if (dataSource === "desktop" && !canModifyEntries) {
      throw new Error(sharedStatus.message || "Shared workspace unavailable. Saving changes locally.");
    }

    if (dialogState?.mode === "edit" && dialogState.entryId) {
      const existingEntry = entriesById.get(dialogState.entryId);
      if (!existingEntry) {
        throw new Error("The selected entry could not be found.");
      }

      if (dataSource === "desktop" && window.inventoryDesktop?.updateEntry) {
        const result = await runDesktopMutation(() => window.inventoryDesktop!.updateEntry(dialogState.entryId!, input, editContext));
        setEntries((current) =>
          current.map((entry) =>
            entry.id === dialogState.entryId ||
            entry.id === result.entry.id ||
            (entry.entryUuid && result.entry.entryUuid && entry.entryUuid === result.entry.entryUuid)
              ? result.entry
              : entry,
          ),
        );
        applyDesktopMutationFeedback(result);
        scheduleDesktopSync();
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
      scheduleDesktopSync();
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
        scheduleDesktopSync();
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
        scheduleDesktopSync();
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

  function handleExportHtml(): void {
    announceStatus("HTML export is not implemented yet.");
  }

  async function handleOpenExternalLink(url: string, successMessage = "Opened link."): Promise<void> {
    const opened = await openExternalUrl(url);
    if (!opened) {
      announceStatus("Could not open the saved link.");
      return;
    }

    announceStatus(successMessage);
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

    await handleOpenExternalLink(externalUrl, `Opened link: ${linkText}`);
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
                  onOpenExternalLink={(url) => {
                    void handleOpenExternalLink(url);
                  }}
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
