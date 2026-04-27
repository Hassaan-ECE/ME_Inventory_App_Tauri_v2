import type {
  InventoryDeleteMutationResult,
  InventoryEntry,
  InventoryEntryInput,
  InventoryEntryMutationResult,
  InventoryQueryInput,
  InventoryQueryResult,
  InventorySharedStatus,
  UpdateState,
} from "@/types/inventory";

export interface InventorySyncResult {
  dbPath: string;
  entries: InventoryEntry[];
  entriesChanged?: boolean;
  shared: InventorySharedStatus;
}

declare global {
  interface Window {
    inventoryDesktop?: {
      isDesktop: boolean;
      loadInventory: () => Promise<InventorySyncResult>;
      queryInventory?: (input: InventoryQueryInput) => Promise<InventoryQueryResult>;
      syncInventory: () => Promise<InventorySyncResult>;
      toggleVerifiedEntry: (entryId: string, nextVerified: boolean) => Promise<InventoryEntryMutationResult>;
      createEntry: (input: InventoryEntryInput) => Promise<InventoryEntryMutationResult>;
      updateEntry: (entryId: string, input: InventoryEntryInput) => Promise<InventoryEntryMutationResult>;
      setArchivedEntry: (entryId: string, archived: boolean) => Promise<InventoryEntryMutationResult>;
      deleteEntry: (entryId: string) => Promise<InventoryDeleteMutationResult>;
      openExternal?: (url: string) => Promise<boolean>;
      openPath?: (path: string) => Promise<boolean>;
      loadPicturePreview?: (path: string) => Promise<string | null>;
      pickPicturePath?: () => Promise<string | null>;
      exportExcel?: () => Promise<{
        canceled: boolean;
        error?: string;
        outputPath?: string;
      }>;
      checkForUpdate?: () => Promise<UpdateState>;
      downloadUpdate?: () => Promise<UpdateState>;
      installUpdate?: () => Promise<UpdateState>;
      onSharedInventoryChanged?: (callback: () => void) => () => void;
      onUpdateStateChanged?: (callback: (state: UpdateState) => void) => () => void;
    };
  }
}
