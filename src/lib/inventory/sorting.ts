import type { ColumnKey, InventoryEntry, SortState } from "@/types/inventory";

import { formatLinkLabel } from "./columns";

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
