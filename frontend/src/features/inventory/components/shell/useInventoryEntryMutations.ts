import { useState } from "react";
import type { Dispatch, SetStateAction } from "react";

import type {
  InventoryEntry,
  InventoryEntryEditContext,
  InventoryEntryInput,
  InventorySharedStatus,
} from "@/features/inventory/types";

import {
  buildLocalCreatedEntry,
  buildLocalUpdatedEntry,
  normalizeSharedStatus,
  sharedStatusesMatch,
} from "./helpers";

export interface DialogState {
  mode: "add" | "edit";
  entryId?: string;
}

interface UseInventoryEntryMutationsOptions {
  announceStatus: (message: string) => void;
  canModifyEntries: boolean;
  dataSource: "desktop" | "mock";
  entriesById: Map<string, InventoryEntry>;
  onDialogOpen: () => void;
  scheduleDesktopSync: () => void;
  setEntries: Dispatch<SetStateAction<InventoryEntry[]>>;
  setSharedStatus: Dispatch<SetStateAction<InventorySharedStatus>>;
  sharedStatus: InventorySharedStatus;
}

export function useInventoryEntryMutations({
  announceStatus,
  canModifyEntries,
  dataSource,
  entriesById,
  onDialogOpen,
  scheduleDesktopSync,
  setEntries,
  setSharedStatus,
  sharedStatus,
}: UseInventoryEntryMutationsOptions) {
  const [dialogState, setDialogState] = useState<DialogState | null>(null);
  const [pendingDeleteEntryId, setPendingDeleteEntryId] = useState<string | null>(null);

  function applyDesktopMutationFeedback(result: { message: string; shared?: InventorySharedStatus }): void {
    if (result.shared) {
      const shared = normalizeSharedStatus(result.shared);
      setSharedStatus((current) => (sharedStatusesMatch(current, shared) ? current : shared));
    }
    announceStatus(result.message);
  }

  function handleAddEntry(): void {
    if (!canModifyEntries) {
      announceStatus(sharedStatus.message || "Shared workspace unavailable. Saving changes locally.");
      return;
    }
    onDialogOpen();
    setDialogState({ mode: "add" });
  }

  function handleOpenEntry(entryId: string): void {
    onDialogOpen();
    setDialogState({ mode: "edit", entryId });
  }

  async function handleToggleVerified(entryId: string): Promise<void> {
    if (dataSource === "desktop" && !canModifyEntries) {
      announceStatus(sharedStatus.message || "Shared workspace unavailable. Saving changes locally.");
      return;
    }

    const nextVerified = !entriesById.get(entryId)?.verifiedInSurvey;

    if (dataSource === "desktop" && window.inventoryDesktop?.toggleVerifiedEntry) {
      try {
        const result = await window.inventoryDesktop.toggleVerifiedEntry(entryId, nextVerified);
        setEntries((current) => current.map((entry) => (entry.id === entryId ? result.entry : entry)));
        applyDesktopMutationFeedback(result);
        scheduleDesktopSync();
        return;
      } catch {
        announceStatus("Could not update the ME Inventory database.");
        return;
      }
    }

    setEntries((current) =>
      current.map((entry) =>
        entry.id === entryId ? { ...entry, verifiedInSurvey: !entry.verifiedInSurvey } : entry,
      ),
    );
    announceStatus("Verified state updated locally.");
  }

  async function handleSaveEntry(input: InventoryEntryInput, editContext?: InventoryEntryEditContext): Promise<void> {
    if (dataSource === "desktop" && !canModifyEntries) {
      throw new Error(sharedStatus.message || "Shared workspace unavailable. Saving changes locally.");
    }

    if (dialogState?.mode === "edit" && dialogState.entryId) {
      const entryId = dialogState.entryId;
      const existingEntry = entriesById.get(entryId);
      if (!existingEntry) {
        throw new Error("The selected entry could not be found.");
      }

      if (dataSource === "desktop" && window.inventoryDesktop?.updateEntry) {
        const result = await window.inventoryDesktop.updateEntry(entryId, input, editContext);
        setEntries((current) =>
          current.map((entry) =>
            entry.id === entryId ||
            entry.id === result.entry.id ||
            (entry.entryUuid && result.entry.entryUuid && entry.entryUuid === result.entry.entryUuid)
              ? result.entry
              : entry,
          ),
        );
        applyDesktopMutationFeedback(result);
        scheduleDesktopSync();
      } else {
        const updatedEntry = buildLocalUpdatedEntry(existingEntry, input);
        setEntries((current) => current.map((entry) => (entry.id === updatedEntry.id ? updatedEntry : entry)));
        announceStatus("Entry updated locally.");
      }

      setDialogState(null);
      return;
    }

    if (dataSource === "desktop" && window.inventoryDesktop?.createEntry) {
      const result = await window.inventoryDesktop.createEntry(input);
      setEntries((current) => [result.entry, ...current.filter((entry) => entry.id !== result.entry.id)]);
      applyDesktopMutationFeedback(result);
      scheduleDesktopSync();
    } else {
      const createdEntry = buildLocalCreatedEntry(input);
      setEntries((current) => [createdEntry, ...current]);
      announceStatus("Entry added locally.");
    }

    setDialogState(null);
  }

  async function handleArchiveChange(entryId: string, archived: boolean): Promise<void> {
    if (dataSource === "desktop" && !canModifyEntries) {
      announceStatus(sharedStatus.message || "Shared workspace unavailable. Saving changes locally.");
      return;
    }

    const entry = entriesById.get(entryId);
    if (!entry || entry.archived === archived) {
      return;
    }

    if (dataSource === "desktop" && window.inventoryDesktop?.setArchivedEntry) {
      try {
        const result = await window.inventoryDesktop.setArchivedEntry(entryId, archived);
        setEntries((current) => current.map((entry) => (entry.id === result.entry.id ? result.entry : entry)));
        applyDesktopMutationFeedback(result);
        scheduleDesktopSync();
      } catch {
        announceStatus("Could not update the shared inventory database.");
        return;
      }
    } else {
      setEntries((current) =>
        current.map((entry) => (entry.id === entryId ? { ...entry, archived, updatedAt: new Date().toISOString() } : entry)),
      );
      announceStatus(archived ? "Entry moved to the archive." : "Entry restored to inventory.");
    }
  }

  function handleRequestDeleteEntry(entryId: string): void {
    if (dataSource === "desktop" && !canModifyEntries) {
      announceStatus(sharedStatus.message || "Shared workspace unavailable. Saving changes locally.");
      return;
    }

    const entry = entriesById.get(entryId);
    if (!entry) {
      return;
    }

    setPendingDeleteEntryId(entryId);
  }

  async function handleConfirmDeleteEntry(entryId: string): Promise<void> {
    setPendingDeleteEntryId(null);

    if (dataSource === "desktop" && window.inventoryDesktop?.deleteEntry) {
      try {
        const result = await window.inventoryDesktop.deleteEntry(entryId);
        setEntries((current) => current.filter((entry) => entry.id !== entryId));
        applyDesktopMutationFeedback(result);
        scheduleDesktopSync();
        return;
      } catch {
        announceStatus("Could not delete from the shared inventory database.");
        return;
      }
    }

    setEntries((current) => current.filter((entry) => entry.id !== entryId));
    announceStatus("Entry deleted.");
  }

  return {
    cancelDeleteEntry: () => setPendingDeleteEntryId(null),
    closeDialog: () => setDialogState(null),
    dialogState,
    handleAddEntry,
    handleArchiveChange,
    handleConfirmDeleteEntry,
    handleOpenEntry,
    handleRequestDeleteEntry,
    handleSaveEntry,
    handleToggleVerified,
    pendingDeleteEntryId,
  };
}
