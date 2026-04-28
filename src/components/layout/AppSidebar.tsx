import { ArchiveIcon, CommandIcon, PackageIcon, UploadIcon } from "lucide-react";

import { APP_BASE_NAME, APP_VERSION } from "@/branding";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { InventoryScope } from "@/types/inventory";

interface AppSidebarProps {
  archiveCount: number;
  inventoryCount: number;
  onMockAction: (label: string) => void;
  onScopeChange: (scope: InventoryScope) => void;
  scope: InventoryScope;
}

export function AppSidebar({
  archiveCount,
  inventoryCount,
  onMockAction,
  onScopeChange,
  scope,
}: AppSidebarProps) {
  return (
    <aside className="hidden h-full w-[20rem] shrink-0 border-r border-border bg-card text-foreground md:flex md:flex-col">
      <div className="border-b border-border px-3 py-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">{APP_BASE_NAME}</p>
            <p className="mt-1 text-sm font-medium text-foreground">Shared entry workspace</p>
          </div>
          <Badge size="sm" variant="secondary">
            v{APP_VERSION}
          </Badge>
        </div>

        <div className="mt-4">
          <Button className="w-full" size="sm" variant="outline" onClick={() => onMockAction("Export Excel")}>
            <UploadIcon className="size-3.5" />
            Export
          </Button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-2 py-3">
        <section className="mb-4">
          <div className="mb-1 flex items-center gap-2 px-2">
            <span className="inline-flex size-2 rounded-full bg-gradient-to-br from-sky-400 to-emerald-400" />
            <span className="truncate text-xs font-semibold uppercase tracking-[0.08em] text-muted-foreground/80">Views</span>
          </div>

          <div className="space-y-1">
            <button
              className={
                scope === "inventory"
                  ? "w-full rounded-2xl border border-border/70 bg-accent/60 px-2.5 py-2.5 text-left"
                  : "w-full rounded-2xl border border-transparent px-2.5 py-2.5 text-left hover:border-border/70 hover:bg-accent/40"
              }
              type="button"
              onClick={() => onScopeChange("inventory")}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                  <p className="truncate text-sm font-medium text-foreground">Inventory</p>
                  <p className="mt-1 text-[11px] text-muted-foreground">Active ME entries</p>
                </div>
                <Badge size="sm" variant="success">
                  {inventoryCount}
                </Badge>
              </div>
            </button>

            <button
              className={
                scope === "archive"
                  ? "w-full rounded-2xl border border-border/70 bg-accent/60 px-2.5 py-2.5 text-left"
                  : "w-full rounded-2xl border border-transparent px-2.5 py-2.5 text-left hover:border-border/70 hover:bg-accent/40"
              }
              type="button"
              onClick={() => onScopeChange("archive")}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                  <p className="truncate text-sm font-medium text-foreground">Archive</p>
                  <p className="mt-1 text-[11px] text-muted-foreground">Retired and retained entries</p>
                </div>
                <Badge size="sm" variant="outline">
                  {archiveCount}
                </Badge>
              </div>
            </button>
          </div>
        </section>
      </div>

      <div className="border-t border-border p-2">
        <div className="mb-2 rounded-2xl border border-border/70 bg-background/70 px-3 py-2.5">
          <div className="flex items-center gap-2 text-xs font-medium text-foreground">
            <PackageIcon className="size-3.5 text-muted-foreground" />
            Local Runtime
          </div>
          <div className="mt-2 flex items-center gap-2">
            <Badge size="sm" variant="outline">
              <CommandIcon className="size-3" />
              v{APP_VERSION}
            </Badge>
            <span className="text-[11px] text-muted-foreground">Seeded inventory entries only</span>
          </div>
        </div>

        <Button className="w-full justify-start gap-2" size="sm" variant="ghost" onClick={() => onMockAction("Export HTML")}>
          <ArchiveIcon className="size-4" />
          <span>Export HTML</span>
        </Button>
      </div>
    </aside>
  );
}
