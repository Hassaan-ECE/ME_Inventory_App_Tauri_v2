import type { ColumnConfig, ColumnKey } from "@/features/inventory/types";
import { INVENTORY_COLUMNS } from "@/features/inventory/types";

export function buildDefaultColumnVisibility(): Record<ColumnKey, boolean> {
  return INVENTORY_COLUMNS.reduce<Record<ColumnKey, boolean>>((visibility, column) => {
    visibility[column.key] = column.defaultVisible;
    return visibility;
  }, {} as Record<ColumnKey, boolean>);
}

export function mergeColumnVisibility(
  storedValue: Partial<Record<ColumnKey, boolean>> | null | undefined,
): Record<ColumnKey, boolean> {
  const visibility = { ...buildDefaultColumnVisibility(), ...storedValue };
  if (getVisibleDataColumnCount(visibility) === 0) {
    visibility[firstDefaultDataColumnKey()] = true;
  }
  return visibility;
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

function firstDefaultDataColumnKey(): ColumnKey {
  return (
    INVENTORY_COLUMNS.find((column) => column.key !== "verified" && column.defaultVisible)?.key ??
    "qty"
  );
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
