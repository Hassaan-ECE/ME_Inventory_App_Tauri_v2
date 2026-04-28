import { MoonIcon, SunIcon, UploadIcon } from "lucide-react";

import { APP_BASE_NAME, APP_VERSION } from "@/branding";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { InventoryScope, ThemeMode } from "@/types/inventory";

interface MobileTopBarProps {
  archiveCount: number;
  inventoryCount: number;
  onMockAction: (label: string) => void;
  onScopeChange: (scope: InventoryScope) => void;
  onThemeToggle: () => void;
  scope: InventoryScope;
  theme: ThemeMode;
}

export function MobileTopBar({
  archiveCount,
  inventoryCount,
  onMockAction,
  onScopeChange,
  onThemeToggle,
  scope,
  theme,
}: MobileTopBarProps) {
  return (
    <div className="border-b border-border bg-card px-3 py-3 md:hidden">
      <div className="flex items-center justify-between gap-3">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">{APP_BASE_NAME}</p>
          <p className="mt-1 text-sm font-medium text-foreground">v{APP_VERSION}</p>
        </div>
        <Button aria-label="Toggle theme" size="sm" variant="outline" onClick={onThemeToggle}>
          {theme === "light" ? <MoonIcon className="size-3.5" /> : <SunIcon className="size-3.5" />}
        </Button>
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Button size="sm" variant={scope === "inventory" ? "default" : "outline"} onClick={() => onScopeChange("inventory")}>
          Inventory
          <Badge size="sm" variant={scope === "inventory" ? "secondary" : "outline"}>
            {inventoryCount}
          </Badge>
        </Button>
        <Button size="sm" variant={scope === "archive" ? "default" : "outline"} onClick={() => onScopeChange("archive")}>
          Archive
          <Badge size="sm" variant={scope === "archive" ? "secondary" : "outline"}>
            {archiveCount}
          </Badge>
        </Button>
        <Button size="sm" variant="outline" onClick={() => onMockAction("Export Excel")}>
          <UploadIcon className="size-3.5" />
          Export
        </Button>
      </div>
    </div>
  );
}
