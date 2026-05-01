import { ArrowUpDownIcon } from "lucide-react";

import { cn } from "@/shared/lib/utils";
import type { ColumnConfig, SortState } from "@/features/inventory/types";

import { getColumnStyle } from "./columnStyles";

interface InventoryTableColumnGroupProps {
  columns: readonly ColumnConfig[];
}

interface InventoryTableHeaderProps {
  columns: readonly ColumnConfig[];
  onSortChange: (columnKey: ColumnConfig["key"]) => void;
  sortState: SortState;
}

export function InventoryTableColumnGroup({ columns }: InventoryTableColumnGroupProps) {
  return (
    <colgroup>
      {columns.map((column) => (
        <col key={column.key} style={getColumnStyle(column.key)} />
      ))}
    </colgroup>
  );
}

export function InventoryTableHeader({ columns, onSortChange, sortState }: InventoryTableHeaderProps) {
  return (
    <thead className="sticky top-0 z-10 bg-background">
      <tr>
        {columns.map((column) => (
          <th
            key={column.key}
            className={cn(
              "border-b border-border px-2.5 py-2.5 text-left text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground sm:px-4 sm:py-3",
              column.align === "center" ? "text-center" : "text-left",
            )}
            scope="col"
          >
            {column.sortable ? (
              <button
                className={cn(
                  "inline-flex min-w-0 max-w-full items-center gap-1 transition-colors hover:text-foreground",
                  column.align === "center" ? "justify-center" : "",
                  sortState.column === column.key ? "text-foreground" : "",
                )}
                type="button"
                onClick={() => onSortChange(column.key)}
              >
                <span>{column.label}</span>
                <ArrowUpDownIcon className="size-3.5" />
              </button>
            ) : (
              column.label
            )}
          </th>
        ))}
      </tr>
    </thead>
  );
}
