import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { InventorySyncResult } from "@/types/desktop-bridge";
import type {
  InventoryDeleteMutationResult,
  InventoryEntryInput,
  InventoryEntryMutationResult,
  InventoryQueryInput,
  InventoryQueryResult,
  UpdateState,
} from "@/types/inventory";

if (typeof window !== "undefined" && isTauri()) {
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
    loadPicturePreview: async (path: string) => {
      const previewPath = await invoke<string | null>("load_picture_preview", { path });
      return previewPath ? convertFileSrc(previewPath) : null;
    },
    pickPicturePath: () => invoke<string | null>("pick_picture_path"),
    exportExcel: () =>
      invoke<{
        canceled: boolean;
        error?: string;
        outputPath?: string;
      }>("export_excel"),
    checkForUpdate: () => invoke<UpdateState>("check_for_update"),
    downloadUpdate: () => invoke<UpdateState>("download_update"),
    installUpdate: () => invoke<UpdateState>("install_update"),
    onSharedInventoryChanged: listenToSharedInventoryChanged,
    onUpdateStateChanged: () => () => undefined,
  };
}

function listenToSharedInventoryChanged(callback: () => void): () => void {
  let disposed = false;
  let unlisten: UnlistenFn | null = null;

  void listen("inventory:shared-changed", () => {
    callback();
  })
    .then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
        return;
      }

      unlisten = nextUnlisten;
    })
    .catch(() => undefined);

  return () => {
    disposed = true;
    unlisten?.();
    unlisten = null;
  };
}
