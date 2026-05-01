import type { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";
import type { Monitor, Window as TauriWindow } from "@tauri-apps/api/window";

const WINDOW_STATE_STORAGE_KEY = "meInventory.windowState.v1";
const MIN_WINDOW_WIDTH = 1100;
const MIN_WINDOW_HEIGHT = 720;
const MAX_WINDOW_DIMENSION = 20_000;
const MIN_VISIBLE_WIDTH = 160;
const MIN_VISIBLE_HEIGHT = 120;
const SAVE_DEBOUNCE_MS = 400;

type PhysicalPositionConstructor = new (x: number, y: number) => PhysicalPosition;
type PhysicalSizeConstructor = new (width: number, height: number) => PhysicalSize;

export interface StoredWindowState {
  height: number;
  maximized: boolean;
  savedAt: string;
  width: number;
  x: number;
  y: number;
}

let installed = false;
let saveTimer: number | null = null;

export function installWindowStatePersistence(): void {
  if (installed || typeof window === "undefined") {
    return;
  }

  installed = true;
  void installWindowStatePersistenceAsync().catch(() => undefined);
}

export async function saveCurrentWindowState(): Promise<void> {
  if (typeof window === "undefined") {
    return;
  }

  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await saveWindowState(getCurrentWindow());
}

export function parseStoredWindowState(rawState: string | null): StoredWindowState | null {
  if (!rawState) {
    return null;
  }

  try {
    const parsed: unknown = JSON.parse(rawState);
    if (!isRecord(parsed)) {
      return null;
    }

    const x = finiteNumber(parsed.x);
    const y = finiteNumber(parsed.y);
    const width = finiteNumber(parsed.width);
    const height = finiteNumber(parsed.height);
    if (x === null || y === null || width === null || height === null) {
      return null;
    }

    return {
      height: clampDimension(height, MIN_WINDOW_HEIGHT),
      maximized: parsed.maximized === true,
      savedAt: typeof parsed.savedAt === "string" ? parsed.savedAt : "",
      width: clampDimension(width, MIN_WINDOW_WIDTH),
      x: Math.round(x),
      y: Math.round(y),
    };
  } catch {
    return null;
  }
}

export function isWindowStateVisibleOnAnyMonitor(
  state: Pick<StoredWindowState, "height" | "width" | "x" | "y">,
  monitors: Pick<Monitor, "workArea">[],
): boolean {
  if (monitors.length === 0) {
    return true;
  }

  const right = state.x + state.width;
  const bottom = state.y + state.height;

  return monitors.some((monitor) => {
    const workArea = monitor.workArea;
    const workAreaRight = workArea.position.x + workArea.size.width;
    const workAreaBottom = workArea.position.y + workArea.size.height;
    const visibleWidth = Math.min(right, workAreaRight) - Math.max(state.x, workArea.position.x);
    const visibleHeight = Math.min(bottom, workAreaBottom) - Math.max(state.y, workArea.position.y);

    return (
      visibleWidth >= Math.min(MIN_VISIBLE_WIDTH, state.width) &&
      visibleHeight >= Math.min(MIN_VISIBLE_HEIGHT, state.height)
    );
  });
}

async function installWindowStatePersistenceAsync(): Promise<void> {
  const [{ getCurrentWindow, availableMonitors }, { PhysicalPosition, PhysicalSize }] = await Promise.all([
    import("@tauri-apps/api/window"),
    import("@tauri-apps/api/dpi"),
  ]);
  const appWindow = getCurrentWindow();

  await restoreWindowState(appWindow, availableMonitors, PhysicalPosition, PhysicalSize);
  scheduleWindowStateSave();

  void appWindow.onMoved(() => {
    scheduleWindowStateSave();
  }).catch(() => undefined);
  void appWindow.onResized(() => {
    scheduleWindowStateSave();
  }).catch(() => undefined);
}

async function restoreWindowState(
  appWindow: TauriWindow,
  availableMonitors: () => Promise<Monitor[]>,
  PhysicalPosition: PhysicalPositionConstructor,
  PhysicalSize: PhysicalSizeConstructor,
): Promise<void> {
  const storedState = parseStoredWindowState(window.localStorage.getItem(WINDOW_STATE_STORAGE_KEY));
  if (!storedState) {
    return;
  }

  const monitors = await availableMonitors().catch(() => []);
  const canRestorePosition = isWindowStateVisibleOnAnyMonitor(storedState, monitors);

  await appWindow.setSize(new PhysicalSize(storedState.width, storedState.height)).catch(() => undefined);
  if (canRestorePosition) {
    await appWindow.setPosition(new PhysicalPosition(storedState.x, storedState.y)).catch(() => undefined);
  }
  if (storedState.maximized) {
    await appWindow.maximize().catch(() => undefined);
  }
}

function scheduleWindowStateSave(): void {
  if (typeof window === "undefined") {
    return;
  }

  if (saveTimer !== null) {
    window.clearTimeout(saveTimer);
  }

  saveTimer = window.setTimeout(() => {
    saveTimer = null;
    void saveCurrentWindowState().catch(() => undefined);
  }, SAVE_DEBOUNCE_MS);
}

async function saveWindowState(appWindow: TauriWindow): Promise<void> {
  const minimized = await appWindow.isMinimized().catch(() => false);
  if (minimized) {
    return;
  }

  const [position, size, maximized] = await Promise.all([
    appWindow.outerPosition(),
    appWindow.outerSize(),
    appWindow.isMaximized().catch(() => false),
  ]);
  const state: StoredWindowState = {
    height: clampDimension(size.height, MIN_WINDOW_HEIGHT),
    maximized,
    savedAt: new Date().toISOString(),
    width: clampDimension(size.width, MIN_WINDOW_WIDTH),
    x: Math.round(position.x),
    y: Math.round(position.y),
  };

  window.localStorage.setItem(WINDOW_STATE_STORAGE_KEY, JSON.stringify(state));
}

function clampDimension(value: number, minimum: number): number {
  return Math.min(MAX_WINDOW_DIMENSION, Math.max(minimum, Math.round(value)));
}

function finiteNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
