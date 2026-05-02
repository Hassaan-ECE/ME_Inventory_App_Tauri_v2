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
import { buildDefaultStatusMessage } from "@/features/inventory/components/shell/helpers";
import { useDesktopInventory } from "@/features/inventory/components/shell/useDesktopInventory";
import { useDesktopUpdates } from "@/features/inventory/components/shell/useDesktopUpdates";
import { useInventoryEntryMutations } from "@/features/inventory/components/shell/useInventoryEntryMutations";
import { useInventoryExportActions } from "@/features/inventory/components/shell/useInventoryExportActions";
import { useInventoryExternalActions } from "@/features/inventory/components/shell/useInventoryExternalActions";
import { useInventoryPreferences } from "@/features/inventory/components/shell/useInventoryPreferences";
import { useInventoryViewModel } from "@/features/inventory/components/shell/useInventoryViewModel";
import { useStatusAnnouncer } from "@/features/inventory/components/shell/useStatusAnnouncer";
import { DEFAULT_FILTERS, getVisibleDataColumnCount } from "@/features/inventory/lib";
import { INVENTORY_COLUMNS } from "@/features/inventory/types";
import type { ColumnKey, FilterState, InventoryScope, SortState } from "@/features/inventory/types";

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
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
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
  const {
    cancelDeleteEntry,
    closeDialog,
    dialogState,
    handleAddEntry,
    handleArchiveChange,
    handleConfirmDeleteEntry,
    handleOpenEntry,
    handleRequestDeleteEntry,
    handleSaveEntry,
    handleToggleVerified,
    pendingDeleteEntryId,
  } = useInventoryEntryMutations({
    announceStatus,
    canModifyEntries,
    dataSource,
    entriesById,
    onDialogOpen: () => setContextMenu(null),
    scheduleDesktopSync,
    setEntries,
    setSharedStatus,
    sharedStatus,
  });
  const { handleExportExcel, handleExportHtml } = useInventoryExportActions({ announceStatus });
  const { handleOpenEntryLink, handleOpenExternalLink, handleSearchOnline } = useInventoryExternalActions({
    announceStatus,
    entriesById,
  });
  const statusMessage = isLoading
    ? "Loading ME inventory database..."
    : statusOverride ?? buildDefaultStatusMessage(counts.total, counts.verified, dataSource, sharedStatus);
  const dialogEntry = dialogState?.mode === "edit" ? entriesById.get(dialogState.entryId ?? "") ?? null : null;
  const contextEntry = contextMenu ? entriesById.get(contextMenu.entryId) ?? null : null;
  const pendingDeleteEntry = pendingDeleteEntryId ? entriesById.get(pendingDeleteEntryId) ?? null : null;

  useEffect(() => {
    document.title = APP_DISPLAY_NAME;
  }, []);

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
          onClose={closeDialog}
          onSave={handleSaveEntry}
        />
      ) : null}

      {pendingDeleteEntry ? (
        <DeleteConfirmationDialog
          entry={pendingDeleteEntry}
          onCancel={cancelDeleteEntry}
          onConfirm={() => {
            void handleConfirmDeleteEntry(pendingDeleteEntry.id);
          }}
        />
      ) : null}
    </div>
  );
}
