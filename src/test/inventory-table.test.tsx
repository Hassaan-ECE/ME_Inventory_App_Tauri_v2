import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { InventoryShell } from "@/components/inventory/InventoryShell";
import { InventoryTable } from "@/components/inventory/InventoryTable";
import { INVENTORY_COLUMNS, type InventoryEntry } from "@/types/inventory";

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
        onSortChange={() => undefined}
        onToggleVerified={() => undefined}
      />,
    );

    expect(screen.getByText("javascript:alert(1)")).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "javascript:alert(1)" })).not.toBeInTheDocument();
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
