import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { InventoryShell } from "@/features/inventory/components/InventoryShell";
import type { InventoryEntry, InventorySharedStatus } from "@/features/inventory/types";
import type { InventorySyncResult } from "@/integrations/tauri/desktop-bridge";
import {
  CONNECTED_SHARED_STATUS,
  DISABLED_SHARED_STATUS,
  TEST_DB_PATH,
  buildDesktopSyncResult,
  buildTestEntry,
  createDeferred,
  createDesktopBridge,
  delay,
  flushAsyncWork,
} from "./inventory-shell/helpers";

describe("InventoryShell shared sync", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
    delete window.inventoryDesktop;
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
      expect(scheduledIntervals).toContain(500);
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

  it("keeps startup loading until the initial shared sync finishes", async () => {
    const staleLocalEntry = buildTestEntry({
      id: "901",
      description: "Stale local startup entry",
      manufacturer: "Local Only Maker",
    });
    const syncedEntry = buildTestEntry({
      id: "902",
      description: "Shared startup entry",
      manufacturer: "Shared Maker",
    });
    const initialSync = createDeferred<InventorySyncResult>();
    const loadInventory = vi.fn().mockResolvedValue(buildDesktopSyncResult(CONNECTED_SHARED_STATUS, [staleLocalEntry]));
    const syncInventory = vi.fn().mockReturnValue(initialSync.promise);

    window.inventoryDesktop = createDesktopBridge({ loadInventory, syncInventory });

    render(<InventoryShell />);

    await waitFor(() => expect(syncInventory).toHaveBeenCalledTimes(1));
    expect(screen.getByText("Loading inventory entries...")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Inventory (0)" })).toBeInTheDocument();
    expect(screen.queryByText("Local Only Maker")).not.toBeInTheDocument();

    await act(async () => {
      initialSync.resolve({
        dbPath: TEST_DB_PATH,
        entries: [syncedEntry],
        entriesChanged: true,
        shared: CONNECTED_SHARED_STATUS,
      });
      await initialSync.promise;
      await Promise.resolve();
    });

    expect(await screen.findByText("Shared Maker")).toBeInTheDocument();
    expect(screen.queryByText("Local Only Maker")).not.toBeInTheDocument();
  });

  it("pushes desktop mutations to shared sync almost immediately", async () => {
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

    await waitFor(() => expect(syncInventory).toHaveBeenCalledTimes(1));
    await delay(100);
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

    await waitFor(() => expect(syncInventory).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(sharedChangeCallback).not.toBeNull());
    expect(screen.getByText("Loading inventory entries...")).toBeInTheDocument();
    expect(screen.queryByText("In-flight sync entry")).not.toBeInTheDocument();

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

    expect(await screen.findByText("In-flight sync entry")).toBeInTheDocument();
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

    await delay(50);

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
        dbPath: TEST_DB_PATH,
        entries: desktopEntries,
        shared: CONNECTED_SHARED_STATUS,
      }),
      syncInventory: vi.fn().mockResolvedValue({
        dbPath: TEST_DB_PATH,
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
});
