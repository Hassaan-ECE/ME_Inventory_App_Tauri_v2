import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { APP_VERSION } from "@/app/branding";
import { InventoryShell } from "@/features/inventory/components/InventoryShell";
import type { UpdateState } from "@/features/inventory/types";
import {
  CONNECTED_SHARED_STATUS,
  TEST_DB_PATH,
} from "./inventory-shell/helpers";

describe("InventoryShell updates", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
    delete window.inventoryDesktop;
  });

  it("shows the shared update button and transitions through download states", async () => {
    const user = userEvent.setup();
    let updateListener: (state: UpdateState) => void = () => undefined;

    window.inventoryDesktop = {
      isDesktop: true,
      loadInventory: vi.fn().mockResolvedValue({
        dbPath: TEST_DB_PATH,
        entries: [],
        shared: CONNECTED_SHARED_STATUS,
      }),
      syncInventory: vi.fn().mockResolvedValue({
        dbPath: TEST_DB_PATH,
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

    const updateButton = await screen.findByRole("button", { name: "Update 0.9.8" });
    expect(updateButton.className).toContain("bg-sky-100");
    expect(updateButton.className).toContain("border-sky-500");
    expect(updateButton.className).toContain("text-sky-700");

    act(() => {
      updateListener({
        available: true,
        currentVersion: APP_VERSION,
        downloadProgress: 43,
        latestVersion: "0.9.8",
        status: "downloading",
      });
    });
    expect(await screen.findByRole("button", { name: "Downloading 43%" })).toBeDisabled();

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
    expect(await screen.findByRole("button", { name: "Installing update..." })).toBeDisabled();
    expect(window.inventoryDesktop?.downloadUpdate).not.toHaveBeenCalled();
  });

  it("checks for desktop updates again while the app stays open", async () => {
    const checkForUpdate = vi
      .fn()
      .mockResolvedValueOnce({
        available: false,
        currentVersion: APP_VERSION,
        status: "not-available",
      })
      .mockResolvedValueOnce({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.9",
        status: "available",
      });

    window.inventoryDesktop = {
      isDesktop: true,
      loadInventory: vi.fn().mockResolvedValue({
        dbPath: TEST_DB_PATH,
        entries: [],
        shared: CONNECTED_SHARED_STATUS,
      }),
      syncInventory: vi.fn().mockResolvedValue({
        dbPath: TEST_DB_PATH,
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
      checkForUpdate,
      downloadUpdate: vi.fn().mockResolvedValue({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.9",
        status: "ready",
      }),
      installUpdate: vi.fn().mockResolvedValue({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.9",
        status: "installing",
      }),
    };

    render(<InventoryShell />);

    await waitFor(() => expect(checkForUpdate).toHaveBeenCalledTimes(1));

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });

    await waitFor(() => expect(checkForUpdate).toHaveBeenCalledTimes(2));
    expect(await screen.findByRole("button", { name: "Update 0.9.9" })).toBeInTheDocument();
  });
});
