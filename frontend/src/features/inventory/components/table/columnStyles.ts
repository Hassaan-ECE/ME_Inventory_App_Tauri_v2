import type { CSSProperties } from "react";

import type { ColumnConfig } from "@/features/inventory/types";

export function getColumnStyle(columnKey: ColumnConfig["key"]): CSSProperties {
  switch (columnKey) {
    case "verified":
      return { width: "4.75rem" };
    case "qty":
      return { width: "3.75rem" };
    case "assetNumber":
      return { width: "7rem" };
    case "projectName":
      return { width: "8.5rem" };
    default:
      return {};
  }
}
