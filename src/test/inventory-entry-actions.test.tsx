import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { InventoryShell } from "@/components/inventory/InventoryShell";

describe("InventoryShell entry actions", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
    delete window.inventoryDesktop;
    vi.restoreAllMocks();
    mockMatchMedia(false);
  });

  it("adds a new entry from the toolbar dialog", async () => {
    const user = userEvent.setup();
    render(<InventoryShell />);

    await user.click(screen.getByRole("button", { name: "Add Entry" }));
    await user.type(screen.getByLabelText("Manufacturer / Brand"), "Acme Tooling");
    await user.type(screen.getByLabelText("Description"), "Laser-cut fixture plate");
    await user.type(screen.getByLabelText("Location"), "Shelf Z9");

    await user.click(screen.getByRole("button", { name: "Save Entry" }));

    expect(await screen.findByText("Laser-cut fixture plate")).toBeInTheDocument();
    expect(screen.getByText("Entry added locally.")).toBeInTheDocument();
    expect(screen.getByText("Showing all 11 entries")).toBeInTheDocument();
  });

  it("opens the editor on row double click and updates the entry", async () => {
    const user = userEvent.setup();
    render(<InventoryShell />);

    await user.dblClick(screen.getByText("Stainless socket-head cap screws, 1/4-20"));

    const locationInput = screen.getByLabelText("Location");
    await user.clear(locationInput);
    await user.type(locationInput, "Shelf Z9");
    await user.click(screen.getByRole("button", { name: "Save Entry" }));

    expect(await screen.findByText("Shelf Z9")).toBeInTheDocument();
    expect(screen.getByText("Entry updated locally.")).toBeInTheDocument();
  });

  it("keeps the edit actions in the sidebar on large viewports", async () => {
    const user = userEvent.setup();
    mockMatchMedia(true);
    render(<InventoryShell />);

    await user.dblClick(screen.getByText("Stainless socket-head cap screws, 1/4-20"));

    expect(screen.getByText("Database Metadata")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Save Entry" })).toHaveLength(1);
    expect(screen.getAllByRole("button", { name: "Cancel" })).toHaveLength(1);

    const locationInput = screen.getByLabelText("Location");
    await user.clear(locationInput);
    await user.type(locationInput, "Shelf Y4");
    await user.click(screen.getByRole("button", { name: "Save Entry" }));

    expect(await screen.findByText("Shelf Y4")).toBeInTheDocument();
  });

  it("archives an entry from the right-click menu", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);

    render(<InventoryShell />);

    fireEvent.contextMenu(screen.getByText("Industrial multimeter"));
    expect(screen.queryByText("Entry Actions")).not.toBeInTheDocument();
    expect(screen.getAllByText("Industrial multimeter")).toHaveLength(1);
    await user.click(await screen.findByRole("button", { name: "Archive Entry" }));

    expect(screen.queryByText("Industrial multimeter")).not.toBeInTheDocument();
    expect(screen.getByText("Entry moved to the archive.")).toBeInTheDocument();
    expect(confirm).not.toHaveBeenCalled();

    await user.click(screen.getAllByRole("button", { name: /Archive/i })[0]);
    expect(await screen.findByText("Industrial multimeter")).toBeInTheDocument();
  });

  it("restores an entry from the right-click menu without a browser prompt", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);

    render(<InventoryShell />);

    await user.click(screen.getAllByRole("button", { name: /Archive/i })[0]);
    fireEvent.contextMenu(screen.getByText("Cabinet table saw"));
    await user.click(await screen.findByRole("button", { name: "Restore Entry" }));

    expect(screen.queryByText("Cabinet table saw")).not.toBeInTheDocument();
    expect(screen.getByText("Entry restored to inventory.")).toBeInTheDocument();
    expect(confirm).not.toHaveBeenCalled();
  });

  it("opens a styled delete dialog and cancels without deleting", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);

    render(<InventoryShell />);

    fireEvent.contextMenu(screen.getByText("Industrial multimeter"));
    await user.click(await screen.findByRole("button", { name: "Delete Entry" }));

    expect(screen.getByText("Delete this entry?")).toBeInTheDocument();
    expect(screen.getAllByText("Industrial multimeter").length).toBeGreaterThan(0);
    expect(confirm).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByText("Delete this entry?")).not.toBeInTheDocument();
    expect(screen.getByText("Industrial multimeter")).toBeInTheDocument();
  });

  it("deletes an entry only after confirming in the app dialog", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);

    render(<InventoryShell />);

    fireEvent.contextMenu(screen.getByText("Industrial multimeter"));
    await user.click(await screen.findByRole("button", { name: "Delete Entry" }));
    await user.click(screen.getByRole("button", { name: "Delete Entry" }));

    expect(screen.queryByText("Industrial multimeter")).not.toBeInTheDocument();
    expect(screen.getByText("Entry deleted.")).toBeInTheDocument();
    expect(confirm).not.toHaveBeenCalled();
  });

  it("hides the saved-link action when a row has no link", () => {
    render(<InventoryShell />);

    fireEvent.contextMenu(screen.getByText("Long handle ratchet"));

    expect(screen.queryByRole("button", { name: "Open Saved Link" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Search Online" })).toBeInTheDocument();
  });

  it("shows the saved-link action when a row has a link", () => {
    render(<InventoryShell />);

    fireEvent.contextMenu(screen.getByText("Industrial multimeter"));

    expect(screen.getByRole("button", { name: "Open Saved Link" })).toBeInTheDocument();
  });

  it("does not open unsafe saved-link schemes", async () => {
    const user = userEvent.setup();
    const openExternal = vi.fn().mockResolvedValue(true);
    const unsafeEntry = {
      archived: false,
      assetNumber: "ME-UNSAFE",
      description: "Unsafe saved link entry",
      id: "501",
      links: "javascript:alert(1)",
      lifecycleStatus: "active" as const,
      location: "Bench",
      manufacturer: "Acme",
      model: "Unsafe",
      notes: "",
      projectName: "Security",
      qty: 1,
      updatedAt: "2026-04-25T12:00:00.000Z",
      verifiedInSurvey: false,
      workingStatus: "working" as const,
    };
    window.inventoryDesktop = {
      isDesktop: true,
      loadInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: [unsafeEntry],
        shared: {
          available: true,
          canModify: true,
          enabled: true,
          message: "",
          mutationMode: "shared",
        },
      }),
      syncInventory: vi.fn().mockResolvedValue({
        dbPath: "D:/coding/ME Inventory/data/me_inventory.db",
        entries: [unsafeEntry],
        shared: {
          available: true,
          canModify: true,
          enabled: true,
          message: "",
          mutationMode: "shared",
        },
      }),
      toggleVerifiedEntry: vi.fn(),
      createEntry: vi.fn(),
      updateEntry: vi.fn(),
      setArchivedEntry: vi.fn(),
      deleteEntry: vi.fn(),
      openExternal,
      openPath: vi.fn().mockResolvedValue(true),
      pickPicturePath: vi.fn().mockResolvedValue(null),
      exportExcel: vi.fn().mockResolvedValue({ canceled: false, outputPath: "D:/exports/ME_Inventory_Export.xlsx" }),
    };

    render(<InventoryShell />);

    fireEvent.contextMenu(await screen.findByText("Unsafe saved link entry"));
    await user.click(await screen.findByRole("button", { name: "Open Saved Link" }));

    expect(openExternal).not.toHaveBeenCalled();
    expect(screen.getByText("This link is not in a valid format.")).toBeInTheDocument();
  });
});

function mockMatchMedia(matches: boolean) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      addEventListener: vi.fn(),
      addListener: vi.fn(),
      dispatchEvent: vi.fn(),
      matches,
      media: query,
      onchange: null,
      removeEventListener: vi.fn(),
      removeListener: vi.fn(),
    })),
    writable: true,
  });
}
