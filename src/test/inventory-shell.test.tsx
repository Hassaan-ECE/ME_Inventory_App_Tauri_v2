import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { APP_CREDIT, APP_DISPLAY_NAME, APP_VERSION } from "@/branding";
import { InventoryShell } from "@/components/inventory/InventoryShell";
import type { InventorySyncResult } from "@/types/desktop-bridge";
import type {
  InventoryCounts,
  InventoryEntry,
  InventoryQueryResult,
  InventorySharedStatus,
  UpdateState,
} from "@/types/inventory";

const CONNECTED_SHARED_STATUS: InventorySharedStatus = {
  available: true,
  canModify: true,
  enabled: true,
  message: "",
  mutationMode: "shared",
  syncIntervalMs: 60_000,
};
const LOCAL_SHARED_STATUS: InventorySharedStatus = {
  available: false,
  canModify: true,
  enabled: true,
  hasLocalOnlyChanges: true,
  message: "Shared workspace unavailable. Saving changes locally.",
  mutationMode: "local",
  syncIntervalMs: 60_000,
};
const DISABLED_SHARED_STATUS: InventorySharedStatus = {
  available: true,
  canModify: true,
  enabled: false,
  message: "",
  mutationMode: "local",
};
const EMPTY_TEST_COUNTS: InventoryCounts = {
  archive: 0,
  inventory: 0,
  total: 0,
  verified: 0,
};
const TEST_DB_PATH = "D:/coding/ME Inventory/data/me_inventory.db";

describe("InventoryShell shell", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
    delete window.inventoryDesktop;
  });

  it("does not register the desktop bridge in a normal browser session", async () => {
    const tauriGlobal = globalThis as { isTauri?: boolean };
    const originalIsTauri = tauriGlobal.isTauri;
    tauriGlobal.isTauri = false;
    delete window.inventoryDesktop;
    vi.resetModules();

    try {
      await import("@/lib/tauriInventoryBridge");
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

    delete window.inventoryDesktop;
    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke,
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({ listen }));

    try {
      await import("@/lib/tauriInventoryBridge");
      const callback = vi.fn();
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;
      const cleanup = desktopBridge?.onSharedInventoryChanged?.(callback);

      expect(cleanup).toEqual(expect.any(Function));
      expect(listen).toHaveBeenCalledWith("inventory:shared-changed", expect.any(Function));

      sharedChangeHandlerRef.current?.({ event: "inventory:shared-changed", id: 1, payload: null });
      expect(callback).toHaveBeenCalledTimes(1);

      await flushAsyncWork();
      cleanup?.();

      expect(unlisten).toHaveBeenCalledTimes(1);
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.resetModules();
    }
  });

  it("runs Tauri shared inventory cleanup after pending listener registration resolves", async () => {
    const deferredUnlisten = createDeferred<() => void>();
    const unlisten = vi.fn();
    const listen = vi.fn(() => deferredUnlisten.promise);

    delete window.inventoryDesktop;
    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke: vi.fn(),
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({ listen }));

    try {
      await import("@/lib/tauriInventoryBridge");
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

    delete window.inventoryDesktop;
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

    try {
      await import("@/lib/tauriInventoryBridge");
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
      expect(update.install).toHaveBeenCalledTimes(1);
      cleanup?.();
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.doUnmock("@tauri-apps/plugin-updater");
      vi.resetModules();
    }
  });

  it("renders the inventory view by default with seeded counts", () => {
    render(<InventoryShell />);

    expect(screen.getAllByText("ME Inventory")).toHaveLength(1);
    expect(screen.getByText(`v${APP_VERSION}`)).toBeInTheDocument();
    expect(screen.getAllByText(APP_CREDIT).length).toBeGreaterThanOrEqual(1);
    expect(document.title).toBe(APP_DISPLAY_NAME);
    expect(document.title).not.toContain(APP_CREDIT);
    expect(screen.queryByText(/prototype/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Import Data" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Export/i })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Export Excel" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Export HTML" })).not.toBeInTheDocument();
    expect(
      screen.getByPlaceholderText(
        "Search entries by asset, serial, maker, model, description, location, status, or notes",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("Showing all 10 entries")).toBeInTheDocument();
    expect(screen.getByText("Total: 14 | Verified: 8/14")).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /Manufacturer/i })).toBeInTheDocument();
  });

  it("loads entries from the desktop bridge when available", async () => {
    const desktopEntries: InventoryEntry[] = [
      {
        id: "101",
        assetNumber: "ME-101",
        qty: 1,
        manufacturer: "Bridgeport",
        model: "Series I",
        description: "Vertical milling machine",
        projectName: "Fixture rework",
        location: "ME Bay",
        links: "",
        notes: "",
        lifecycleStatus: "active",
        workingStatus: "working",
        verifiedInSurvey: true,
        archived: false,
        updatedAt: "2026-04-23 10:00:00",
      },
      {
        id: "102",
        assetNumber: "ME-102",
        qty: 2,
        manufacturer: "Mitutoyo",
        model: "500-196-30",
        description: "Digital caliper",
        projectName: "Incoming inspection",
        location: "Tool crib",
        links: "",
        notes: "",
        lifecycleStatus: "active",
        workingStatus: "working",
        verifiedInSurvey: false,
        archived: false,
        updatedAt: "2026-04-22 09:00:00",
      },
    ];

    window.inventoryDesktop = {
      isDesktop: true,
      loadInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: desktopEntries,
        shared: CONNECTED_SHARED_STATUS,
      }),
      syncInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: desktopEntries,
        shared: CONNECTED_SHARED_STATUS,
      }),
      toggleVerifiedEntry: vi.fn().mockResolvedValue(desktopEntries[0]),
      createEntry: vi.fn().mockResolvedValue(desktopEntries[0]),
      updateEntry: vi.fn().mockResolvedValue(desktopEntries[0]),
      setArchivedEntry: vi.fn().mockResolvedValue(desktopEntries[0]),
      deleteEntry: vi.fn().mockResolvedValue({ entryId: desktopEntries[0].id }),
      openExternal: vi.fn().mockResolvedValue(true),
      openPath: vi.fn().mockResolvedValue(true),
      pickPicturePath: vi.fn().mockResolvedValue(null),
      exportExcel: vi.fn().mockResolvedValue({ canceled: false, outputPath: "D:/exports/ME_Inventory_Export.xlsx" }),
    };

    render(<InventoryShell />);

    expect(screen.getByText("Loading inventory entries...")).toBeInTheDocument();
    expect(await screen.findByText("Showing all 2 entries")).toBeInTheDocument();
    expect(screen.getByText("Bridgeport")).toBeInTheDocument();
    expect(screen.getByText("Total: 2 | Verified: 1/2")).toBeInTheDocument();
  });

  it("filters desktop search locally without querying the backend per keystroke", async () => {
    const user = userEvent.setup();
    const desktopEntries = [
      buildTestEntry({ id: "801", description: "Bridgeport mill", manufacturer: "Bridgeport" }),
      buildTestEntry({ id: "802", description: "Digital caliper", manufacturer: "Mitutoyo" }),
    ];
    const loadInventory = vi.fn().mockResolvedValue(buildDesktopSyncResult(DISABLED_SHARED_STATUS, desktopEntries));
    const queryInventory = vi.fn().mockResolvedValue(buildDesktopQueryResult(DISABLED_SHARED_STATUS));
    const syncInventory = vi.fn().mockResolvedValue({
      dbPath: TEST_DB_PATH,
      entries: [],
      entriesChanged: false,
      shared: DISABLED_SHARED_STATUS,
    });

    window.inventoryDesktop = createDesktopBridge({ loadInventory, queryInventory, syncInventory });

    render(<InventoryShell />);

    expect(await screen.findByText("Showing all 2 entries")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Inventory search"), "mit");

    await waitFor(() => expect(screen.getByText('1 result for "mit"')).toBeInTheDocument());
    expect(screen.getByText("Mitutoyo")).toBeInTheDocument();
    expect(screen.queryByText("Bridgeport")).not.toBeInTheDocument();
    expect(loadInventory).toHaveBeenCalledTimes(1);
    expect(queryInventory).not.toHaveBeenCalled();
    expect(syncInventory).not.toHaveBeenCalled();
  });

  it("does not keep shared sync polling active when shared sync is disabled", async () => {
    const activeIntervals = new Set<number>();
    let nextIntervalId = 1;
    const setIntervalSpy = vi.spyOn(window, "setInterval").mockImplementation(() => {
      const intervalId = nextIntervalId;
      nextIntervalId += 1;
      activeIntervals.add(intervalId);
      return intervalId as unknown as ReturnType<typeof window.setInterval>;
    });
    const clearIntervalSpy = vi.spyOn(window, "clearInterval").mockImplementation((intervalId) => {
      activeIntervals.delete(Number(intervalId));
    });
    const syncInventory = vi.fn().mockResolvedValue({
      dbPath: TEST_DB_PATH,
      entries: [],
      entriesChanged: false,
      shared: DISABLED_SHARED_STATUS,
    });

    try {
      window.inventoryDesktop = createDesktopBridge({
        loadInventory: vi.fn().mockResolvedValue(buildDesktopSyncResult(DISABLED_SHARED_STATUS)),
        syncInventory,
      });

      render(<InventoryShell />);

      await flushAsyncWork();
      expect(screen.getByText("Showing all 0 entries")).toBeInTheDocument();
      expect(activeIntervals.size).toBe(0);
      expect(syncInventory).not.toHaveBeenCalled();
    } finally {
      setIntervalSpy.mockRestore();
      clearIntervalSpy.mockRestore();
    }
  });

  it("clamps shared sync intervals before scheduling polling", async () => {
    const scheduledIntervals: number[] = [];
    let nextIntervalId = 1;
    const setIntervalSpy = vi.spyOn(window, "setInterval").mockImplementation((_handler, timeout) => {
      scheduledIntervals.push(Number(timeout));
      const intervalId = nextIntervalId;
      nextIntervalId += 1;
      return intervalId as unknown as ReturnType<typeof window.setInterval>;
    });
    const clearIntervalSpy = vi.spyOn(window, "clearInterval").mockImplementation(() => undefined);
    const fastSharedStatus: InventorySharedStatus = {
      ...CONNECTED_SHARED_STATUS,
      syncIntervalMs: 1,
    };

    try {
      window.inventoryDesktop = createDesktopBridge({
        loadInventory: vi.fn().mockResolvedValue(buildDesktopSyncResult(fastSharedStatus)),
        syncInventory: vi.fn().mockResolvedValue({
          dbPath: TEST_DB_PATH,
          entries: [],
          entriesChanged: false,
          shared: fastSharedStatus,
        }),
      });

      render(<InventoryShell />);

      await flushAsyncWork();
      await flushAsyncWork();
      expect(scheduledIntervals).toContain(30_000);
      expect(scheduledIntervals).not.toContain(1);
    } finally {
      setIntervalSpy.mockRestore();
      clearIntervalSpy.mockRestore();
    }
  });

  it("subscribes to shared change events while shared sync polling is enabled", async () => {
    const activeIntervals = new Set<number>();
    let nextIntervalId = 1;
    let sharedChangeCallback: (() => void) | null = null;
    const unsubscribeSharedChanges = vi.fn();
    const setIntervalSpy = vi.spyOn(window, "setInterval").mockImplementation(() => {
      const intervalId = nextIntervalId;
      nextIntervalId += 1;
      activeIntervals.add(intervalId);
      return intervalId as unknown as ReturnType<typeof window.setInterval>;
    });
    const clearIntervalSpy = vi.spyOn(window, "clearInterval").mockImplementation((intervalId) => {
      activeIntervals.delete(Number(intervalId));
    });
    const syncInventory = vi.fn().mockResolvedValue({
      dbPath: TEST_DB_PATH,
      entries: [],
      entriesChanged: false,
      shared: CONNECTED_SHARED_STATUS,
    });
    const onSharedInventoryChanged = vi.fn((callback: () => void) => {
      sharedChangeCallback = callback;
      return unsubscribeSharedChanges;
    });

    try {
      window.inventoryDesktop = createDesktopBridge({
        onSharedInventoryChanged,
        loadInventory: vi.fn().mockResolvedValue(buildDesktopSyncResult(CONNECTED_SHARED_STATUS)),
        syncInventory,
      });

      const { unmount } = render(<InventoryShell />);

      await flushAsyncWork();
      await flushAsyncWork();
      expect(onSharedInventoryChanged).toHaveBeenCalledTimes(1);
      expect(syncInventory).toHaveBeenCalledTimes(1);
      expect(activeIntervals.size).toBe(1);

      syncInventory.mockClear();
      act(() => {
        sharedChangeCallback?.();
      });

      await flushAsyncWork();
      expect(syncInventory).toHaveBeenCalledTimes(1);

      unmount();
      expect(activeIntervals.size).toBe(0);
      expect(unsubscribeSharedChanges).toHaveBeenCalledTimes(1);
    } finally {
      setIntervalSpy.mockRestore();
      clearIntervalSpy.mockRestore();
    }
  });

  it("coalesces quick desktop mutations into one delayed sync", async () => {
    const user = userEvent.setup();
    const entry = buildTestEntry({ description: "Delayed sync entry" });
    const syncInventory = vi.fn().mockResolvedValue({
      dbPath: TEST_DB_PATH,
      entries: [],
      entriesChanged: false,
      shared: CONNECTED_SHARED_STATUS,
    });
    const toggleVerifiedEntry = vi
      .fn()
      .mockResolvedValueOnce({
        entry: { ...entry, verifiedInSurvey: true },
        message: "Verified state updated.",
        mutationMode: "local",
        shared: CONNECTED_SHARED_STATUS,
      })
      .mockResolvedValueOnce({
        entry,
        message: "Verified state updated.",
        mutationMode: "local",
        shared: CONNECTED_SHARED_STATUS,
      });

    window.inventoryDesktop = createDesktopBridge({
      loadInventory: vi.fn().mockResolvedValue(buildDesktopSyncResult(CONNECTED_SHARED_STATUS, [entry])),
      syncInventory,
      toggleVerifiedEntry,
    });

    render(<InventoryShell />);

    expect(await screen.findByText("Delayed sync entry")).toBeInTheDocument();
    await flushAsyncWork();
    syncInventory.mockClear();

    const toggleButton = screen.getByRole("button", { name: /Toggle verified for Delayed sync entry/i });
    await user.click(toggleButton);
    await user.click(toggleButton);

    expect(toggleVerifiedEntry).toHaveBeenCalledTimes(2);
    expect(syncInventory).not.toHaveBeenCalled();

    await delay(700);
    expect(syncInventory).not.toHaveBeenCalled();

    await delay(150);
    expect(syncInventory).toHaveBeenCalledTimes(1);
  });

  it("runs one follow-up sync when a shared change arrives during an in-flight sync", async () => {
    const firstSync = createDeferred<Awaited<ReturnType<NonNullable<Window["inventoryDesktop"]>["syncInventory"]>>>();
    let sharedChangeCallback: (() => void) | null = null;
    const entry = buildTestEntry({ description: "In-flight sync entry" });
    const syncInventory = vi
      .fn()
      .mockReturnValueOnce(firstSync.promise)
      .mockResolvedValue({
        dbPath: TEST_DB_PATH,
        entries: [],
        entriesChanged: false,
        shared: CONNECTED_SHARED_STATUS,
      });

    window.inventoryDesktop = createDesktopBridge({
      onSharedInventoryChanged: vi.fn((callback: () => void) => {
        sharedChangeCallback = callback;
        return () => undefined;
      }),
      loadInventory: vi.fn().mockResolvedValue(buildDesktopSyncResult(CONNECTED_SHARED_STATUS, [entry])),
      syncInventory,
    });

    render(<InventoryShell />);

    expect(await screen.findByText("In-flight sync entry")).toBeInTheDocument();
    await waitFor(() => expect(syncInventory).toHaveBeenCalledTimes(1));

    act(() => {
      sharedChangeCallback?.();
      sharedChangeCallback?.();
    });
    expect(syncInventory).toHaveBeenCalledTimes(1);

    await act(async () => {
      firstSync.resolve({
        dbPath: TEST_DB_PATH,
        entries: [],
        entriesChanged: false,
        shared: CONNECTED_SHARED_STATUS,
      });
      await firstSync.promise;
      await Promise.resolve();
    });

    await waitFor(() => expect(syncInventory).toHaveBeenCalledTimes(2));
  });

  it("cleans up delayed mutation sync timers on unmount", async () => {
    const user = userEvent.setup();
    const entry = buildTestEntry({ description: "Unmount sync entry" });
    const syncInventory = vi.fn().mockResolvedValue({
      dbPath: TEST_DB_PATH,
      entries: [],
      entriesChanged: false,
      shared: CONNECTED_SHARED_STATUS,
    });

    window.inventoryDesktop = createDesktopBridge({
      loadInventory: vi.fn().mockResolvedValue(buildDesktopSyncResult(CONNECTED_SHARED_STATUS, [entry])),
      syncInventory,
      toggleVerifiedEntry: vi.fn().mockResolvedValue({
        entry: { ...entry, verifiedInSurvey: true },
        message: "Verified state updated.",
        mutationMode: "local",
        shared: CONNECTED_SHARED_STATUS,
      }),
    });

    const { unmount } = render(<InventoryShell />);

    expect(await screen.findByText("Unmount sync entry")).toBeInTheDocument();
    await flushAsyncWork();
    syncInventory.mockClear();

    await user.click(screen.getByRole("button", { name: /Toggle verified for Unmount sync entry/i }));
    unmount();

    await delay(850);

    expect(syncInventory).not.toHaveBeenCalled();
  });

  it("does not start initial sync after a desktop load resolves post-unmount", async () => {
    const deferredLoad = createDeferred<InventorySyncResult>();
    const loadInventory = vi.fn(() => deferredLoad.promise);
    const syncInventory = vi.fn().mockResolvedValue({
      dbPath: TEST_DB_PATH,
      entries: [],
      entriesChanged: false,
      shared: CONNECTED_SHARED_STATUS,
    });
    window.inventoryDesktop = createDesktopBridge({ loadInventory, syncInventory });

    const { unmount } = render(<InventoryShell />);
    await waitFor(() => expect(loadInventory).toHaveBeenCalled());

    unmount();
    await act(async () => {
      deferredLoad.resolve(buildDesktopSyncResult(CONNECTED_SHARED_STATUS));
      await deferredLoad.promise;
    });

    expect(syncInventory).not.toHaveBeenCalled();
  });

  it("keeps current rows when desktop sync reports no entry changes", async () => {
    const desktopEntries: InventoryEntry[] = [
      {
        id: "301",
        assetNumber: "ME-301",
        qty: 1,
        manufacturer: "Stable Maker",
        model: "SM-1",
        description: "Stable entry",
        projectName: "Sync",
        location: "Bench",
        links: "",
        notes: "",
        lifecycleStatus: "active",
        workingStatus: "working",
        verifiedInSurvey: true,
        archived: false,
        updatedAt: "2026-04-23 10:00:00",
      },
    ];

    window.inventoryDesktop = {
      isDesktop: true,
      loadInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: desktopEntries,
        shared: CONNECTED_SHARED_STATUS,
      }),
      syncInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: [{ ...desktopEntries[0], manufacturer: "Replacement Maker" }],
        entriesChanged: false,
        shared: CONNECTED_SHARED_STATUS,
      }),
      toggleVerifiedEntry: vi.fn().mockResolvedValue(desktopEntries[0]),
      createEntry: vi.fn().mockResolvedValue(desktopEntries[0]),
      updateEntry: vi.fn().mockResolvedValue(desktopEntries[0]),
      setArchivedEntry: vi.fn().mockResolvedValue(desktopEntries[0]),
      deleteEntry: vi.fn().mockResolvedValue({ entryId: desktopEntries[0].id }),
      openExternal: vi.fn().mockResolvedValue(true),
      openPath: vi.fn().mockResolvedValue(true),
      pickPicturePath: vi.fn().mockResolvedValue(null),
      exportExcel: vi.fn().mockResolvedValue({ canceled: false, outputPath: "D:/exports/ME_Inventory_Export.xlsx" }),
    };

    render(<InventoryShell />);

    expect(await screen.findByText("Stable Maker")).toBeInTheDocument();
    await waitFor(() => expect(window.inventoryDesktop?.syncInventory).toHaveBeenCalled());
    expect(screen.getByText("Stable Maker")).toBeInTheDocument();
    expect(screen.queryByText("Replacement Maker")).not.toBeInTheDocument();
  });

  it("shows the shared update button and transitions through download states", async () => {
    const user = userEvent.setup();
    let updateListener: (state: UpdateState) => void = () => undefined;

    window.inventoryDesktop = {
      isDesktop: true,
      loadInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: [],
        shared: CONNECTED_SHARED_STATUS,
      }),
      syncInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: [],
        shared: CONNECTED_SHARED_STATUS,
      }),
      toggleVerifiedEntry: vi.fn().mockResolvedValue(null),
      createEntry: vi.fn().mockResolvedValue(null),
      updateEntry: vi.fn().mockResolvedValue(null),
      setArchivedEntry: vi.fn().mockResolvedValue(null),
      deleteEntry: vi.fn().mockResolvedValue({ entryId: "0" }),
      openExternal: vi.fn().mockResolvedValue(true),
      openPath: vi.fn().mockResolvedValue(true),
      pickPicturePath: vi.fn().mockResolvedValue(null),
      exportExcel: vi.fn().mockResolvedValue({ canceled: false, outputPath: "D:/exports/ME_Inventory_Export.xlsx" }),
      checkForUpdate: vi.fn().mockResolvedValue({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.8",
        status: "available",
      }),
      downloadUpdate: vi.fn().mockResolvedValue({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.8",
        status: "ready",
      }),
      installUpdate: vi.fn().mockResolvedValue({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.8",
        status: "installing",
      }),
      onUpdateStateChanged: vi.fn((callback) => {
        updateListener = callback;
        return () => undefined;
      }),
    };

    render(<InventoryShell />);

    const updateButton = await screen.findByRole("button", { name: "Update available" });
    expect(updateButton.className).toContain("bg-sky-100");
    expect(updateButton.className).toContain("border-sky-500");
    expect(updateButton.className).toContain("text-sky-700");

    act(() => {
      updateListener({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.8",
        status: "downloading",
      });
    });
    expect(await screen.findByRole("button", { name: "Downloading update..." })).toBeDisabled();

    act(() => {
      updateListener({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.8",
        status: "ready",
      });
    });

    await user.click(await screen.findByRole("button", { name: "Install update" }));
    expect(window.inventoryDesktop?.installUpdate).toHaveBeenCalledTimes(1);
    expect(await screen.findByRole("button", { name: "Installer opened" })).toBeDisabled();
    expect(window.inventoryDesktop?.downloadUpdate).not.toHaveBeenCalled();
  });

  it("switches to archive view and updates the summary", async () => {
    const user = userEvent.setup();
    render(<InventoryShell />);

    await user.click(screen.getAllByRole("button", { name: /Archive/i })[0]);

    expect(screen.getByText("Showing all 4 archived entries")).toBeInTheDocument();
    expect(
      screen.getByPlaceholderText("Search archived entries by asset, serial, maker, model, description, location, or notes"),
    ).toBeInTheDocument();
    expect(screen.getByText("Cabinet table saw")).toBeInTheDocument();
  });

  it("shows and clears the filter panel", async () => {
    const user = userEvent.setup();
    render(<InventoryShell />);

    await user.click(screen.getByRole("button", { name: "Filters" }));
    const manufacturerFilter = screen.getByLabelText("Filter manufacturer");
    await user.type(manufacturerFilter, "Mitutoyo");

    expect(screen.getByText("Showing 1 filtered entries")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Clear Column Filters" }));
    expect(screen.getByText("Showing all 10 entries")).toBeInTheDocument();
  });

  it("shows the inventory empty-state CTA for unmatched searches", async () => {
    const user = userEvent.setup();
    render(<InventoryShell />);

    await user.type(screen.getByLabelText("Inventory search"), "no-match-value");

    expect(screen.getByText('No results for "no-match-value"')).toBeInTheDocument();
    expect(
      screen.getByText("Try a broader search, clear the column filters, or add a new entry."),
    ).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Add Entry" }).length).toBeGreaterThan(0);
  });

  it("updates theme preference and shows mock verified feedback", async () => {
    const user = userEvent.setup();
    render(<InventoryShell />);

    await user.click(screen.getAllByRole("button", { name: /Dark/i })[0]);
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(localStorage.getItem("meInventory.theme")).toBe("dark");

    await user.click(screen.getByRole("button", { name: /Toggle verified for Stainless socket-head cap screws/i }));
    expect(screen.getByText("Verified state updated locally.")).toBeInTheDocument();
  });

  it("shows the HTML export placeholder message", async () => {
    const user = userEvent.setup();
    render(<InventoryShell />);

    await user.click(screen.getByRole("button", { name: /Export/i }));
    await user.click(screen.getByRole("menuitem", { name: "HTML" }));

    expect(screen.getByText("HTML export is not implemented yet.")).toBeInTheDocument();
  });

  it("runs desktop Excel export when available", async () => {
    const user = userEvent.setup();
    const exportExcel = vi.fn().mockResolvedValue({
      canceled: false,
      outputPath: "D:/exports/ME_Inventory_Export.xlsx",
    });

    window.inventoryDesktop = {
      isDesktop: true,
      loadInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: [],
        shared: CONNECTED_SHARED_STATUS,
      }),
      syncInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: [],
        shared: CONNECTED_SHARED_STATUS,
      }),
      toggleVerifiedEntry: vi.fn().mockResolvedValue(null),
      createEntry: vi.fn().mockResolvedValue(null),
      updateEntry: vi.fn().mockResolvedValue(null),
      setArchivedEntry: vi.fn().mockResolvedValue(null),
      deleteEntry: vi.fn().mockResolvedValue({ entryId: "0" }),
      openExternal: vi.fn().mockResolvedValue(true),
      openPath: vi.fn().mockResolvedValue(true),
      pickPicturePath: vi.fn().mockResolvedValue(null),
      exportExcel,
    };

    render(<InventoryShell />);

    await user.click(screen.getByRole("button", { name: /Export/i }));
    await user.click(screen.getByRole("menuitem", { name: "Excel" }));

    expect(exportExcel).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("Excel export completed.")).toBeInTheDocument();
  });

  it("keeps desktop editing enabled when mutations are local-only", async () => {
    const user = userEvent.setup();
    let desktopEntries: InventoryEntry[] = [
      {
        id: "201",
        assetNumber: "ME-201",
        qty: 1,
        manufacturer: "Offline Maker",
        model: "OM-1",
        description: "Offline editable entry",
        projectName: "Local Work",
        location: "Bench 1",
        links: "",
        notes: "",
        lifecycleStatus: "active",
        workingStatus: "working",
        verifiedInSurvey: false,
        archived: false,
        updatedAt: "2026-04-23 10:00:00",
      },
    ];
    const createdEntry: InventoryEntry = {
      ...desktopEntries[0],
      id: "202",
      assetNumber: "",
      description: "Local-only saved entry",
      manufacturer: "Local Maker",
      verifiedInSurvey: false,
    };
    const createEntry = vi.fn().mockImplementation(async () => {
      desktopEntries = [createdEntry, ...desktopEntries];
      return {
        entry: createdEntry,
        message: "Entry added locally.",
        mutationMode: "local",
        shared: LOCAL_SHARED_STATUS,
      };
    });

    window.inventoryDesktop = {
      isDesktop: true,
      loadInventory: vi.fn().mockImplementation(async () => ({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: desktopEntries,
        shared: LOCAL_SHARED_STATUS,
      })),
      syncInventory: vi.fn().mockImplementation(async () => ({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: desktopEntries,
        shared: LOCAL_SHARED_STATUS,
      })),
      toggleVerifiedEntry: vi.fn().mockImplementation(async (entryId: string, nextVerified: boolean) => {
        const updatedEntry = { ...desktopEntries.find((entry) => entry.id === entryId)!, verifiedInSurvey: nextVerified };
        desktopEntries = desktopEntries.map((entry) => (entry.id === entryId ? updatedEntry : entry));
        return {
          entry: updatedEntry,
          message: "Verified state updated locally.",
          mutationMode: "local",
          shared: LOCAL_SHARED_STATUS,
        };
      }),
      createEntry,
      updateEntry: vi.fn().mockResolvedValue(desktopEntries[0]),
      setArchivedEntry: vi.fn().mockResolvedValue(desktopEntries[0]),
      deleteEntry: vi.fn().mockResolvedValue({ entryId: desktopEntries[0].id }),
      openExternal: vi.fn().mockResolvedValue(true),
      openPath: vi.fn().mockResolvedValue(true),
      pickPicturePath: vi.fn().mockResolvedValue(null),
      exportExcel: vi.fn().mockResolvedValue({ canceled: false, outputPath: "D:/exports/ME_Inventory_Export.xlsx" }),
    };

    render(<InventoryShell />);

    expect(await screen.findByText(/Shared workspace unavailable\. Saving changes locally\./)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Add Entry" })).toBeEnabled();

    await user.click(screen.getByRole("button", { name: "Add Entry" }));
    const manufacturerInput = screen.getByLabelText("Manufacturer / Brand");
    await user.type(manufacturerInput, "Local Maker");
    await user.type(screen.getByLabelText("Description"), "Local-only saved entry");

    expect(manufacturerInput).toHaveValue("Local Maker");

    await user.click(screen.getByRole("button", { name: "Save Entry" }));

    expect(createEntry).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("Entry added locally.")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Toggle verified for Local-only saved entry/i }));
    expect(await screen.findByText("Verified state updated locally.")).toBeInTheDocument();
  });

  it("uses the mutation result message when shared status changes during a save", async () => {
    const user = userEvent.setup();
    const existingEntry: InventoryEntry = {
      id: "601",
      assetNumber: "ME-601",
      qty: 1,
      manufacturer: "Connected Maker",
      model: "CM-1",
      description: "Connected entry",
      projectName: "Shared",
      location: "Bench 1",
      links: "",
      notes: "",
      lifecycleStatus: "active",
      workingStatus: "working",
      verifiedInSurvey: false,
      archived: false,
      updatedAt: "2026-04-23 10:00:00",
    };
    const createdEntry: InventoryEntry = {
      ...existingEntry,
      id: "602",
      description: "Saved while shared vanished",
      manufacturer: "Local Maker",
    };

    window.inventoryDesktop = {
      isDesktop: true,
      loadInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: [existingEntry],
        shared: CONNECTED_SHARED_STATUS,
      }),
      syncInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: [existingEntry],
        entriesChanged: false,
        shared: CONNECTED_SHARED_STATUS,
      }),
      toggleVerifiedEntry: vi.fn(),
      createEntry: vi.fn().mockResolvedValue({
        entry: createdEntry,
        message: "Entry added locally.",
        mutationMode: "local",
        shared: LOCAL_SHARED_STATUS,
      }),
      updateEntry: vi.fn(),
      setArchivedEntry: vi.fn(),
      deleteEntry: vi.fn(),
      openExternal: vi.fn().mockResolvedValue(true),
      openPath: vi.fn().mockResolvedValue(true),
      pickPicturePath: vi.fn().mockResolvedValue(null),
      exportExcel: vi.fn().mockResolvedValue({ canceled: false, outputPath: "D:/exports/ME_Inventory_Export.xlsx" }),
    };

    render(<InventoryShell />);

    expect(await screen.findByText("Connected Maker")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Add Entry" }));
    await user.type(screen.getByLabelText("Manufacturer / Brand"), "Local Maker");
    await user.type(screen.getByLabelText("Description"), "Saved while shared vanished");
    await user.click(screen.getByRole("button", { name: "Save Entry" }));

    expect(await screen.findByText("Entry added locally.")).toBeInTheDocument();
    expect(screen.queryByText("Entry added to the ME Inventory database.")).not.toBeInTheDocument();
  });
});

function buildTestEntry(overrides: Partial<InventoryEntry> = {}): InventoryEntry {
  return {
    id: "701",
    assetNumber: "ME-701",
    qty: 1,
    manufacturer: "Sync Maker",
    model: "SM-701",
    description: "Sync test entry",
    projectName: "Shared",
    location: "Bench 1",
    links: "",
    notes: "",
    lifecycleStatus: "active",
    workingStatus: "working",
    verifiedInSurvey: false,
    archived: false,
    updatedAt: "2026-04-26T10:00:00.000Z",
    ...overrides,
  };
}

function buildDesktopQueryResult(shared: InventorySharedStatus, entries: InventoryEntry[] = []): InventoryQueryResult {
  return {
    counts: entries.length === 0 ? EMPTY_TEST_COUNTS : buildInventoryCounts(entries),
    dbPath: TEST_DB_PATH,
    entries,
    shared,
    totalFiltered: entries.length,
  };
}

function buildDesktopSyncResult(shared: InventorySharedStatus, entries: InventoryEntry[] = []): InventorySyncResult {
  return {
    dbPath: TEST_DB_PATH,
    entries,
    shared,
  };
}

function buildInventoryCounts(entries: InventoryEntry[]): InventoryCounts {
  let archive = 0;
  let verified = 0;

  for (const entry of entries) {
    if (entry.archived) {
      archive += 1;
    }
    if (entry.verifiedInSurvey) {
      verified += 1;
    }
  }

  return {
    archive,
    inventory: entries.length - archive,
    total: entries.length,
    verified,
  };
}

function createDesktopBridge(
  overrides: Partial<NonNullable<Window["inventoryDesktop"]>>,
): NonNullable<Window["inventoryDesktop"]> {
  return {
    isDesktop: true,
    loadInventory: vi.fn().mockResolvedValue({
      dbPath: TEST_DB_PATH,
      entries: [],
      shared: CONNECTED_SHARED_STATUS,
    }),
    queryInventory: vi.fn().mockResolvedValue(buildDesktopQueryResult(CONNECTED_SHARED_STATUS)),
    syncInventory: vi.fn().mockResolvedValue({
      dbPath: TEST_DB_PATH,
      entries: [],
      entriesChanged: false,
      shared: CONNECTED_SHARED_STATUS,
    }),
    toggleVerifiedEntry: vi.fn(),
    createEntry: vi.fn(),
    updateEntry: vi.fn(),
    setArchivedEntry: vi.fn(),
    deleteEntry: vi.fn(),
    openExternal: vi.fn().mockResolvedValue(true),
    openPath: vi.fn().mockResolvedValue(true),
    pickPicturePath: vi.fn().mockResolvedValue(null),
    exportExcel: vi.fn().mockResolvedValue({ canceled: false, outputPath: "D:/exports/ME_Inventory_Export.xlsx" }),
    ...overrides,
  } as NonNullable<Window["inventoryDesktop"]>;
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

async function delay(ms: number): Promise<void> {
  await act(async () => {
    await new Promise((resolve) => window.setTimeout(resolve, ms));
  });
}
