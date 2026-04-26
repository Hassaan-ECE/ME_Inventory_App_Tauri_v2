import type { ReactNode } from "react";

import { Input } from "@/components/ui/input";
import { Toggle } from "@/components/ui/toggle";
import type { InventoryScope } from "@/types/inventory";

interface SearchCardProps {
  colorRows: boolean;
  columnMenu: ReactNode;
  filtersOpen: boolean;
  onColorRowsChange: (nextValue: boolean) => void;
  onFiltersToggle: () => void;
  onQueryChange: (value: string) => void;
  query: string;
  resultsLabel: string;
  scope: InventoryScope;
}

export function SearchCard({
  colorRows,
  columnMenu,
  filtersOpen,
  onColorRowsChange,
  onFiltersToggle,
  onQueryChange,
  query,
  resultsLabel,
  scope,
}: SearchCardProps) {
  return (
    <section className="rounded-3xl border border-border/70 bg-card/80 p-4 shadow-sm sm:p-5">
      <Input
        aria-label="Inventory search"
        inputClassName="h-12 px-4 text-base leading-12"
        placeholder={
          scope === "archive"
            ? "Search archived entries by asset, serial, maker, model, description, location, or notes"
            : "Search entries by asset, serial, maker, model, description, location, status, or notes"
        }
        value={query}
        onChange={(event) => onQueryChange(event.currentTarget.value)}
      />

      <div className="mt-3 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-sm text-muted-foreground">{resultsLabel}</p>
        <div className="flex flex-wrap items-center gap-2">
          <div className="flex items-center gap-2 rounded-lg border border-border/70 bg-background/70 px-2 py-1 text-xs text-muted-foreground">
            <Toggle aria-label="Color rows" pressed={colorRows} onPressedChange={onColorRowsChange} />
            <span>Color Rows</span>
          </div>
          <button
            className="inline-flex h-8 items-center justify-center rounded-lg border border-input bg-background px-3 text-xs font-medium text-foreground transition-colors hover:bg-accent/50"
            type="button"
            onClick={onFiltersToggle}
          >
            {filtersOpen ? "Hide Filters" : "Filters"}
          </button>
          {columnMenu}
        </div>
      </div>
    </section>
  );
}
