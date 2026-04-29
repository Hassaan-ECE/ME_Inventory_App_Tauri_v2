import type { InventoryEntry } from "@/types/inventory";

export function getInventoryCounts(entries: InventoryEntry[]): {
  inventory: number;
  archive: number;
  total: number;
  verified: number;
} {
  let archive = 0;
  let verified = 0;

  for (const entry of entries) {
    if (entry.archived) {
      archive += 1;
    }
    if (entry.verifiedInSurvey) {
      verified += 1;
    }
  }

  return {
    inventory: entries.length - archive,
    archive,
    total: entries.length,
    verified,
  };
}
