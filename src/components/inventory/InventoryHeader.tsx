import { MoonIcon, PlusIcon, SunIcon } from "lucide-react";

import { ExportMenu } from "@/components/inventory/header/ExportMenu";
import { ScopeToggle } from "@/components/inventory/header/ScopeToggle";
import { UpdateActionButton } from "@/components/inventory/header/UpdateActionButton";
import { Button } from "@/components/ui/button";
import { APP_VERSION } from "@/branding";
import type { InventoryScope, ThemeMode, UpdateState } from "@/types/inventory";

interface InventoryHeaderProps {
  archiveCount: number;
  canModifyEntries: boolean;
  inventoryCount: number;
  onAddEntry: () => void;
  onExportExcel: () => void;
  onExportHtml: () => void;
  onScopeChange: (scope: InventoryScope) => void;
  onThemeToggle: () => void;
  onUpdateAction: () => void;
  scope: InventoryScope;
  theme: ThemeMode;
  updateState: UpdateState;
}

export function InventoryHeader({
  archiveCount,
  canModifyEntries,
  inventoryCount,
  onAddEntry,
  onExportExcel,
  onExportHtml,
  onScopeChange,
  onThemeToggle,
  onUpdateAction,
  scope,
  theme,
  updateState,
}: InventoryHeaderProps) {
  return (
    <header className="shrink-0 border-b border-border px-3 py-3 sm:px-5">
      <div className="flex flex-wrap items-center gap-3">
        <div className="flex min-w-0 items-center gap-3">
          <div className="min-w-0">
            <div className="flex min-w-0 items-baseline gap-2">
              <h1 className="min-w-0 text-2xl font-semibold tracking-tight text-foreground">ME Inventory</h1>
              <span className="text-xs font-semibold text-muted-foreground">v{APP_VERSION}</span>
            </div>
          </div>
          <UpdateActionButton state={updateState} onClick={onUpdateAction} />
        </div>

        <div className="ml-auto flex flex-wrap items-center justify-end gap-2">
          <ScopeToggle
            archiveCount={archiveCount}
            inventoryCount={inventoryCount}
            scope={scope}
            onScopeChange={onScopeChange}
          />

          <Button size="sm" variant="outline" onClick={onThemeToggle}>
            {theme === "light" ? <MoonIcon className="size-3.5" /> : <SunIcon className="size-3.5" />}
            {theme === "light" ? "Dark Theme" : "Light Theme"}
          </Button>
          <ExportMenu onExportExcel={onExportExcel} onExportHtml={onExportHtml} />
          <Button disabled={!canModifyEntries} size="sm" onClick={onAddEntry}>
            <PlusIcon className="size-3.5" />
            Add Entry
          </Button>
        </div>
      </div>
    </header>
  );
}
