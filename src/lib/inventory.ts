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
  return INVENTORY_COLUMNS.filter((column) => column.key !== "verified" && columnVisibility[column.key]).length;
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
  const archive = entries.filter((entry) => entry.archived).length;
  const verified = entries.filter((entry) => entry.verifiedInSurvey).length;

  return {
    inventory: entries.length - archive,
    archive,
    total: entries.length,
    verified,
  };
}

export function hasActiveFilters(filters: FilterState): boolean {
  return Object.values(filters).some((value) => value.trim().length > 0);
}

export function filterEntries(
  entries: InventoryEntry[],
  scope: InventoryScope,
  query: string,
  filters: FilterState,
): InventoryEntry[] {
  const normalizedQuery = query.trim().toLowerCase();

  return entries.filter((entry) => {
    if (scope === "inventory" && entry.archived) {
      return false;
    }
    if (scope === "archive" && !entry.archived) {
      return false;
    }

    const fieldFiltersMatch =
      includesText(entry.assetNumber, filters.assetNumber) &&
      includesText(entry.manufacturer, filters.manufacturer) &&
      includesText(entry.model, filters.model) &&
      includesText(entry.description, filters.description) &&
      includesText(entry.location, filters.location);

    if (!fieldFiltersMatch) {
      return false;
    }

    if (!normalizedQuery) {
      return true;
    }

    return [
      entry.assetNumber,
      entry.serialNumber ?? "",
      entry.manufacturer,
      entry.model,
      entry.description,
      entry.projectName,
      entry.location,
      entry.links,
      entry.notes,
      entry.lifecycleStatus,
      entry.workingStatus,
    ].some((value) => value.toLowerCase().includes(normalizedQuery));
  });
}

export function sortEntries(entries: InventoryEntry[], sortState: SortState): InventoryEntry[] {
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

function includesText(value: string, filterValue: string): boolean {
  const filter = filterValue.trim().toLowerCase();
  if (!filter) {
    return true;
  }
  return value.toLowerCase().includes(filter);
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
