import type { FilterState, InventoryScope } from "@/features/inventory/types";

import { hasActiveFilters } from "./filtering";

export function buildResultsLabel(
  count: number,
  scope: InventoryScope,
  query: string,
  filters: FilterState,
): string {
  const filtersActive = hasActiveFilters(filters);
  const trimmedQuery = query.trim();

  if (!trimmedQuery) {
    if (scope === "archive" && count === 0 && !filtersActive) {
      return "No archived entries yet";
    }
    if (filtersActive) {
      return scope === "archive" ? `Showing ${count} filtered archived entries` : `Showing ${count} filtered entries`;
    }
    return scope === "archive" ? `Showing all ${count} archived entries` : `Showing all ${count} entries`;
  }

  if (count === 0) {
    return scope === "archive"
      ? `No archived results for "${trimmedQuery}"`
      : `No results for "${trimmedQuery}"`;
  }

  const suffix = filtersActive ? " after column filters" : "";
  const resultWord = count === 1 ? "result" : "results";
  if (scope === "archive") {
    return `${count} archived ${resultWord} for "${trimmedQuery}"${suffix}`;
  }
  return `${count} ${resultWord} for "${trimmedQuery}"${suffix}`;
}
