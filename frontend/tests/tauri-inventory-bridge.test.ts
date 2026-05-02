import { act } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { APP_VERSION } from "@/app/branding";
import type { InventoryEntryInput, UpdateState } from "@/features/inventory/types";

describe("tauri inventory bridge", () => {
  beforeEach(() => {
    delete window.inventoryDesktop;
  });

  it("does not register the desktop bridge in a normal browser session", async () => {
    const tauriGlobal = globalThis as { isTauri?: boolean };
    const originalIsTauri = tauriGlobal.isTauri;
    tauriGlobal.isTauri = false;
    delete window.inventoryDesktop;
    vi.resetModules();

    try {
      await import("@/integrations/tauri/tauriInventoryBridge");
      expect(window.inventoryDesktop).toBeUndefined();
    } finally {
      if (originalIsTauri === undefined) {
        Reflect.deleteProperty(tauriGlobal, "isTauri");
      } else {
        tauriGlobal.isTauri = originalIsTauri;
      }
    }
  });

  it("registers and cleans up Tauri shared inventory change events", async () => {
    type SharedInventoryChangedEvent = {
      event: string;
      id: number;
      payload: unknown;
    };

    const sharedChangeHandlerRef: {
      current: ((event: SharedInventoryChangedEvent) => void) | null;
    } = { current: null };
    const unlisten = vi.fn();
    const listen = vi.fn((_eventName: string, handler: (event: SharedInventoryChangedEvent) => void) => {
      sharedChangeHandlerRef.current = handler;
      return Promise.resolve(unlisten);
    });
    const invoke = vi.fn();

    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke,
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({ listen }));
    const { installWindowStatePersistence } = mockWindowStatePersistence();

    try {
      await import("@/integrations/tauri/tauriInventoryBridge");
      const callback = vi.fn();
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;
      const cleanup = desktopBridge?.onSharedInventoryChanged?.(callback);

      expect(cleanup).toEqual(expect.any(Function));
      expect(installWindowStatePersistence).toHaveBeenCalledTimes(1);
      expect(listen).toHaveBeenCalledWith("inventory:shared-changed", expect.any(Function));

      sharedChangeHandlerRef.current?.({ event: "inventory:shared-changed", id: 1, payload: null });
      expect(callback).toHaveBeenCalledTimes(1);

      await flushAsyncWork();
      cleanup?.();

      expect(unlisten).toHaveBeenCalledTimes(1);
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.doUnmock("@/integrations/tauri/windowState");
      vi.resetModules();
    }
  });

  it("runs Tauri shared inventory cleanup after pending listener registration resolves", async () => {
    const deferredUnlisten = createDeferred<() => void>();
    const unlisten = vi.fn();
    const listen = vi.fn(() => deferredUnlisten.promise);

    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke: vi.fn(),
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({ listen }));
    mockWindowStatePersistence();

    try {
      await import("@/integrations/tauri/tauriInventoryBridge");
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;
      const cleanup = desktopBridge?.onSharedInventoryChanged?.(() => undefined);

      cleanup?.();
      expect(unlisten).not.toHaveBeenCalled();

      await act(async () => {
        deferredUnlisten.resolve(unlisten);
        await deferredUnlisten.promise;
      });

      expect(unlisten).toHaveBeenCalledTimes(1);
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.doUnmock("@/integrations/tauri/windowState");
      vi.resetModules();
    }
  });

  it("rejects malformed Tauri inventory payloads before they reach React state", async () => {
    const invoke = vi.fn().mockResolvedValue({
      dbPath: "inventory.feox",
      entries: [{ description: "missing id" }],
      shared: { message: "ready" },
    });

    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke,
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({
      listen: vi.fn(() => Promise.resolve(() => undefined)),
    }));
    mockWindowStatePersistence();

    try {
      await import("@/integrations/tauri/tauriInventoryBridge");
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;

      await expect(desktopBridge?.loadInventory()).rejects.toThrow("Invalid inventory entry");
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.doUnmock("@/integrations/tauri/windowState");
      vi.resetModules();
    }
  });

  it("rejects malformed Tauri query, mutation, delete, and export payloads", async () => {
    const invoke = vi.fn((command: string) => {
      switch (command) {
        case "query_inventory":
          return Promise.resolve({
            counts: {},
            dbPath: "inventory.feox",
            entries: "not-an-array",
            shared: { message: "ready" },
            totalFiltered: 0,
          });
        case "delete_entry":
          return Promise.resolve({ message: "missing entry id" });
        case "export_excel":
          return Promise.resolve({ outputPath: 42 });
        default:
          return Promise.resolve({
            entry: { description: "missing id" },
            message: "saved",
            shared: { message: "ready" },
          });
      }
    });

    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke,
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({
      listen: vi.fn(() => Promise.resolve(() => undefined)),
    }));
    mockWindowStatePersistence();

    try {
      await import("@/integrations/tauri/tauriInventoryBridge");
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;

      await expect(
        desktopBridge?.queryInventory?.({
          filters: { assetNumber: "", description: "", location: "", manufacturer: "", model: "" },
          query: "",
          scope: "inventory",
          sort: { column: "manufacturer", direction: "asc" },
        }),
      ).rejects.toThrow("Invalid inventory entries");
      await expect(desktopBridge?.createEntry(validEntryInput())).rejects.toThrow("Invalid inventory entry");
      await expect(desktopBridge?.updateEntry("1", validEntryInput())).rejects.toThrow("Invalid inventory entry");
      await expect(desktopBridge?.toggleVerifiedEntry("1", true)).rejects.toThrow("Invalid inventory entry");
      await expect(desktopBridge?.setArchivedEntry("1", true)).rejects.toThrow("Invalid inventory entry");
      await expect(desktopBridge?.deleteEntry("1")).rejects.toThrow("missing entry id");
      await expect(desktopBridge?.exportExcel?.()).rejects.toThrow("Excel export payload");
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.doUnmock("@/integrations/tauri/windowState");
      vi.resetModules();
    }
  });

  it("normalizes malformed but recoverable Tauri inventory fields", async () => {
    const invoke = vi.fn().mockResolvedValue({
      dbPath: "inventory.feox",
      entries: [
        {
          id: "1",
          archived: false,
          description: "Caliper",
          lifecycleStatus: "bad",
          qty: "not-a-number",
          verifiedInSurvey: true,
          workingStatus: "also-bad",
        },
      ],
      shared: { message: "ready", mutationMode: "shared" },
    });

    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke,
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({
      listen: vi.fn(() => Promise.resolve(() => undefined)),
    }));
    mockWindowStatePersistence();

    try {
      await import("@/integrations/tauri/tauriInventoryBridge");
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;

      await expect(desktopBridge?.loadInventory()).resolves.toMatchObject({
        entries: [
          {
            id: "1",
            lifecycleStatus: "active",
            qty: null,
            workingStatus: "unknown",
          },
        ],
        shared: { canModify: true, mutationMode: "shared" },
      });
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.doUnmock("@/integrations/tauri/windowState");
      vi.resetModules();
    }
  });

  it("normalizes malformed shared status values at the bridge boundary", async () => {
    const invoke = vi.fn().mockResolvedValue({
      dbPath: "inventory.feox",
      entries: [validBridgeEntry()],
      shared: "not-a-shared-status",
    });

    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke,
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({
      listen: vi.fn(() => Promise.resolve(() => undefined)),
    }));
    mockWindowStatePersistence();

    try {
      await import("@/integrations/tauri/tauriInventoryBridge");
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;

      await expect(desktopBridge?.loadInventory()).resolves.toMatchObject({
        entries: [{ id: "1" }],
        shared: {
          available: false,
          canModify: true,
          enabled: true,
          message: "Shared sync status unavailable.",
          mutationMode: "local",
        },
      });
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.doUnmock("@/integrations/tauri/windowState");
      vi.resetModules();
    }
  });

  it("backs desktop update checks with Tauri updater progress events", async () => {
    const receivedStates: UpdateState[] = [];
    const update = {
      body: "Signed updater release",
      close: vi.fn().mockResolvedValue(undefined),
      currentVersion: APP_VERSION,
      date: "2026-04-29T00:00:00Z",
      download: vi.fn(async (onEvent?: (event: unknown) => void) => {
        onEvent?.({ event: "Started", data: { contentLength: 100 } });
        onEvent?.({ event: "Progress", data: { chunkLength: 25 } });
        onEvent?.({ event: "Finished" });
      }),
      install: vi.fn().mockResolvedValue(undefined),
      version: "0.9.8",
    };
    const check = vi.fn().mockResolvedValue(update);

    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke: vi.fn(),
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({
      listen: vi.fn(() => Promise.resolve(() => undefined)),
    }));
    vi.doMock("@tauri-apps/plugin-updater", () => ({ check }));
    const { saveCurrentWindowState } = mockWindowStatePersistence();

    try {
      await import("@/integrations/tauri/tauriInventoryBridge");
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;
      const cleanup = desktopBridge?.onUpdateStateChanged?.((state) => {
        receivedStates.push(state);
      });

      const availableState = await desktopBridge?.checkForUpdate?.();
      expect(check).toHaveBeenCalledTimes(1);
      expect(availableState).toMatchObject({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.8",
        notes: "Signed updater release",
        publishedAt: "2026-04-29T00:00:00Z",
        status: "available",
      });

      const readyState = await desktopBridge?.downloadUpdate?.();
      expect(update.download).toHaveBeenCalledTimes(1);
      expect(readyState).toMatchObject({
        available: true,
        downloadPhase: "ready",
        downloadProgress: 100,
        latestVersion: "0.9.8",
        status: "ready",
      });
      expect(receivedStates).toEqual(
        expect.arrayContaining([
          expect.objectContaining({ status: "checking" }),
          expect.objectContaining({ status: "available" }),
          expect.objectContaining({ downloadPhase: "copying", downloadProgress: 25 }),
          expect.objectContaining({ downloadPhase: "verifying", downloadProgress: 100 }),
          expect.objectContaining({ downloadPhase: "ready", downloadProgress: 100 }),
        ]),
      );

      await desktopBridge?.installUpdate?.();
      expect(saveCurrentWindowState).toHaveBeenCalledTimes(1);
      expect(saveCurrentWindowState.mock.invocationCallOrder[0]).toBeLessThan(update.install.mock.invocationCallOrder[0]);
      expect(update.install).toHaveBeenCalledTimes(1);
      cleanup?.();
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.doUnmock("@tauri-apps/plugin-updater");
      vi.doUnmock("@/integrations/tauri/windowState");
      vi.resetModules();
    }
  });

  it("normalizes malformed updater metadata before publishing state", async () => {
    const receivedStates: UpdateState[] = [];
    const update = {
      body: { unexpected: true },
      currentVersion: 42,
      date: 10,
      download: vi.fn(),
      install: vi.fn(),
      version: 99,
    };
    const check = vi.fn().mockResolvedValue(update);

    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke: vi.fn(),
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({
      listen: vi.fn(() => Promise.resolve(() => undefined)),
    }));
    vi.doMock("@tauri-apps/plugin-updater", () => ({ check }));
    mockWindowStatePersistence();

    try {
      await import("@/integrations/tauri/tauriInventoryBridge");
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;
      const cleanup = desktopBridge?.onUpdateStateChanged?.((state) => {
        receivedStates.push(state);
      });

      const state = await desktopBridge?.checkForUpdate?.();

      expect(state).toMatchObject({
        available: true,
        currentVersion: APP_VERSION,
        status: "available",
      });
      expect(state?.latestVersion).toBeUndefined();
      expect(state?.notes).toBeUndefined();
      expect(receivedStates.at(-1)).toEqual(state);
      cleanup?.();
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.doUnmock("@tauri-apps/plugin-updater");
      vi.doUnmock("@/integrations/tauri/windowState");
      vi.resetModules();
    }
  });
});

function mockWindowStatePersistence() {
  const installWindowStatePersistence = vi.fn();
  const saveCurrentWindowState = vi.fn().mockResolvedValue(undefined);
  vi.doMock("@/integrations/tauri/windowState", () => ({
    installWindowStatePersistence,
    saveCurrentWindowState,
  }));

  return { installWindowStatePersistence, saveCurrentWindowState };
}

function validBridgeEntry(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: "1",
    archived: false,
    description: "Caliper",
    lifecycleStatus: "active",
    qty: 1,
    verifiedInSurvey: false,
    workingStatus: "working",
    ...overrides,
  };
}

function validEntryInput(): InventoryEntryInput {
  return {
    archived: false,
    assetNumber: "",
    assignedTo: "",
    condition: "",
    description: "Caliper",
    lifecycleStatus: "active",
    links: "",
    location: "",
    manufacturer: "",
    model: "",
    notes: "",
    picturePath: "",
    projectName: "",
    qty: 1,
    serialNumber: "",
    verifiedInSurvey: false,
    workingStatus: "working",
  };
}

function createDeferred<T>(): {
  promise: Promise<T>;
  reject: (reason?: unknown) => void;
  resolve: (value: T) => void;
} {
  let reject!: (reason?: unknown) => void;
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });

  return { promise, reject, resolve };
}

async function flushAsyncWork(): Promise<void> {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}
