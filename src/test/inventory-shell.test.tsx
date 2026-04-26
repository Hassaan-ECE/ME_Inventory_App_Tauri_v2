import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { APP_DISPLAY_NAME, APP_VERSION } from "@/branding";
import { InventoryShell } from "@/components/inventory/InventoryShell";
import type { InventoryEntry, InventorySharedStatus, UpdateState } from "@/types/inventory";

const CONNECTED_SHARED_STATUS: InventorySharedStatus = {
  available: true,
  canModify: true,
  enabled: true,
  message: "",
  mutationMode: "shared",
  syncIntervalMs: 10_000,
};
const LOCAL_SHARED_STATUS: InventorySharedStatus = {
  available: false,
  canModify: true,
  enabled: true,
  hasLocalOnlyChanges: true,
  message: "Shared workspace unavailable. Saving changes locally.",
  mutationMode: "local",
  syncIntervalMs: 10_000,
};

describe("InventoryShell shell", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
    delete window.inventoryDesktop;
  });

  it("renders the inventory view by default with seeded counts", () => {
    render(<InventoryShell />);

    expect(screen.getAllByText("ME Inventory")).toHaveLength(1);
    expect(screen.getByText(`v${APP_VERSION}`)).toBeInTheDocument();
    expect(document.title).toBe(APP_DISPLAY_NAME);
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
        latestVersion: "0.9.6",
        status: "available",
      }),
      downloadUpdate: vi.fn().mockResolvedValue({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.6",
        status: "ready",
      }),
      installUpdate: vi.fn().mockResolvedValue({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.6",
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
        latestVersion: "0.9.6",
        status: "downloading",
      });
    });
    expect(await screen.findByRole("button", { name: "Downloading update..." })).toBeDisabled();

    act(() => {
      updateListener({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.6",
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
