import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";

import { APP_VERSION } from "@/branding";
import type { InventorySyncResult } from "@/types/desktop-bridge";
import type {
  InventoryDeleteMutationResult,
  InventoryEntryInput,
  InventoryEntryMutationResult,
  InventoryQueryInput,
  InventoryQueryResult,
  UpdateState,
} from "@/types/inventory";

const updateStateListeners = new Set<(state: UpdateState) => void>();
let pendingUpdate: Update | null = null;
let pendingUpdateState: UpdateState | null = null;

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
    checkForUpdate,
    downloadUpdate,
    installUpdate,
    onSharedInventoryChanged: listenToSharedInventoryChanged,
    onUpdateStateChanged: listenToUpdateStateChanged,
  };
}

async function checkForUpdate(): Promise<UpdateState> {
  publishUpdateState({
    available: false,
    currentVersion: APP_VERSION,
    status: "checking",
  });

  try {
    const update = await check();
    pendingUpdate?.close().catch(() => undefined);
    pendingUpdate = update;

    if (!update) {
      return publishUpdateState({
        available: false,
        currentVersion: APP_VERSION,
        notes: "ME Inventory is up to date.",
        status: "not-available",
      });
    }

    pendingUpdateState = updateStateFromUpdate(update, "available");
    return publishUpdateState(pendingUpdateState);
  } catch (error) {
    pendingUpdate = null;
    pendingUpdateState = null;
    return publishUpdateState(errorUpdateState(error));
  }
}

async function downloadUpdate(): Promise<UpdateState> {
  let update = pendingUpdate;
  if (!update) {
    const state = await checkForUpdate();
    if (!pendingUpdate || !state.available) {
      return state;
    }
    update = pendingUpdate;
  }

  let totalBytes: number | undefined;
  let downloadedBytes = 0;
  const downloadingState = updateStateFromUpdate(update, "downloading", {
    downloadPhase: "copying",
    downloadProgress: 0,
  });
  pendingUpdateState = downloadingState;
  publishUpdateState(downloadingState);

  try {
    await update.download((event) => {
      const nextState = updateDownloadState(update, event, totalBytes, downloadedBytes);
      if (event.event === "Started") {
        totalBytes = event.data.contentLength;
        downloadedBytes = 0;
      } else if (event.event === "Progress") {
        downloadedBytes += event.data.chunkLength;
      }
      pendingUpdateState = nextState;
      publishUpdateState(nextState);
    });

    pendingUpdateState = updateStateFromUpdate(update, "ready", {
      downloadPhase: "ready",
      downloadProgress: 100,
    });
    return publishUpdateState(pendingUpdateState);
  } catch (error) {
    pendingUpdate = null;
    pendingUpdateState = null;
    return publishUpdateState(errorUpdateState(error));
  }
}

async function installUpdate(): Promise<UpdateState> {
  const update = pendingUpdate;
  if (!update) {
    return publishUpdateState({
      available: false,
      currentVersion: APP_VERSION,
      error: "Download the update before installing it.",
      status: "error",
    });
  }

  const installingState = updateStateFromUpdate(update, "installing");
  publishUpdateState(installingState);

  try {
    await update.install();
    pendingUpdate = null;
    pendingUpdateState = installingState;
    return publishUpdateState(installingState);
  } catch (error) {
    return publishUpdateState(errorUpdateState(error, update));
  }
}

function listenToUpdateStateChanged(callback: (state: UpdateState) => void): () => void {
  updateStateListeners.add(callback);
  if (pendingUpdateState) {
    callback(pendingUpdateState);
  }

  return () => {
    updateStateListeners.delete(callback);
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

function publishUpdateState(state: UpdateState): UpdateState {
  pendingUpdateState = state;
  for (const listener of updateStateListeners) {
    listener(state);
  }
  return state;
}

function updateStateFromUpdate(
  update: Update,
  status: UpdateState["status"],
  overrides: Partial<UpdateState> = {},
): UpdateState {
  return {
    available: true,
    currentVersion: update.currentVersion || APP_VERSION,
    latestVersion: update.version,
    notes: update.body,
    publishedAt: update.date,
    status,
    ...overrides,
  };
}

function updateDownloadState(
  update: Update,
  event: DownloadEvent,
  previousTotalBytes: number | undefined,
  previousDownloadedBytes: number,
): UpdateState {
  if (event.event === "Started") {
    return updateStateFromUpdate(update, "downloading", {
      downloadPhase: "copying",
      downloadProgress: event.data.contentLength ? 0 : undefined,
    });
  }

  if (event.event === "Finished") {
    return updateStateFromUpdate(update, "downloading", {
      downloadPhase: "verifying",
      downloadProgress: 100,
    });
  }

  const totalBytes = previousTotalBytes;
  const nextDownloadedBytes = previousDownloadedBytes + event.data.chunkLength;
  const downloadProgress =
    totalBytes && totalBytes > 0 ? Math.min(99, Math.round((nextDownloadedBytes / totalBytes) * 100)) : undefined;

  return updateStateFromUpdate(update, "downloading", {
    downloadPhase: "copying",
    downloadProgress,
  });
}

function errorUpdateState(error: unknown, update?: Update): UpdateState {
  const message = error instanceof Error ? error.message : "Update failed.";
  if (update) {
    return updateStateFromUpdate(update, "error", { error: message });
  }

  return {
    available: false,
    currentVersion: APP_VERSION,
    error: message,
    status: "error",
  };
}
