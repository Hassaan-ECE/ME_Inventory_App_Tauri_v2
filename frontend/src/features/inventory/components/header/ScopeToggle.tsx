import { cn } from "@/shared/lib/utils";
import type { InventoryScope } from "@/features/inventory/types";

interface ScopeToggleProps {
  archiveCount: number;
  inventoryCount: number;
  onScopeChange: (scope: InventoryScope) => void;
  scope: InventoryScope;
}

export function ScopeToggle({ archiveCount, inventoryCount, onScopeChange, scope }: ScopeToggleProps) {
  return (
    <div className="inline-flex rounded-2xl border border-border/70 bg-card/80 p-1">
      <button
        className={cn(
          "rounded-xl px-3 py-1.5 text-sm font-medium transition-colors",
          scope === "inventory"
            ? "bg-success/15 text-success-foreground"
            : "text-success-foreground/80 hover:bg-success/10 hover:text-success-foreground",
        )}
        type="button"
        onClick={() => onScopeChange("inventory")}
      >
        Inventory ({inventoryCount})
      </button>
      <button
        className={cn(
          "rounded-xl px-3 py-1.5 text-sm font-medium transition-colors",
          scope === "archive"
            ? "bg-warning/15 text-warning-foreground"
            : "text-warning-foreground/80 hover:bg-warning/10 hover:text-warning-foreground",
        )}
        type="button"
        onClick={() => onScopeChange("archive")}
      >
        Archive ({archiveCount})
      </button>
    </div>
  );
}
