import { APP_VERSION } from "@/branding";
import { buildDefaultColumnVisibility, mergeColumnVisibility } from "@/lib/inventory";
import type {
  ColumnKey,
  InventoryEntry,
  InventoryEntryInput,
  InventorySharedStatus,
  ThemeMode,
  UpdateState,
} from "@/types/inventory";

export const THEME_STORAGE_KEY = "meInventory.theme";
export const COLOR_ROWS_STORAGE_KEY = "meInventory.colorRows";
export const COLUMN_VISIBILITY_STORAGE_KEY = "meInventory.columnVisibility";
export const DEFAULT_SHARED_SYNC_INTERVAL_MS = 60_000;

const MIN_SHARED_SYNC_INTERVAL_MS = 30_000;
const MAX_SHARED_SYNC_INTERVAL_MS = 5 * 60_000;

export const MOCK_SHARED_STATUS: InventorySharedStatus = {
  available: true,
  canModify: true,
  enabled: false,
  message: "",
  mutationMode: "shared",
};

export const DESKTOP_SHARED_PENDING_STATUS: InventorySharedStatus = {
  available: false,
  canModify: false,
  enabled: false,
  message: "Checking shared workspace...",
  mutationMode: "local",
  syncIntervalMs: DEFAULT_SHARED_SYNC_INTERVAL_MS,
};

export function buildIdleUpdateState(): UpdateState {
  return {
    available: false,
    currentVersion: APP_VERSION,
    status: "idle",
  };
}

export function chooseFreshUpdateState(current: UpdateState, next: UpdateState): UpdateState {
  if (current.latestVersion && current.latestVersion === next.latestVersion) {
    return getUpdateStatusRank(current.status) > getUpdateStatusRank(next.status) ? current : next;
  }

  return next;
}

function getUpdateStatusRank(status: UpdateState["status"]): number {
  switch (status) {
    case "idle":
      return 0;
    case "checking":
      return 1;
    case "not-available":
      return 2;
    case "available":
      return 3;
    case "downloading":
      return 4;
    case "ready":
      return 5;
    case "installing":
      return 6;
    case "error":
      return 7;
    default:
      return 0;
  }
}

export function sharedStatusesMatch(left: InventorySharedStatus, right: InventorySharedStatus): boolean {
  return (
    left.available === right.available &&
    left.canModify === right.canModify &&
    left.enabled === right.enabled &&
    left.hasLocalOnlyChanges === right.hasLocalOnlyChanges &&
    left.message === right.message &&
    left.mutationMode === right.mutationMode &&
    left.revision === right.revision &&
    left.sharedDbPath === right.sharedDbPath &&
    left.sharedRootPath === right.sharedRootPath &&
    left.syncIntervalMs === right.syncIntervalMs
  );
}

export function normalizeSharedStatus(status: InventorySharedStatus): InventorySharedStatus {
  if (status.syncIntervalMs === undefined) {
    return status;
  }

  const syncIntervalMs = clampSharedSyncIntervalMs(status.syncIntervalMs);
  return syncIntervalMs === status.syncIntervalMs ? status : { ...status, syncIntervalMs };
}

export function clampSharedSyncIntervalMs(syncIntervalMs: number | undefined): number {
  if (typeof syncIntervalMs !== "number" || !Number.isFinite(syncIntervalMs)) {
    return DEFAULT_SHARED_SYNC_INTERVAL_MS;
  }

  return Math.min(MAX_SHARED_SYNC_INTERVAL_MS, Math.max(MIN_SHARED_SYNC_INTERVAL_MS, syncIntervalMs));
}

export function hasDesktopBridge(): boolean {
  return typeof window !== "undefined" && Boolean(window.inventoryDesktop?.isDesktop);
}

export function buildDefaultStatusMessage(
  totalCount: number,
  verifiedCount: number,
  dataSource: "desktop" | "mock",
  sharedStatus: InventorySharedStatus,
): string {
  const summary = `Total: ${totalCount} | Verified: ${verifiedCount}/${totalCount}`;
  if (dataSource !== "desktop" || !sharedStatus.message) {
    return summary;
  }
  return `${summary} | ${sharedStatus.message}`;
}

export function buildLocalCreatedEntry(input: InventoryEntryInput): InventoryEntry {
  const timestamp = new Date().toISOString();

  return {
    id: `local-${Date.now()}`,
    entryUuid: "",
    assetNumber: input.assetNumber,
    serialNumber: input.serialNumber,
    qty: input.qty,
    manufacturer: input.manufacturer,
    model: input.model,
    description: input.description,
    projectName: input.projectName,
    location: input.location,
    assignedTo: input.assignedTo,
    links: input.links,
    notes: input.notes,
    lifecycleStatus: input.lifecycleStatus,
    workingStatus: input.workingStatus,
    condition: input.condition,
    verifiedInSurvey: input.verifiedInSurvey,
    archived: input.archived,
    manualEntry: true,
    picturePath: input.picturePath ?? "",
    createdAt: timestamp,
    updatedAt: timestamp,
  };
}

export function buildLocalUpdatedEntry(existingEntry: InventoryEntry, input: InventoryEntryInput): InventoryEntry {
  return {
    ...existingEntry,
    assetNumber: input.assetNumber,
    serialNumber: input.serialNumber,
    qty: input.qty,
    manufacturer: input.manufacturer,
    model: input.model,
    description: input.description,
    projectName: input.projectName,
    location: input.location,
    assignedTo: input.assignedTo,
    links: input.links,
    notes: input.notes,
    lifecycleStatus: input.lifecycleStatus,
    workingStatus: input.workingStatus,
    condition: input.condition,
    verifiedInSurvey: input.verifiedInSurvey,
    archived: input.archived,
    picturePath: input.picturePath ?? "",
    updatedAt: new Date().toISOString(),
  };
}

export function readTheme(): ThemeMode {
  if (typeof window === "undefined") {
    return "light";
  }

  const storedTheme = window.localStorage.getItem(THEME_STORAGE_KEY);
  return storedTheme === "dark" ? "dark" : "light";
}

export function readColorRows(): boolean {
  if (typeof window === "undefined") {
    return true;
  }

  const storedValue = window.localStorage.getItem(COLOR_ROWS_STORAGE_KEY);
  return storedValue == null ? true : storedValue === "true";
}

export function readColumnVisibility(): Record<ColumnKey, boolean> {
  if (typeof window === "undefined") {
    return buildDefaultColumnVisibility();
  }

  const storedValue = window.localStorage.getItem(COLUMN_VISIBILITY_STORAGE_KEY);
  if (!storedValue) {
    return buildDefaultColumnVisibility();
  }

  try {
    return mergeColumnVisibility(JSON.parse(storedValue) as Partial<Record<ColumnKey, boolean>>);
  } catch {
    return buildDefaultColumnVisibility();
  }
}
