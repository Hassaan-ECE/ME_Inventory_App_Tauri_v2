import type {
  InventoryEntry,
  InventoryEntryEditContext,
  InventoryEntryInput,
} from "@/features/inventory/types";

import { buildEntryInput, changedEntryInputFields, type EntryFormState } from "./form";

export function buildEditContext(
  mode: "add" | "edit",
  entry: InventoryEntry | null | undefined,
  initialForm: EntryFormState | null,
  input: InventoryEntryInput,
): InventoryEntryEditContext | undefined {
  if (mode !== "edit" || !entry || !initialForm) {
    return undefined;
  }

  const initialInput = buildEntryInput(initialForm);
  return {
    baseVersion: entry.updatedAt,
    changedFields: "value" in initialInput ? changedEntryInputFields(initialInput.value, input) : [],
  };
}
