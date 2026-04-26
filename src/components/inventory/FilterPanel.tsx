import { Input } from "@/components/ui/input";
import type { FilterState } from "@/types/inventory";

interface FilterPanelProps {
  filters: FilterState;
  onChange: (field: keyof FilterState, value: string) => void;
  onClear: () => void;
}

export function FilterPanel({ filters, onChange, onClear }: FilterPanelProps) {
  return (
    <section className="rounded-3xl border border-border/70 bg-card/80 p-4 sm:p-5">
      <div className="flex items-center justify-between gap-3">
        <h2 className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.08em] text-foreground/50">
          <span aria-hidden className="inline-block h-px w-3 bg-border" />
          Column Filters
        </h2>
        <button className="text-xs text-muted-foreground transition-colors hover:text-foreground" type="button" onClick={onClear}>
          Clear Column Filters
        </button>
      </div>

      <div className="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-5">
        <Input
          aria-label="Filter asset number"
          placeholder="Filter asset number"
          value={filters.assetNumber}
          onChange={(event) => onChange("assetNumber", event.currentTarget.value)}
        />
        <Input
          aria-label="Filter manufacturer"
          placeholder="Filter manufacturer"
          value={filters.manufacturer}
          onChange={(event) => onChange("manufacturer", event.currentTarget.value)}
        />
        <Input
          aria-label="Filter model"
          placeholder="Filter model"
          value={filters.model}
          onChange={(event) => onChange("model", event.currentTarget.value)}
        />
        <Input
          aria-label="Filter description"
          placeholder="Filter description"
          value={filters.description}
          onChange={(event) => onChange("description", event.currentTarget.value)}
        />
        <Input
          aria-label="Filter location"
          placeholder="Filter location"
          value={filters.location}
          onChange={(event) => onChange("location", event.currentTarget.value)}
        />
      </div>
    </section>
  );
}
