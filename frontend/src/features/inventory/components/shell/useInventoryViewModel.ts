import { useDeferredValue, useMemo } from "react";

import {
  buildResultsLabel,
  filterEntries,
  getInventoryCounts,
  getVisibleColumns,
  sortEntries,
} from "@/features/inventory/lib";
import type {
  ColumnKey,
  FilterState,
  InventoryEntry,
  InventoryScope,
  SortState,
} from "@/features/inventory/types";

interface UseInventoryViewModelOptions {
  columnVisibility: Record<ColumnKey, boolean>;
  entries: InventoryEntry[];
  filters: FilterState;
  isLoading: boolean;
  query: string;
  scope: InventoryScope;
  sortState: SortState;
}

const LOADING_ENTRIES: InventoryEntry[] = [];

export function useInventoryViewModel({
  columnVisibility,
  entries,
  filters,
  isLoading,
  query,
  scope,
  sortState,
}: UseInventoryViewModelOptions) {
  const sourceEntries = isLoading ? LOADING_ENTRIES : entries;
  const deferredQuery = useDeferredValue(query);
  const deferredFilters = useDeferredValue(filters);
  const filteredEntries = useMemo(
    () => filterEntries(sourceEntries, scope, deferredQuery, deferredFilters),
    [deferredFilters, deferredQuery, scope, sourceEntries],
  );
  const sortedEntries = useMemo(() => sortEntries(filteredEntries, sortState), [filteredEntries, sortState]);
  const counts = useMemo(() => getInventoryCounts(sourceEntries), [sourceEntries]);
  const visibleColumns = useMemo(() => getVisibleColumns(columnVisibility), [columnVisibility]);
  const entriesById = useMemo(() => {
    const map = new Map<string, InventoryEntry>();
    for (const entry of sortedEntries) {
      map.set(entry.id, entry);
    }
    return map;
  }, [sortedEntries]);

  return {
    counts,
    deferredFilters,
    deferredQuery,
    displayCount: sortedEntries.length,
    displayEntries: sortedEntries,
    entriesById,
    resultsLabel: isLoading
      ? "Loading inventory entries..."
      : buildResultsLabel(sortedEntries.length, scope, deferredQuery, deferredFilters),
    visibleColumns,
  };
}
