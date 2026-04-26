import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { EntryDialog } from "@/components/inventory/EntryDialog";
import type { InventoryEntry, InventoryEntryInput } from "@/types/inventory";

const BASE_ENTRY: InventoryEntry = {
  archived: false,
  assetNumber: "ME-401",
  description: "Precision fixture plate",
  id: "401",
  links: "",
  lifecycleStatus: "active",
  location: "Shelf B2",
  manufacturer: "Acme",
  model: "FP-401",
  notes: "",
  picturePath: "C:\\Pictures\\fixture-plate.jpg",
  projectName: "Fixture Lab",
  qty: 1,
  entryUuid: "uuid-401",
  serialNumber: "SER-401",
  updatedAt: "2026-04-23 09:00:00",
  verifiedInSurvey: true,
  workingStatus: "working",
};

describe("EntryDialog", () => {
  beforeEach(() => {
    document.documentElement.classList.remove("dark");
    delete window.inventoryDesktop;
    vi.restoreAllMocks();
    mockMatchMedia(false);
  });

  it("hides the picture path from the preview but saves it with the entry input", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn().mockResolvedValue(undefined) as unknown as (_: InventoryEntryInput) => Promise<void>;

    render(
      <EntryDialog
        mode="edit"
        entry={BASE_ENTRY}
        onClose={vi.fn()}
        onSave={onSave}
      />,
    );

    expect(screen.queryByLabelText("Picture Path")).not.toBeInTheDocument();
    expect(screen.queryByText("C:\\Pictures\\fixture-plate.jpg")).not.toBeInTheDocument();
    expect(screen.queryByText("Selected image")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Save Entry" }));

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        picturePath: "C:\\Pictures\\fixture-plate.jpg",
      }),
    );
  });

  it("fills the picture path from the desktop picker without showing the selected path", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn().mockResolvedValue(undefined) as unknown as (_: InventoryEntryInput) => Promise<void>;
    const pickPicturePath = vi.fn().mockResolvedValue("C:\\Pictures\\selected-image.jpg");
    window.inventoryDesktop = createDesktopBridge({
      pickPicturePath,
    });

    render(
      <EntryDialog
        mode="add"
        onClose={vi.fn()}
        onSave={onSave}
      />,
    );

    await user.type(screen.getByLabelText("Asset Number"), "ME-900");
    await user.click(screen.getByRole("button", { name: "Browse" }));

    expect(pickPicturePath).toHaveBeenCalledTimes(1);
    expect(screen.queryByLabelText("Picture Path")).not.toBeInTheDocument();
    expect(screen.queryByText("C:\\Pictures\\selected-image.jpg")).not.toBeInTheDocument();
    expect(screen.queryByText("Selected image")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Save Entry" }));

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        assetNumber: "ME-900",
        picturePath: "C:\\Pictures\\selected-image.jpg",
      }),
    );
  });

  it("opens the picture in the desktop viewer from the large-screen preview", async () => {
    const openPath = vi.fn().mockResolvedValue(true);
    window.inventoryDesktop = createDesktopBridge({
      openPath,
    });
    mockMatchMedia(true);

    render(
      <EntryDialog
        mode="edit"
        entry={BASE_ENTRY}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />,
    );

    fireEvent.load(screen.getByAltText("Entry picture preview"));
    fireEvent.doubleClick(screen.getByRole("button", { name: "Picture preview" }));

    expect(openPath).toHaveBeenCalledWith("C:\\Pictures\\fixture-plate.jpg");
  });

  it("shows a missing-picture fallback when the preview fails to load", () => {
    render(
      <EntryDialog
        mode="edit"
        entry={BASE_ENTRY}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />,
    );

    fireEvent.error(screen.getByAltText("Entry picture preview"));

    expect(screen.getAllByText("Picture not found").length).toBeGreaterThan(0);
  });

  it("uses the taller large-screen dialog sizing so the 1080p editor has more headroom", () => {
    mockMatchMedia(true);

    render(
      <EntryDialog
        mode="edit"
        entry={BASE_ENTRY}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />,
    );

    const dialogPanel = screen.getByRole("dialog").firstElementChild;
    expect(dialogPanel).toHaveClass("max-h-[92vh]", "lg:max-h-[94vh]");
  });

  it("uses dark-safe native select and option colors", () => {
    document.documentElement.classList.add("dark");

    render(
      <EntryDialog
        mode="edit"
        entry={BASE_ENTRY}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />,
    );

    const lifecycleSelect = screen.getByLabelText("Lifecycle");
    const workingStatusSelect = screen.getByLabelText("Working Status");

    expect(lifecycleSelect).toHaveClass("dark:bg-neutral-950", "dark:text-neutral-100");
    expect(workingStatusSelect).toHaveClass("dark:bg-neutral-950", "dark:text-neutral-100");
    expect(screen.getByRole("option", { name: "Active" })).toHaveClass("dark:bg-neutral-950", "dark:text-neutral-100");
    expect(screen.getByRole("option", { name: "Working" })).toHaveClass("dark:bg-neutral-950", "dark:text-neutral-100");
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

function createDesktopBridge(
  overrides: Partial<NonNullable<Window["inventoryDesktop"]>>,
): NonNullable<Window["inventoryDesktop"]> {
  return {
    isDesktop: true,
    loadInventory: vi.fn().mockResolvedValue({ dbPath: "", entries: [] }),
    syncInventory: vi.fn().mockResolvedValue({
      dbPath: "",
      entries: [],
      shared: {
        available: true,
        canModify: true,
        enabled: true,
        message: "",
      },
    }),
    toggleVerifiedEntry: vi.fn().mockResolvedValue(BASE_ENTRY),
    createEntry: vi.fn().mockResolvedValue(BASE_ENTRY),
    updateEntry: vi.fn().mockResolvedValue(BASE_ENTRY),
    setArchivedEntry: vi.fn().mockResolvedValue(BASE_ENTRY),
    deleteEntry: vi.fn().mockResolvedValue({ entryId: BASE_ENTRY.id }),
    openExternal: vi.fn().mockResolvedValue(true),
    openPath: vi.fn().mockResolvedValue(true),
    pickPicturePath: vi.fn().mockResolvedValue(null),
    ...overrides,
  } as NonNullable<Window["inventoryDesktop"]>;
}
