import { CheckIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { toSafeExternalUrl } from "@/lib/externalUrl";
import { formatLinkLabel } from "@/lib/inventory";
import { cn } from "@/lib/utils";
import type { ColumnConfig, InventoryEntry } from "@/types/inventory";

interface InventoryTableBodyProps {
  activeEntryId: string | null;
  bottomSpacerHeight: number;
  canModifyEntries: boolean;
  colorRows: boolean;
  columns: readonly ColumnConfig[];
  onOpenContextMenu: (entryId: string, clientX: number, clientY: number) => void;
  onOpenEntry: (entryId: string) => void;
  onOpenExternalLink: (url: string) => void;
  onToggleVerified: (entryId: string) => void;
  topSpacerHeight: number;
  visibleEntries: InventoryEntry[];
}

interface InventoryTableRowProps {
  activeEntryId: string | null;
  canModifyEntries: boolean;
  colorRows: boolean;
  columns: readonly ColumnConfig[];
  entry: InventoryEntry;
  onOpenContextMenu: (entryId: string, clientX: number, clientY: number) => void;
  onOpenEntry: (entryId: string) => void;
  onOpenExternalLink: (url: string) => void;
  onToggleVerified: (entryId: string) => void;
}

export function InventoryTableBody({
  activeEntryId,
  bottomSpacerHeight,
  canModifyEntries,
  colorRows,
  columns,
  onOpenContextMenu,
  onOpenEntry,
  onOpenExternalLink,
  onToggleVerified,
  topSpacerHeight,
  visibleEntries,
}: InventoryTableBodyProps) {
  return (
    <tbody>
      {topSpacerHeight > 0 ? <SpacerRow colSpan={columns.length} height={topSpacerHeight} /> : null}
      {visibleEntries.map((entry) => (
        <InventoryTableRow
          key={entry.id}
          activeEntryId={activeEntryId}
          canModifyEntries={canModifyEntries}
          colorRows={colorRows}
          columns={columns}
          entry={entry}
          onOpenContextMenu={onOpenContextMenu}
          onOpenEntry={onOpenEntry}
          onOpenExternalLink={onOpenExternalLink}
          onToggleVerified={onToggleVerified}
        />
      ))}
      {bottomSpacerHeight > 0 ? <SpacerRow colSpan={columns.length} height={bottomSpacerHeight} /> : null}
    </tbody>
  );
}

function InventoryTableRow({
  activeEntryId,
  canModifyEntries,
  colorRows,
  columns,
  entry,
  onOpenContextMenu,
  onOpenEntry,
  onOpenExternalLink,
  onToggleVerified,
}: InventoryTableRowProps) {
  return (
    <tr
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
          {renderCell(entry, column, onToggleVerified, canModifyEntries, onOpenExternalLink)}
        </td>
      ))}
    </tr>
  );
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
  onOpenExternalLink: (url: string) => void,
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
          onClick={(event) => {
            event.preventDefault();
            onOpenExternalLink(safeUrl);
          }}
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
