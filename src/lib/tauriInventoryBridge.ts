import { invoke } from "@tauri-apps/api/core";

import type { InventorySyncResult } from "@/types/desktop-bridge";
import type {
  InventoryDeleteMutationResult,
  InventoryEntryInput,
  InventoryEntryMutationResult,
  InventoryQueryInput,
  InventoryQueryResult,
  UpdateState,
} from "@/types/inventory";

const idleUpdateState: UpdateState = {
  available: false,
  currentVersion: "0.9.7",
  status: "idle",
};

if (typeof window !== "undefined") {
  window.inventoryDesktop = {
    isDesktop: true,
    loadInventory: () => invoke<InventorySyncResult>("load_inventory"),
    queryInventory: (input: InventoryQueryInput) =>
      invoke<InventoryQueryResult>("query_inventory", { input }),
    syncInventory: () => invoke<InventorySyncResult>("sync_inventory"),
    toggleVerifiedEntry: (entryId: string, nextVerified: boolean) =>
      invoke<InventoryEntryMutationResult>("toggle_verified_entry", {
        entryId,
        nextVerified,
      }),
    createEntry: (input: InventoryEntryInput) =>
      invoke<InventoryEntryMutationResult>("create_entry", { input }),
    updateEntry: (entryId: string, input: InventoryEntryInput) =>
      invoke<InventoryEntryMutationResult>("update_entry", { entryId, input }),
    setArchivedEntry: (entryId: string, archived: boolean) =>
      invoke<InventoryEntryMutationResult>("set_archived_entry", {
        entryId,
        archived,
      }),
    deleteEntry: (entryId: string) =>
      invoke<InventoryDeleteMutationResult>("delete_entry", { entryId }),
    openExternal: async (url: string) => invoke<boolean>("open_external", { url }),
    openPath: async (path: string) => invoke<boolean>("open_path", { path }),
    pickPicturePath: () => invoke<string | null>("pick_picture_path"),
    checkForUpdate: async () => idleUpdateState,
    downloadUpdate: async () => idleUpdateState,
    installUpdate: async () => idleUpdateState,
    onSharedInventoryChanged: () => () => undefined,
    onUpdateStateChanged: () => () => undefined,
  };
}
