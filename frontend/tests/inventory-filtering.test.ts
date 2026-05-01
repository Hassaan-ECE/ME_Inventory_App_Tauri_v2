import { describe, expect, it } from "vitest";

import { MOCK_INVENTORY } from "@/features/inventory/data/mockInventory";
import {
  DEFAULT_FILTERS,
  buildResultsLabel,
  filterEntries,
  formatLinkLabel,
  getInventoryCounts,
  sortEntries,
} from "@/features/inventory/lib";

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

  it("searches assigned user and condition fields", () => {
    const entries = MOCK_INVENTORY.map((entry) => {
      if (entry.id === "me-1002") {
        return { ...entry, assignedTo: "Avery Morgan" };
      }
      if (entry.id === "me-1004") {
        return { ...entry, condition: "Calibration due" };
      }
      return entry;
    });

    expect(filterEntries(entries, "inventory", "avery", DEFAULT_FILTERS).map((entry) => entry.id)).toEqual([
      "me-1002",
    ]);
    expect(filterEntries(entries, "inventory", "calibration due", DEFAULT_FILTERS).map((entry) => entry.id)).toEqual([
      "me-1004",
    ]);
  });

  it("builds result labels that match the source behavior", () => {
    const activeLocationFilter = { ...DEFAULT_FILTERS, location: "archive" };

    expect(buildResultsLabel(10, "inventory", "", DEFAULT_FILTERS)).toBe("Showing all 10 entries");
    expect(buildResultsLabel(0, "archive", "", DEFAULT_FILTERS)).toBe("No archived entries yet");
    expect(buildResultsLabel(3, "inventory", "", activeLocationFilter)).toBe("Showing 3 filtered entries");
    expect(buildResultsLabel(2, "archive", "", activeLocationFilter)).toBe("Showing 2 filtered archived entries");
    expect(buildResultsLabel(1, "inventory", "fluke", DEFAULT_FILTERS)).toBe('1 result for "fluke"');
    expect(buildResultsLabel(0, "archive", "bridgeport", DEFAULT_FILTERS)).toBe('No archived results for "bridgeport"');
    expect(buildResultsLabel(2, "archive", "active", activeLocationFilter)).toBe(
      '2 archived results for "active" after column filters',
    );
  });

  it("keeps blank values at the bottom when sorting", () => {
    const results = filterEntries(MOCK_INVENTORY, "inventory", "", DEFAULT_FILTERS);
    const ascendingQuantity = sortEntries(results, { column: "qty", direction: "asc" });
    const descendingQuantity = sortEntries(results, { column: "qty", direction: "desc" });
    const ascendingAssetNumber = sortEntries(results, { column: "assetNumber", direction: "asc" });
    const descendingAssetNumber = sortEntries(results, { column: "assetNumber", direction: "desc" });

    expect(ascendingQuantity.at(-1)?.id).toBe("me-1007");
    expect(descendingQuantity.at(-1)?.id).toBe("me-1007");
    expect(ascendingAssetNumber.at(-1)?.id).toBe("me-1004");
    expect(descendingAssetNumber.at(-1)?.id).toBe("me-1004");
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
