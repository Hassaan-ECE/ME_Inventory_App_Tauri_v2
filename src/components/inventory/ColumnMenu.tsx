import { ChevronDownIcon, SlidersHorizontalIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { getVisibleDataColumnCount } from "@/lib/inventory";
import type { ColumnConfig, ColumnKey } from "@/types/inventory";

interface ColumnMenuProps {
  columns: readonly ColumnConfig[];
  onToggleColumn: (columnKey: ColumnKey) => void;
  visibility: Record<ColumnKey, boolean>;
}

export function ColumnMenu({ columns, onToggleColumn, visibility }: ColumnMenuProps) {
  const [open, setOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const visibleDataColumns = getVisibleDataColumnCount(visibility);

  useEffect(() => {
    if (!open) {
      return undefined;
    }

    function handlePointerDown(event: MouseEvent): void {
      if (!menuRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    }

    document.addEventListener("mousedown", handlePointerDown);
    return () => document.removeEventListener("mousedown", handlePointerDown);
  }, [open]);

  return (
    <div className="relative" ref={menuRef}>
      <Button aria-expanded={open} size="sm" variant="outline" onClick={() => setOpen((current) => !current)}>
        <SlidersHorizontalIcon className="size-3.5" />
        Columns
        <ChevronDownIcon className="size-3.5" />
      </Button>

      {open ? (
        <div className="absolute right-0 z-20 mt-2 w-64 rounded-2xl border border-border/70 bg-card p-2 shadow-lg">
          <div className="px-2 py-1">
            <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">Visible columns</p>
          </div>
          <div className="mt-1 space-y-1">
            {columns.map((column) => {
              const isLastVisibleDataColumn =
                column.key !== "verified" && visibility[column.key] && visibleDataColumns === 1;

              return (
                <label
                  key={column.key}
                  className={
                    isLastVisibleDataColumn
                      ? "flex cursor-not-allowed items-center justify-between rounded-xl px-3 py-2 text-sm text-muted-foreground opacity-60"
                      : "flex cursor-pointer items-center justify-between rounded-xl px-3 py-2 text-sm text-foreground hover:bg-accent/60"
                  }
                >
                  <span>{column.label}</span>
                  <input
                    checked={visibility[column.key]}
                    className="size-4 accent-[var(--primary)]"
                    disabled={isLastVisibleDataColumn}
                    type="checkbox"
                    onChange={() => onToggleColumn(column.key)}
                  />
                </label>
              );
            })}
          </div>
        </div>
      ) : null}
    </div>
  );
}
