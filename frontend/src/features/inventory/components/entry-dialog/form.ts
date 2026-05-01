import type { Dispatch, SetStateAction } from "react";

import type {
  InventoryEntry,
  InventoryEntryInput,
  LifecycleStatus,
  WorkingStatus,
} from "@/features/inventory/types";

export interface EntryFormState {
  archived: boolean;
  assetNumber: string;
  assignedTo: string;
  condition: string;
  description: string;
  lifecycleStatus: LifecycleStatus;
  links: string;
  location: string;
  manufacturer: string;
  model: string;
  notes: string;
  picturePath: string;
  projectName: string;
  qty: string;
  serialNumber: string;
  verifiedInSurvey: boolean;
  workingStatus: WorkingStatus;
}

export function buildFormState(entry: InventoryEntry | null | undefined, defaultArchived: boolean): EntryFormState {
  return {
    archived: entry?.archived ?? defaultArchived,
    assetNumber: entry?.assetNumber ?? "",
    assignedTo: entry?.assignedTo ?? "",
    condition: entry?.condition ?? "",
    description: entry?.description ?? "",
    lifecycleStatus: entry?.lifecycleStatus ?? "active",
    links: entry?.links ?? "",
    location: entry?.location ?? "",
    manufacturer: entry?.manufacturer ?? "",
    model: entry?.model ?? "",
    notes: entry?.notes ?? "",
    picturePath: entry?.picturePath ?? "",
    projectName: entry?.projectName ?? "",
    qty: entry?.qty == null ? "" : String(entry.qty),
    serialNumber: entry?.serialNumber ?? "",
    verifiedInSurvey: entry?.verifiedInSurvey ?? false,
    workingStatus: entry?.workingStatus ?? "unknown",
  };
}

export function buildEntryInput(form: EntryFormState): { value: InventoryEntryInput } | { error: string } {
  const qtyText = form.qty.trim();
  let qty: number | null = null;

  if (qtyText) {
    qty = Number(qtyText);
    if (!Number.isFinite(qty)) {
      return { error: "Enter quantity as a number, for example 4 or 4.5." };
    }
  }

  if (!hasIdentity(form)) {
    return {
      error: "Provide at least an asset number, serial number, manufacturer, model, or description before saving.",
    };
  }

  return {
    value: {
      archived: form.archived,
      assetNumber: form.assetNumber.trim(),
      assignedTo: form.assignedTo.trim(),
      condition: form.condition.trim(),
      description: form.description.trim(),
      lifecycleStatus: form.lifecycleStatus,
      links: form.links.trim(),
      location: form.location.trim(),
      manufacturer: form.manufacturer.trim(),
      model: form.model.trim(),
      notes: form.notes.trim(),
      picturePath: form.picturePath.trim(),
      projectName: form.projectName.trim(),
      qty,
      serialNumber: form.serialNumber.trim(),
      verifiedInSurvey: form.verifiedInSurvey,
      workingStatus: form.workingStatus,
    },
  };
}

export function changedEntryInputFields(before: InventoryEntryInput, after: InventoryEntryInput): string[] {
  const fields: string[] = [];

  if (before.assetNumber !== after.assetNumber) {
    fields.push("assetNumber");
  }
  if (before.serialNumber !== after.serialNumber) {
    fields.push("serialNumber");
  }
  if (before.qty !== after.qty) {
    fields.push("qty");
  }
  if (before.manufacturer !== after.manufacturer) {
    fields.push("manufacturer");
  }
  if (before.model !== after.model) {
    fields.push("model");
  }
  if (before.description !== after.description) {
    fields.push("description");
  }
  if (before.projectName !== after.projectName) {
    fields.push("projectName");
  }
  if (before.location !== after.location) {
    fields.push("location");
  }
  if (before.assignedTo !== after.assignedTo) {
    fields.push("assignedTo");
  }
  if (before.links !== after.links) {
    fields.push("links");
  }
  if (before.notes !== after.notes) {
    fields.push("notes");
  }
  if (before.lifecycleStatus !== after.lifecycleStatus) {
    fields.push("lifecycleStatus");
  }
  if (before.workingStatus !== after.workingStatus) {
    fields.push("workingStatus");
  }
  if (before.condition !== after.condition) {
    fields.push("condition");
  }
  if (before.verifiedInSurvey !== after.verifiedInSurvey) {
    fields.push("verifiedInSurvey");
  }
  if (before.archived !== after.archived) {
    fields.push("archived");
  }
  if ((before.picturePath ?? "") !== (after.picturePath ?? "")) {
    fields.push("picturePath");
  }

  return fields;
}

function hasIdentity(form: EntryFormState): boolean {
  return Boolean(
    form.assetNumber.trim() ||
      form.serialNumber.trim() ||
      form.manufacturer.trim() ||
      form.model.trim() ||
      form.description.trim(),
  );
}

export function updateForm<Key extends keyof EntryFormState>(
  setForm: Dispatch<SetStateAction<EntryFormState>>,
  key: Key,
  value: EntryFormState[Key],
): void {
  setForm((current) => ({ ...current, [key]: value }));
}

export function formatOptionLabel(option: string): string {
  return option.replaceAll("_", " ").replace(/\b\w/g, (character) => character.toUpperCase());
}
