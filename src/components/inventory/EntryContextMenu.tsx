import { useEffect, useRef } from "react";

import { cn } from "@/lib/utils";
import type { InventoryEntry, InventoryScope } from "@/types/inventory";

export type EntryContextAction = "open" | "open-link" | "search-online" | "archive-toggle" | "delete";

interface EntryContextMenuProps {
  canModifyEntries: boolean;
  onAction: (action: EntryContextAction) => void;
  onClose: () => void;
  position: {
    x: number;
    y: number;
  };
  entry: InventoryEntry;
  scope: InventoryScope;
}

export function EntryContextMenu({ canModifyEntries, onAction, onClose, position, entry, scope }: EntryContextMenuProps) {
  const menuRef = useRef<HTMLDivElement | null>(null);
  const archiveLabel = scope === "archive" || entry.archived ? "Restore Entry" : "Archive Entry";
  const hasSavedLink = entry.links.trim().length > 0;

  useEffect(() => {
    function handlePointerDown(event: MouseEvent): void {
      if (!menuRef.current?.contains(event.target as Node)) {
        onClose();
      }
    }

    function handleKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape") {
        onClose();
      }
    }

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-30"
      onContextMenu={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <div
        ref={menuRef}
        className="absolute w-56 rounded-2xl border border-border/70 bg-card p-2 shadow-xl"
        style={{ left: position.x, top: position.y }}
      >
        <div className="space-y-1">
          <MenuButton label="Open Full Entry" onClick={() => onAction("open")} />
          {hasSavedLink ? <MenuButton label="Open Saved Link" onClick={() => onAction("open-link")} /> : null}
          <MenuButton label="Search Online" onClick={() => onAction("search-online")} />
          <div className="my-1 h-px bg-border/70" />
          <MenuButton disabled={!canModifyEntries} label={archiveLabel} onClick={() => onAction("archive-toggle")} />
          <MenuButton destructive disabled={!canModifyEntries} label="Delete Entry" onClick={() => onAction("delete")} />
        </div>
      </div>
    </div>
  );
}

interface MenuButtonProps {
  destructive?: boolean;
  disabled?: boolean;
  label: string;
  onClick: () => void;
}

function MenuButton({ destructive = false, disabled = false, label, onClick }: MenuButtonProps) {
  return (
    <button
      className={cn(
        "flex w-full items-center rounded-xl px-3 py-2 text-left text-sm transition-colors",
        destructive ? "text-destructive-foreground hover:bg-destructive/8" : "text-foreground hover:bg-accent/60",
        disabled ? "cursor-not-allowed opacity-50 hover:bg-transparent" : "",
      )}
      disabled={disabled}
      type="button"
      onClick={onClick}
    >
      {label}
    </button>
  );
}
