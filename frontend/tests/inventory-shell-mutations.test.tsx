import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { InventoryShell } from "@/features/inventory/components/InventoryShell";
import type { InventoryEntry } from "@/features/inventory/types";
import {
  CONNECTED_SHARED_STATUS,
  LOCAL_SHARED_STATUS,
  TEST_DB_PATH,
} from "./inventory-shell/helpers";

describe("InventoryShell mutations", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
    delete window.inventoryDesktop;
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
        dbPath: TEST_DB_PATH,
        entries: desktopEntries,
        shared: LOCAL_SHARED_STATUS,
      })),
      syncInventory: vi.fn().mockImplementation(async () => ({
        dbPath: TEST_DB_PATH,
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
        dbPath: TEST_DB_PATH,
        entries: [existingEntry],
        shared: CONNECTED_SHARED_STATUS,
      }),
      syncInventory: vi.fn().mockResolvedValue({
        dbPath: TEST_DB_PATH,
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
