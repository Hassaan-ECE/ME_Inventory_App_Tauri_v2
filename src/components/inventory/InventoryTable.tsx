import { ArrowUpDownIcon, CheckIcon } from "lucide-react";
import { memo, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties } from "react";

import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { toSafeExternalUrl } from "@/lib/externalUrl";
import { formatLinkLabel } from "@/lib/inventory";
import type { ColumnConfig, InventoryEntry, SortState } from "@/types/inventory";

interface InventoryTableProps {
  activeEntryId?: string | null;
  canModifyEntries: boolean;
  colorRows: boolean;
  columns: readonly ColumnConfig[];
  onOpenContextMenu: (entryId: string, clientX: number, clientY: number) => void;
  onOpenEntry: (entryId: string) => void;
  onSortChange: (columnKey: ColumnConfig["key"]) => void;
  onToggleVerified: (entryId: string) => void;
  entries: InventoryEntry[];
  sortState: SortState;
}

const ROW_HEIGHT = 45;
const OVERSCAN_ROWS = 10;

export const InventoryTable = memo(function InventoryTable({
  activeEntryId = null,
  canModifyEntries,
  colorRows,
  columns,
  onOpenContextMenu,
  onOpenEntry,
  onSortChange,
  onToggleVerified,
  entries,
  sortState,
}: InventoryTableProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(640);
  const visibleRange = useMemo(
    () => getVisibleRange(entries.length, scrollTop, viewportHeight),
    [entries.length, scrollTop, viewportHeight],
  );
  const visibleEntries = entries.slice(visibleRange.start, visibleRange.end);
  const topSpacerHeight = visibleRange.start * ROW_HEIGHT;
  const bottomSpacerHeight = Math.max(0, (entries.length - visibleRange.end) * ROW_HEIGHT);

  useEffect(() => {
    const node = scrollRef.current;
    if (!node) {
      return undefined;
    }

    setViewportHeight(node.clientHeight || 640);
    if (typeof ResizeObserver === "undefined") {
      return undefined;
    }

    const observer = new ResizeObserver(() => {
      setViewportHeight(node.clientHeight || 640);
    });
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  return (
    <section className="flex h-full min-h-0 flex-1 overflow-hidden rounded-3xl border border-border/70 bg-card/80 shadow-sm">
      <div
        ref={scrollRef}
        className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden"
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
      >
        <table className="w-full table-fixed border-separate border-spacing-0">
          <colgroup>
            {columns.map((column) => (
              <col key={column.key} style={getColumnStyle(column.key)} />
            ))}
          </colgroup>
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
          <tbody>
            {topSpacerHeight > 0 ? <SpacerRow colSpan={columns.length} height={topSpacerHeight} /> : null}
            {visibleEntries.map((entry) => (
              <tr
                key={entry.id}
                className={cn(
                  rowToneClass(entry, colorRows),
                  activeEntryId === entry.id ? "ring-1 ring-inset ring-primary/25" : "",
                  "cursor-default transition-colors hover:bg-accent/35",
                )}
                onContextMenu={(event) => {
                  event.preventDefault();
                  onOpenContextMenu(entry.id, event.clientX, event.clientY);
                }}
                onDoubleClick={(event) => {
                  if (event.target instanceof Element && event.target.closest("button,a,input")) {
                    return;
                  }
                  onOpenEntry(entry.id);
                }}
              >
                {columns.map((column) => (
                  <td
                    key={`${entry.id}-${column.key}`}
                    className={cn(
                      "border-b border-border/60 px-2.5 py-2.5 text-sm text-foreground/92 sm:px-4 sm:py-3",
                      column.align === "center" ? "text-center" : "text-left",
                    )}
                  >
                    {renderCell(entry, column, onToggleVerified, canModifyEntries)}
                  </td>
                ))}
              </tr>
            ))}
            {bottomSpacerHeight > 0 ? <SpacerRow colSpan={columns.length} height={bottomSpacerHeight} /> : null}
          </tbody>
        </table>
      </div>
    </section>
  );
});

function getVisibleRange(entryCount: number, scrollTop: number, viewportHeight: number): { end: number; start: number } {
  const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN_ROWS);
  const visibleCount = Math.ceil(viewportHeight / ROW_HEIGHT) + OVERSCAN_ROWS * 2;
  return {
    end: Math.min(entryCount, start + visibleCount),
    start,
  };
}

function SpacerRow({ colSpan, height }: { colSpan: number; height: number }) {
  return (
    <tr aria-hidden="true">
      <td colSpan={colSpan} style={{ height, padding: 0 }} />
    </tr>
  );
}

function renderCell(
  entry: InventoryEntry,
  column: ColumnConfig,
  onToggleVerified: (entryId: string) => void,
  canModifyEntries: boolean,
) {
  switch (column.key) {
    case "verified":
      return (
        <button
          aria-label={`Toggle verified for ${entry.description}`}
          className="inline-flex items-center justify-center"
          disabled={!canModifyEntries}
          type="button"
          onClick={() => onToggleVerified(entry.id)}
        >
          <Badge size="sm" variant={entry.verifiedInSurvey ? "success" : "outline"}>
            {entry.verifiedInSurvey ? <CheckIcon className="size-3" /> : null}
            {entry.verifiedInSurvey ? "Verified" : "Pending"}
          </Badge>
        </button>
      );
    case "assetNumber":
      return renderText(entry.assetNumber);
    case "qty":
      return renderText(entry.qty == null ? "" : String(entry.qty));
    case "manufacturer":
      return renderText(entry.manufacturer);
    case "model":
      return renderText(entry.model);
    case "description":
      return renderText(entry.description);
    case "projectName":
      return renderText(entry.projectName);
    case "location":
      return renderText(entry.location);
    case "links": {
      const label = formatLinkLabel(entry.links);
      if (!label) {
        return renderText("");
      }
      const safeUrl = toSafeExternalUrl(entry.links);
      if (!safeUrl) {
        return renderText(entry.links.trim());
      }
      return (
        <a
          className="inline-block max-w-full truncate font-mono text-xs text-foreground underline decoration-border underline-offset-4 transition-colors hover:text-primary"
          href={safeUrl}
          rel="noreferrer"
          title={safeUrl}
          target="_blank"
        >
          {label}
        </a>
      );
    }
  }
}

function renderText(value: string) {
  if (!value.trim()) {
    return <span className="text-muted-foreground">-</span>;
  }
  return (
    <span className="block min-w-0 truncate" title={value}>
      {value}
    </span>
  );
}

function rowToneClass(entry: InventoryEntry, colorRows: boolean): string {
  if (!colorRows) {
    return "bg-transparent";
  }

  switch (entry.lifecycleStatus) {
    case "active":
      return "bg-success/10";
    case "repair":
      return "bg-warning/10";
    case "scrapped":
    case "missing":
      return "bg-destructive/10";
    case "rental":
      return "bg-accent/60";
  }
}

function getColumnStyle(columnKey: ColumnConfig["key"]): CSSProperties {
  switch (columnKey) {
    case "verified":
      return { width: "4.75rem" };
    case "qty":
      return { width: "3.75rem" };
    case "assetNumber":
      return { width: "7rem" };
    case "projectName":
      return { width: "8.5rem" };
    default:
      return {};
  }
}
