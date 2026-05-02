import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { APP_CREDIT, APP_DISPLAY_NAME, APP_VERSION } from "@/app/branding";
import { InventoryShell } from "@/features/inventory/components/InventoryShell";
import type { InventoryEntry } from "@/features/inventory/types";
import {
  CONNECTED_SHARED_STATUS,
  DISABLED_SHARED_STATUS,
  TEST_DB_PATH,
  buildDesktopQueryResult,
  buildDesktopSyncResult,
  buildTestEntry,
  createDesktopBridge,
} from "./inventory-shell/helpers";

describe("InventoryShell loading and search", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
    delete window.inventoryDesktop;
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
        dbPath: TEST_DB_PATH,
        entries: desktopEntries,
        shared: CONNECTED_SHARED_STATUS,
      }),
      syncInventory: vi.fn().mockResolvedValue({
        dbPath: TEST_DB_PATH,
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

  it("does not show bundled mock data when desktop database loading fails", async () => {
    window.inventoryDesktop = createDesktopBridge({
      loadInventory: vi.fn().mockRejectedValue(new Error("database unavailable")),
      syncInventory: vi.fn(),
    });

    render(<InventoryShell />);

    expect(screen.getByText("Loading inventory entries...")).toBeInTheDocument();
    expect(await screen.findByText("Showing all 0 entries")).toBeInTheDocument();
    expect(screen.queryByText("Stainless socket-head cap screws")).not.toBeInTheDocument();
    expect(screen.getByText("Could not load the ME Inventory database.")).toBeInTheDocument();
  });

  it("fails closed when bridge parsing rejects a malformed desktop payload", async () => {
    window.inventoryDesktop = createDesktopBridge({
      loadInventory: vi.fn().mockRejectedValue(new Error("Invalid inventory entry: missing id.")),
      syncInventory: vi.fn(),
    });

    render(<InventoryShell />);

    expect(await screen.findByText("Showing all 0 entries")).toBeInTheDocument();
    expect(screen.queryByText("Stainless socket-head cap screws")).not.toBeInTheDocument();
    expect(screen.getByText("Could not load the ME Inventory database.")).toBeInTheDocument();
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
});
