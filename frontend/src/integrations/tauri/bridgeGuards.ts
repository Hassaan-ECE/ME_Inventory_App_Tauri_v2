import { APP_VERSION } from "@/app/branding";
import type { InventorySyncResult } from "@/integrations/tauri/desktop-bridge";
import {
  LIFECYCLE_OPTIONS,
  WORKING_STATUS_OPTIONS,
  type InventoryCounts,
  type InventoryDeleteMutationResult,
  type InventoryEntry,
  type InventoryEntryMutationResult,
  type InventoryQueryResult,
  type InventorySharedStatus,
  type ExcelExportResult,
  type LifecycleStatus,
  type UpdateState,
  type UpdateStatus,
  type WorkingStatus,
} from "@/features/inventory/types";

const UPDATE_STATUSES = new Set<UpdateStatus>([
  "idle",
  "checking",
  "available",
  "not-available",
  "downloading",
  "ready",
  "installing",
  "error",
]);

export function parseInventorySyncResult(value: unknown): InventorySyncResult {
  const record = requireRecord(value, "inventory sync payload");
  return {
    dbPath: optionalString(record.dbPath) ?? "",
    entries: requireArray(record.entries, "inventory entries").map(parseInventoryEntry),
    entriesChanged: optionalBoolean(record.entriesChanged),
    shared: parseSharedStatus(record.shared),
  };
}

export function parseInventoryQueryResult(value: unknown): InventoryQueryResult {
  const record = requireRecord(value, "inventory query payload");
  return {
    counts: parseCounts(record.counts),
    dbPath: optionalString(record.dbPath) ?? "",
    entries: requireArray(record.entries, "inventory entries").map(parseInventoryEntry),
    shared: parseSharedStatus(record.shared),
    totalFiltered: optionalFiniteNumber(record.totalFiltered) ?? 0,
  };
}

export function parseEntryMutationResult(value: unknown): InventoryEntryMutationResult {
  const record = requireRecord(value, "inventory mutation payload");
  return {
    entry: parseInventoryEntry(record.entry),
    message: optionalString(record.message) ?? "Inventory entry saved.",
    mutationMode: record.mutationMode === "shared" ? "shared" : "local",
    shared: parseSharedStatus(record.shared),
  };
}

export function parseDeleteMutationResult(value: unknown): InventoryDeleteMutationResult {
  const record = requireRecord(value, "inventory delete payload");
  const entryId = optionalString(record.entryId);
  if (!entryId) {
    throw new Error("Invalid inventory delete payload: missing entry id.");
  }

  return {
    entryId,
    message: optionalString(record.message) ?? "Inventory entry deleted.",
    mutationMode: record.mutationMode === "shared" ? "shared" : "local",
    shared: parseSharedStatus(record.shared),
  };
}

export function parseExcelExportResult(value: unknown): ExcelExportResult {
  const record = requireRecord(value, "Excel export payload");
  if (typeof record.canceled !== "boolean") {
    throw new Error("Invalid Excel export payload: missing canceled flag.");
  }

  return {
    canceled: record.canceled,
    error: optionalString(record.error),
    outputPath: optionalString(record.outputPath),
  };
}

export function parseNullableString(value: unknown, label: string): string | null {
  if (value === null || value === undefined) {
    return null;
  }
  if (typeof value !== "string") {
    throw new Error(`Invalid ${label}: expected a string or null.`);
  }
  return value;
}

export function parseBoolean(value: unknown, label: string): boolean {
  if (typeof value !== "boolean") {
    throw new Error(`Invalid ${label}: expected a boolean.`);
  }
  return value;
}

export function parseUpdateState(value: unknown): UpdateState {
  const record = requireRecord(value, "update state");
  const status = UPDATE_STATUSES.has(record.status as UpdateStatus)
    ? (record.status as UpdateStatus)
    : "error";
  return {
    available: record.available === true,
    currentVersion: optionalString(record.currentVersion) ?? APP_VERSION,
    downloadPhase:
      record.downloadPhase === "copying" ||
      record.downloadPhase === "verifying" ||
      record.downloadPhase === "ready"
        ? record.downloadPhase
        : undefined,
    downloadProgress: optionalFiniteNumber(record.downloadProgress),
    downloadedInstallerPath: optionalString(record.downloadedInstallerPath),
    error: optionalString(record.error),
    installLogPath: optionalString(record.installLogPath),
    installerPid: optionalFiniteNumber(record.installerPid),
    latestVersion: optionalString(record.latestVersion),
    notes: optionalString(record.notes),
    publishedAt: optionalString(record.publishedAt),
    status,
  };
}

function parseInventoryEntry(value: unknown): InventoryEntry {
  const record = requireRecord(value, "inventory entry");
  const id = optionalString(record.id);
  if (!id) {
    throw new Error("Invalid inventory entry: missing id.");
  }

  return {
    id,
    archived: record.archived === true,
    assetNumber: optionalString(record.assetNumber) ?? "",
    assignedTo: optionalString(record.assignedTo),
    condition: optionalString(record.condition),
    createdAt: optionalString(record.createdAt),
    databaseId: optionalFiniteNumber(record.databaseId),
    description: optionalString(record.description) ?? "",
    entryUuid: optionalString(record.entryUuid),
    lifecycleStatus: parseLifecycleStatus(record.lifecycleStatus),
    links: optionalString(record.links) ?? "",
    location: optionalString(record.location) ?? "",
    manufacturer: optionalString(record.manufacturer) ?? "",
    manualEntry: record.manualEntry === true,
    model: optionalString(record.model) ?? "",
    notes: optionalString(record.notes) ?? "",
    picturePath: optionalString(record.picturePath),
    projectName: optionalString(record.projectName) ?? "",
    qty: parseNullableFiniteNumber(record.qty),
    serialNumber: optionalString(record.serialNumber),
    updatedAt: optionalString(record.updatedAt) ?? "",
    verifiedInSurvey: record.verifiedInSurvey === true,
    workingStatus: parseWorkingStatus(record.workingStatus),
  };
}

function parseSharedStatus(value: unknown): InventorySharedStatus {
  const record = isRecord(value) ? value : {};
  return {
    available: record.available === true,
    canModify: record.canModify !== false,
    enabled: record.enabled !== false,
    hasLocalOnlyChanges: optionalBoolean(record.hasLocalOnlyChanges),
    lastSnapshotId: optionalString(record.lastSnapshotId),
    message: optionalString(record.message) ?? "Shared sync status unavailable.",
    mutationMode: record.mutationMode === "shared" ? "shared" : "local",
    revision: optionalString(record.revision),
    sharedRootPath: optionalString(record.sharedRootPath),
    syncIntervalMs: optionalFiniteNumber(record.syncIntervalMs),
  };
}

function parseCounts(value: unknown): InventoryCounts {
  const record = isRecord(value) ? value : {};
  return {
    archive: optionalFiniteNumber(record.archive) ?? 0,
    inventory: optionalFiniteNumber(record.inventory) ?? 0,
    total: optionalFiniteNumber(record.total) ?? 0,
    verified: optionalFiniteNumber(record.verified) ?? 0,
  };
}

function parseLifecycleStatus(value: unknown): LifecycleStatus {
  return LIFECYCLE_OPTIONS.includes(value as LifecycleStatus) ? (value as LifecycleStatus) : "active";
}

function parseWorkingStatus(value: unknown): WorkingStatus {
  return WORKING_STATUS_OPTIONS.includes(value as WorkingStatus) ? (value as WorkingStatus) : "unknown";
}

function parseNullableFiniteNumber(value: unknown): number | null {
  if (value === null || value === undefined) {
    return null;
  }
  return optionalFiniteNumber(value) ?? null;
}

function optionalFiniteNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function optionalBoolean(value: unknown): boolean | undefined {
  return typeof value === "boolean" ? value : undefined;
}

function requireArray(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) {
    throw new Error(`Invalid ${label}: expected an array.`);
  }
  return value;
}

function requireRecord(value: unknown, label: string): Record<string, unknown> {
  if (!isRecord(value)) {
    throw new Error(`Invalid ${label}: expected an object.`);
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
