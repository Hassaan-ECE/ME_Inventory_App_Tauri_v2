import { useEffect } from "react";

import { Button } from "@/shared/components/ui/button";
import type { InventoryEntry } from "@/features/inventory/types";

interface DeleteConfirmationDialogProps {
  entry: InventoryEntry;
  onCancel: () => void;
  onConfirm: () => void;
}

export function DeleteConfirmationDialog({ entry, onCancel, onConfirm }: DeleteConfirmationDialogProps) {
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape") {
        onCancel();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onCancel]);

  const title = entry.description || entry.manufacturer || entry.model || `ID ${entry.id}`;

  return (
    <div
      aria-modal="true"
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/45 p-4"
      role="dialog"
      onClick={(event) => {
        if (event.target === event.currentTarget) {
          onCancel();
        }
      }}
    >
      <section className="w-full max-w-md rounded-2xl border border-border/70 bg-card p-5 text-card-foreground shadow-2xl">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-destructive-foreground">Delete Entry</p>
          <h2 className="mt-1 text-xl font-semibold tracking-tight text-foreground">Delete this entry?</h2>
          <p className="mt-3 text-sm text-muted-foreground">
            This removes the entry from the inventory database.
          </p>
        </div>

        <div className="mt-4 rounded-xl border border-border/70 bg-background/70 px-4 py-3">
          <p className="text-sm font-medium text-foreground">{title}</p>
          {entry.assetNumber || entry.location ? (
            <p className="mt-1 text-xs text-muted-foreground">
              {[entry.assetNumber, entry.location].filter(Boolean).join(" | ")}
            </p>
          ) : null}
        </div>

        <div className="mt-5 flex justify-end gap-2">
          <Button variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
          <Button className="border-destructive bg-destructive text-white hover:bg-destructive/90" onClick={onConfirm}>
            Delete Entry
          </Button>
        </div>
      </section>
    </div>
  );
}
