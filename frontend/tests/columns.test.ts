import { beforeEach, describe, expect, it } from "vitest";

import { getVisibleDataColumnCount } from "@/features/inventory/lib";
import { INVENTORY_COLUMNS } from "@/features/inventory/types";
import {
  COLUMN_VISIBILITY_STORAGE_KEY,
  readColumnVisibility,
} from "@/features/inventory/components/shell/helpers";

describe("column visibility persistence", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("repairs stored visibility when every data column is hidden", () => {
    const allHidden = Object.fromEntries(
      INVENTORY_COLUMNS.map((column) => [column.key, false]),
    );
    localStorage.setItem(COLUMN_VISIBILITY_STORAGE_KEY, JSON.stringify(allHidden));

    const visibility = readColumnVisibility();

    expect(getVisibleDataColumnCount(visibility)).toBe(1);
    expect(visibility.qty).toBe(true);
  });
});
