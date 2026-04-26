import { Button } from "@/components/ui/button";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import type { InventoryScope } from "@/types/inventory";

interface EmptyResultsProps {
  onAddEntry: () => void;
  query: string;
  scope: InventoryScope;
}

export function EmptyResults({ onAddEntry, query, scope }: EmptyResultsProps) {
  if (scope === "archive") {
    return (
      <section className="flex h-full min-h-0 flex-1 rounded-3xl border border-border/70 bg-card/80 shadow-sm">
        <Empty>
          <EmptyHeader>
            <EmptyTitle>{query.trim() ? "No archived matches" : "No archived entries yet"}</EmptyTitle>
            <EmptyDescription>
              {query.trim()
                ? "The current archive filters did not match any archived entries."
                : "Archived entries will appear here when entries are restored or moved out of inventory."}
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      </section>
    );
  }

  return (
    <section className="flex h-full min-h-0 flex-1 rounded-3xl border border-border/70 bg-card/80 shadow-sm">
      <Empty>
        <EmptyHeader>
          <EmptyTitle>Can&apos;t find what you&apos;re looking for?</EmptyTitle>
          <EmptyDescription>
            Try a broader search, clear the column filters, or add a new entry.
          </EmptyDescription>
        </EmptyHeader>
        <Button size="sm" onClick={onAddEntry}>
          Add Entry
        </Button>
      </Empty>
    </section>
  );
}
