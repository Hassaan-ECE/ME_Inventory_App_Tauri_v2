import { describe, expect, it } from "vitest";

import { MOCK_INVENTORY } from "@/data/mockInventory";
import {
  DEFAULT_FILTERS,
  buildResultsLabel,
  filterEntries,
  formatLinkLabel,
  getInventoryCounts,
  sortEntries,
} from "@/lib/inventory";

describe("inventory helpers", () => {
  it("filters inventory entries by query and scope", () => {
    const results = filterEntries(MOCK_INVENTORY, "inventory", "reorder threshold", DEFAULT_FILTERS);

    expect(results).toHaveLength(1);
    expect(results[0]?.id).toBe("me-1006");
  });

  it("combines field filters with global search", () => {
    const results = filterEntries(MOCK_INVENTORY, "inventory", "fixture", {
      ...DEFAULT_FILTERS,
      location: "tool crib",
    });

    expect(results).toHaveLength(1);
    expect(results[0]?.id).toBe("me-1003");
  });

  it("builds archive empty-state labels that match the source behavior", () => {
    expect(buildResultsLabel(0, "archive", "", DEFAULT_FILTERS)).toBe("No archived entries yet");
    expect(buildResultsLabel(0, "archive", "bridgeport", DEFAULT_FILTERS)).toBe('No archived results for "bridgeport"');
  });

  it("keeps blank numeric values at the bottom when sorting", () => {
    const results = filterEntries(MOCK_INVENTORY, "inventory", "", DEFAULT_FILTERS);
    const sorted = sortEntries(results, { column: "qty", direction: "asc" });

    expect(sorted.at(-1)?.id).toBe("me-1007");
  });

  it("formats long links into compact table labels", () => {
    expect(
      formatLinkLabel(
        "https://www.cejn.com/en-us/products/thermal-control/?filters=null%3D1191&mtm_campaign=Semicon-Campaign",
      ),
    ).toBe("www.cejn.com/en-us/products/thermal-control");
  });

  it("counts verified and archived entries from the seeded dataset", () => {
    expect(getInventoryCounts(MOCK_INVENTORY)).toEqual({
      archive: 4,
      inventory: 10,
      total: 14,
      verified: 8,
    });
  });
});
