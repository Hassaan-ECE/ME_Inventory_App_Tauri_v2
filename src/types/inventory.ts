export type InventoryScope = "inventory" | "archive";
export type ThemeMode = "light" | "dark";
export type SortDirection = "asc" | "desc";
export type LifecycleStatus = "active" | "repair" | "scrapped" | "missing" | "rental";
export type WorkingStatus = "working" | "limited" | "not_working" | "unknown";

export const LIFECYCLE_OPTIONS = ["active", "repair", "scrapped", "missing", "rental"] as const satisfies readonly LifecycleStatus[];
export const WORKING_STATUS_OPTIONS = ["unknown", "working", "limited", "not_working"] as const satisfies readonly WorkingStatus[];

export interface InventoryEntry {
  id: string;
  databaseId?: number;
  assetNumber: string;
  serialNumber?: string;
  qty: number | null;
  manufacturer: string;
  model: string;
  description: string;
  projectName: string;
  location: string;
  assignedTo?: string;
  links: string;
  notes: string;
  lifecycleStatus: LifecycleStatus;
  workingStatus: WorkingStatus;
  condition?: string;
  verifiedInSurvey: boolean;
  archived: boolean;
  createdAt?: string;
  updatedAt: string;
  entryUuid?: string;
  manualEntry?: boolean;
  picturePath?: string;
}

export interface InventoryEntryInput {
  assetNumber: string;
  serialNumber: string;
  qty: number | null;
  manufacturer: string;
  model: string;
  description: string;
  projectName: string;
  location: string;
  assignedTo: string;
  links: string;
  notes: string;
  lifecycleStatus: LifecycleStatus;
  workingStatus: WorkingStatus;
  condition: string;
  verifiedInSurvey: boolean;
  archived: boolean;
  picturePath?: string;
}

export interface InventorySharedStatus {
  available: boolean;
  canModify: boolean;
  enabled: boolean;
  hasLocalOnlyChanges?: boolean;
  message: string;
  mutationMode?: "shared" | "local";
  revision?: string;
  sharedDbPath?: string;
  sharedRootPath?: string;
  syncIntervalMs?: number;
}

export interface InventoryCounts {
  archive: number;
  inventory: number;
  total: number;
  verified: number;
}

export type InventoryMutationMode = "shared" | "local";

export interface InventoryEntryMutationResult {
  entry: InventoryEntry;
  message: string;
  mutationMode: InventoryMutationMode;
  shared?: InventorySharedStatus;
}

export interface InventoryDeleteMutationResult {
  entryId: string;
  message: string;
  mutationMode: InventoryMutationMode;
  shared?: InventorySharedStatus;
}

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "not-available"
  | "downloading"
  | "ready"
  | "installing"
  | "error";

export interface UpdateState {
  available: boolean;
  currentVersion: string;
  downloadPhase?: "copying" | "verifying" | "ready";
  downloadProgress?: number;
  downloadedInstallerPath?: string;
  error?: string;
  installLogPath?: string;
  installerPid?: number;
  latestVersion?: string;
  notes?: string;
  publishedAt?: string;
  status: UpdateStatus;
}

export interface FilterState {
  assetNumber: string;
  manufacturer: string;
  model: string;
  description: string;
  location: string;
}

export interface InventoryQueryInput {
  filters: FilterState;
  limit?: number;
  offset?: number;
  query: string;
  scope: InventoryScope;
  sort: SortState;
}

export interface InventoryQueryResult {
  counts: InventoryCounts;
  dbPath: string;
  entries: InventoryEntry[];
  shared: InventorySharedStatus;
  totalFiltered: number;
}

export interface ColumnConfig {
  key:
    | "verified"
    | "assetNumber"
    | "qty"
    | "manufacturer"
    | "model"
    | "description"
    | "projectName"
    | "location"
    | "links";
  label: string;
  defaultVisible: boolean;
  sortable: boolean;
  align?: "left" | "center";
}

export type ColumnKey = ColumnConfig["key"];

export interface SortState {
  column: ColumnKey;
  direction: SortDirection;
}

export const INVENTORY_COLUMNS = [
  { key: "verified", label: "Verified", defaultVisible: true, sortable: true, align: "center" },
  { key: "assetNumber", label: "Asset #", defaultVisible: false, sortable: true },
  { key: "qty", label: "Qty", defaultVisible: true, sortable: true, align: "center" },
  { key: "manufacturer", label: "Manufacturer", defaultVisible: true, sortable: true },
  { key: "model", label: "Model", defaultVisible: true, sortable: true },
  { key: "description", label: "Description", defaultVisible: true, sortable: true },
  { key: "projectName", label: "Project", defaultVisible: false, sortable: true },
  { key: "location", label: "Location", defaultVisible: true, sortable: true },
  { key: "links", label: "Links", defaultVisible: true, sortable: true },
] as const satisfies readonly ColumnConfig[];
