import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { InventoryShell } from "@/features/inventory/components/InventoryShell";
import { InventoryTable } from "@/features/inventory/components/InventoryTable";
import { OVERSCAN_ROWS, ROW_HEIGHT, getVisibleRange } from "@/features/inventory/components/table/virtualization";
import { INVENTORY_COLUMNS, type InventoryEntry } from "@/features/inventory/types";

describe("InventoryShell table controls", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
  });

  it("renders compact labels for long links", () => {
    render(<InventoryShell />);

    expect(screen.getByText("www.cejn.com/en-us/products/thermal-control")).toBeInTheDocument();
  });

  it("renders unsafe link values as inert text instead of anchors", () => {
    const unsafeEntry: InventoryEntry = {
      archived: false,
      assetNumber: "ME-UNSAFE",
      description: "Unsafe link entry",
      id: "unsafe-1",
      links: "javascript:alert(1)",
      lifecycleStatus: "active",
      location: "Bench",
      manufacturer: "Acme",
      model: "Unsafe",
      notes: "",
      projectName: "Security",
      qty: 1,
      updatedAt: "2026-04-25T12:00:00.000Z",
      verifiedInSurvey: false,
      workingStatus: "working",
    };

    render(
      <InventoryTable
        canModifyEntries
        colorRows={false}
        columns={INVENTORY_COLUMNS}
        entries={[unsafeEntry]}
        sortState={{ column: "manufacturer", direction: "asc" }}
        onOpenContextMenu={() => undefined}
        onOpenEntry={() => undefined}
        onOpenExternalLink={() => undefined}
        onSortChange={() => undefined}
        onToggleVerified={() => undefined}
      />,
    );

    expect(screen.getByText("javascript:alert(1)")).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "javascript:alert(1)" })).not.toBeInTheDocument();
  });

  it("routes safe table links through the supplied native-open handler", async () => {
    const user = userEvent.setup();
    const onOpenExternalLink = vi.fn();
    const entry: InventoryEntry = {
      archived: false,
      assetNumber: "ME-LINK",
      description: "Safe link entry",
      id: "link-1",
      links: "https://example.com/item",
      lifecycleStatus: "active",
      location: "Bench",
      manufacturer: "Acme",
      model: "Link",
      notes: "",
      projectName: "Security",
      qty: 1,
      updatedAt: "2026-04-25T12:00:00.000Z",
      verifiedInSurvey: false,
      workingStatus: "working",
    };

    render(
      <InventoryTable
        canModifyEntries
        colorRows={false}
        columns={INVENTORY_COLUMNS}
        entries={[entry]}
        sortState={{ column: "manufacturer", direction: "asc" }}
        onOpenContextMenu={() => undefined}
        onOpenEntry={() => undefined}
        onOpenExternalLink={onOpenExternalLink}
        onSortChange={() => undefined}
        onToggleVerified={() => undefined}
      />,
    );

    await user.click(screen.getByRole("link", { name: "example.com/item" }));

    expect(onOpenExternalLink).toHaveBeenCalledWith("https://example.com/item");
  });

  it("keeps row actions and verified controls wired through extracted table rows", async () => {
    const user = userEvent.setup();
    const onOpenContextMenu = vi.fn();
    const onOpenEntry = vi.fn();
    const onToggleVerified = vi.fn();
    const entry = buildEntry({
      description: "Row actions entry",
      id: "row-actions-1",
      links: "https://example.com/item",
    });

    render(
      <InventoryTable
        canModifyEntries
        colorRows={false}
        columns={INVENTORY_COLUMNS}
        entries={[entry]}
        sortState={{ column: "manufacturer", direction: "asc" }}
        onOpenContextMenu={onOpenContextMenu}
        onOpenEntry={onOpenEntry}
        onOpenExternalLink={() => undefined}
        onSortChange={() => undefined}
        onToggleVerified={onToggleVerified}
      />,
    );

    const row = screen.getByText("Row actions entry").closest("tr");

    await user.dblClick(screen.getByText("Row actions entry"));
    expect(onOpenEntry).toHaveBeenCalledWith("row-actions-1");

    expect(row).not.toBeNull();
    fireEvent.contextMenu(row as HTMLTableRowElement, { clientX: 42, clientY: 84 });
    expect(onOpenContextMenu).toHaveBeenCalledWith("row-actions-1", 42, 84);

    const verifiedButton = screen.getByRole("button", { name: "Toggle verified for Row actions entry" });
    await user.click(verifiedButton);
    expect(onToggleVerified).toHaveBeenCalledWith("row-actions-1");

    await user.dblClick(verifiedButton);
    expect(onOpenEntry).toHaveBeenCalledTimes(1);
  });

  it("calculates virtualized ranges with the table row height and overscan window", () => {
    const viewportHeight = ROW_HEIGHT * 5;

    expect(getVisibleRange(100, ROW_HEIGHT * 20, viewportHeight)).toEqual({
      end: 20 + 5 + OVERSCAN_ROWS,
      start: 20 - OVERSCAN_ROWS,
    });
    expect(getVisibleRange(100, ROW_HEIGHT * 500, viewportHeight)).toEqual({
      end: 100,
      start: 95 - OVERSCAN_ROWS,
    });
  });

  it("hides a selected column from the table", async () => {
    const user = userEvent.setup();
    render(<InventoryShell />);

    await user.click(screen.getByRole("button", { name: /Columns/i }));
    await user.click(screen.getByRole("checkbox", { name: "Links" }));

    expect(screen.queryByRole("columnheader", { name: /Links/i })).not.toBeInTheDocument();
  });

  it("shows a selected style on the color rows toggle and still toggles row colors", async () => {
    const user = userEvent.setup();
    render(<InventoryShell />);

    const colorRowsToggle = screen.getByRole("button", { name: "Color rows" });
    const firstRow = screen.getByText("Stainless socket-head cap screws, 1/4-20").closest("tr");

    expect(colorRowsToggle).toHaveAttribute("aria-pressed", "true");
    expect(colorRowsToggle.className).toContain("bg-primary");
    expect(colorRowsToggle.className).toContain("text-primary-foreground");
    expect(colorRowsToggle.className).toContain("shadow-sm");
    expect(firstRow?.className).toContain("bg-success/10");

    await user.click(colorRowsToggle);

    expect(colorRowsToggle).toHaveAttribute("aria-pressed", "false");
    expect(firstRow?.className).toContain("bg-transparent");
  });

  it("disables the last visible data column in the menu", async () => {
    const user = userEvent.setup();
    render(<InventoryShell />);

    await user.click(screen.getByRole("button", { name: /Columns/i }));
    await user.click(screen.getByRole("checkbox", { name: "Links" }));
    await user.click(screen.getByRole("checkbox", { name: "Location" }));
    await user.click(screen.getByRole("checkbox", { name: "Description" }));
    await user.click(screen.getByRole("checkbox", { name: "Model" }));
    await user.click(screen.getByRole("checkbox", { name: "Manufacturer" }));

    expect(screen.getByRole("checkbox", { name: "Qty" })).toBeDisabled();
    expect(screen.getByRole("columnheader", { name: /Qty/i })).toBeInTheDocument();
  });
});

function buildEntry(overrides: Partial<InventoryEntry> = {}): InventoryEntry {
  return {
    archived: false,
    assetNumber: "ME-ROW",
    description: "Table row entry",
    id: "row-1",
    links: "",
    lifecycleStatus: "active",
    location: "Bench",
    manufacturer: "Acme",
    model: "Row",
    notes: "",
    projectName: "Inventory",
    qty: 1,
    updatedAt: "2026-04-25T12:00:00.000Z",
    verifiedInSurvey: false,
    workingStatus: "working",
    ...overrides,
  };
}
