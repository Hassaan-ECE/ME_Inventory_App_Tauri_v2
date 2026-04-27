import type {
  ColumnConfig,
  ColumnKey,
  FilterState,
  InventoryEntry,
  InventoryScope,
  SortState,
} from "@/types/inventory";
import { INVENTORY_COLUMNS } from "@/types/inventory";

export const DEFAULT_FILTERS: FilterState = {
  assetNumber: "",
  manufacturer: "",
  model: "",
  description: "",
  location: "",
};

export function buildDefaultColumnVisibility(): Record<ColumnKey, boolean> {
  return INVENTORY_COLUMNS.reduce<Record<ColumnKey, boolean>>((visibility, column) => {
    visibility[column.key] = column.defaultVisible;
    return visibility;
  }, {} as Record<ColumnKey, boolean>);
}

export function mergeColumnVisibility(
  storedValue: Partial<Record<ColumnKey, boolean>> | null | undefined,
): Record<ColumnKey, boolean> {
  return { ...buildDefaultColumnVisibility(), ...storedValue };
}

export function getVisibleColumns(columnVisibility: Record<ColumnKey, boolean>): ColumnConfig[] {
  return INVENTORY_COLUMNS.filter((column) => columnVisibility[column.key]);
}

export function getVisibleDataColumnCount(columnVisibility: Record<ColumnKey, boolean>): number {
  let visibleColumns = 0;
  for (const column of INVENTORY_COLUMNS) {
    if (column.key !== "verified" && columnVisibility[column.key]) {
      visibleColumns += 1;
    }
  }
  return visibleColumns;
}

export function formatLinkLabel(link: string): string {
  const text = link.trim();
  if (!text) {
    return "";
  }

  try {
    const parsed = new URL(text);
    const compact = `${parsed.host}${parsed.pathname.replace(/\/$/, "")}`;
    if (compact.length <= 54) {
      return compact;
    }
    return `${compact.slice(0, 51)}...`;
  } catch {
    if (text.length <= 54) {
      return text;
    }
    return `${text.slice(0, 51)}...`;
  }
}

export function getInventoryCounts(entries: InventoryEntry[]): {
  inventory: number;
  archive: number;
  total: number;
  verified: number;
} {
  let archive = 0;
  let verified = 0;

  for (const entry of entries) {
    if (entry.archived) {
      archive += 1;
    }
    if (entry.verifiedInSurvey) {
      verified += 1;
    }
  }

  return {
    inventory: entries.length - archive,
    archive,
    total: entries.length,
    verified,
  };
}

export function hasActiveFilters(filters: FilterState): boolean {
  return (
    filters.assetNumber.trim().length > 0 ||
    filters.manufacturer.trim().length > 0 ||
    filters.model.trim().length > 0 ||
    filters.description.trim().length > 0 ||
    filters.location.trim().length > 0
  );
}

export function filterEntries(
  entries: InventoryEntry[],
  scope: InventoryScope,
  query: string,
  filters: FilterState,
): InventoryEntry[] {
  const normalizedQuery = query.trim().toLowerCase();
  const assetNumberFilter = filters.assetNumber.trim().toLowerCase();
  const manufacturerFilter = filters.manufacturer.trim().toLowerCase();
  const modelFilter = filters.model.trim().toLowerCase();
  const descriptionFilter = filters.description.trim().toLowerCase();
  const locationFilter = filters.location.trim().toLowerCase();

  return entries.filter((entry) => {
    if (scope === "inventory" && entry.archived) {
      return false;
    }
    if (scope === "archive" && !entry.archived) {
      return false;
    }

    const fieldFiltersMatch =
      includesNormalizedFilter(entry.assetNumber, assetNumberFilter) &&
      includesNormalizedFilter(entry.manufacturer, manufacturerFilter) &&
      includesNormalizedFilter(entry.model, modelFilter) &&
      includesNormalizedFilter(entry.description, descriptionFilter) &&
      includesNormalizedFilter(entry.location, locationFilter);

    if (!fieldFiltersMatch) {
      return false;
    }

    if (!normalizedQuery) {
      return true;
    }

    return entryMatchesQuery(entry, normalizedQuery);
  });
}

export function sortEntries(entries: InventoryEntry[], sortState: SortState): InventoryEntry[] {
  if (entries.length <= 1) {
    return entries;
  }

  const multiplier = sortState.direction === "asc" ? 1 : -1;

  return [...entries].sort((left, right) => {
    const leftValue = getSortValue(left, sortState.column);
    const rightValue = getSortValue(right, sortState.column);
    const leftBlank = isBlankValue(leftValue);
    const rightBlank = isBlankValue(rightValue);

    if (leftBlank && rightBlank) {
      return 0;
    }
    if (leftBlank) {
      return 1;
    }
    if (rightBlank) {
      return -1;
    }
    if (leftValue < rightValue) {
      return -1 * multiplier;
    }
    if (leftValue > rightValue) {
      return 1 * multiplier;
    }
    return 0;
  });
}

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

function includesNormalizedFilter(value: string, normalizedFilter: string): boolean {
  if (!normalizedFilter) {
    return true;
  }
  return value.toLowerCase().includes(normalizedFilter);
}

function entryMatchesQuery(entry: InventoryEntry, normalizedQuery: string): boolean {
  return (
    includesNormalizedQuery(entry.assetNumber, normalizedQuery) ||
    includesNormalizedQuery(entry.serialNumber, normalizedQuery) ||
    includesNormalizedQuery(entry.manufacturer, normalizedQuery) ||
    includesNormalizedQuery(entry.model, normalizedQuery) ||
    includesNormalizedQuery(entry.description, normalizedQuery) ||
    includesNormalizedQuery(entry.projectName, normalizedQuery) ||
    includesNormalizedQuery(entry.location, normalizedQuery) ||
    includesNormalizedQuery(entry.links, normalizedQuery) ||
    includesNormalizedQuery(entry.notes, normalizedQuery) ||
    includesNormalizedQuery(entry.lifecycleStatus, normalizedQuery) ||
    includesNormalizedQuery(entry.workingStatus, normalizedQuery)
  );
}

function includesNormalizedQuery(value: string | undefined, normalizedQuery: string): boolean {
  return value?.toLowerCase().includes(normalizedQuery) ?? false;
}

function getSortValue(entry: InventoryEntry, column: ColumnKey): number | string {
  switch (column) {
    case "verified":
      return entry.verifiedInSurvey ? 1 : 0;
    case "qty":
      return entry.qty ?? Number.POSITIVE_INFINITY;
    case "assetNumber":
      return entry.assetNumber.trim().toLowerCase();
    case "manufacturer":
      return entry.manufacturer.trim().toLowerCase();
    case "model":
      return entry.model.trim().toLowerCase();
    case "description":
      return entry.description.trim().toLowerCase();
    case "projectName":
      return entry.projectName.trim().toLowerCase();
    case "location":
      return entry.location.trim().toLowerCase();
    case "links":
      return formatLinkLabel(entry.links).toLowerCase();
  }
}

function isBlankValue(value: number | string): boolean {
  if (typeof value === "number") {
    return !Number.isFinite(value);
  }
  return value.trim().length === 0;
}
