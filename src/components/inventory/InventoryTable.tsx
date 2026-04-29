import { memo, useEffect, useMemo, useRef, useState } from "react";

import { InventoryTableBody } from "@/components/inventory/table/InventoryTableBody";
import { InventoryTableColumnGroup, InventoryTableHeader } from "@/components/inventory/table/InventoryTableHeader";
import { ROW_HEIGHT, clampScrollTop, getVisibleRange } from "@/components/inventory/table/virtualization";
import type { ColumnConfig, InventoryEntry, SortState } from "@/types/inventory";

interface InventoryTableProps {
  activeEntryId?: string | null;
  canModifyEntries: boolean;
  colorRows: boolean;
  columns: readonly ColumnConfig[];
  onOpenContextMenu: (entryId: string, clientX: number, clientY: number) => void;
  onOpenEntry: (entryId: string) => void;
  onOpenExternalLink: (url: string) => void;
  onSortChange: (columnKey: ColumnConfig["key"]) => void;
  onToggleVerified: (entryId: string) => void;
  entries: InventoryEntry[];
  sortState: SortState;
}

export const InventoryTable = memo(function InventoryTable({
  activeEntryId = null,
  canModifyEntries,
  colorRows,
  columns,
  onOpenContextMenu,
  onOpenEntry,
  onOpenExternalLink,
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

  useEffect(() => {
    const node = scrollRef.current;
    setScrollTop((currentScrollTop) => {
      const nextScrollTop = clampScrollTop(currentScrollTop, entries.length, viewportHeight);
      if (node && node.scrollTop !== nextScrollTop) {
        node.scrollTop = nextScrollTop;
      }

      return currentScrollTop === nextScrollTop ? currentScrollTop : nextScrollTop;
    });
  }, [entries.length, viewportHeight]);

  return (
    <section className="flex h-full min-h-0 flex-1 overflow-hidden rounded-3xl border border-border/70 bg-card/80 shadow-sm">
      <div
        ref={scrollRef}
        className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden"
        onScroll={(event) => {
          const nextViewportHeight = event.currentTarget.clientHeight || viewportHeight;
          setScrollTop(clampScrollTop(event.currentTarget.scrollTop, entries.length, nextViewportHeight));
        }}
      >
        <table className="w-full table-fixed border-separate border-spacing-0">
          <InventoryTableColumnGroup columns={columns} />
          <InventoryTableHeader columns={columns} sortState={sortState} onSortChange={onSortChange} />
          <InventoryTableBody
            activeEntryId={activeEntryId}
            bottomSpacerHeight={bottomSpacerHeight}
            canModifyEntries={canModifyEntries}
            colorRows={colorRows}
            columns={columns}
            topSpacerHeight={topSpacerHeight}
            visibleEntries={visibleEntries}
            onOpenContextMenu={onOpenContextMenu}
            onOpenEntry={onOpenEntry}
            onOpenExternalLink={onOpenExternalLink}
            onToggleVerified={onToggleVerified}
          />
        </table>
      </div>
    </section>
  );
});
