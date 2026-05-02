import type { InventoryEntry } from "@/features/inventory/types";

import { useMediaQuery } from "./useMediaQuery";

const LARGE_VIEWPORT_QUERY = "(min-width: 1024px)";

interface UseEntryDialogLayoutOptions {
  entry?: InventoryEntry | null;
  mode: "add" | "edit";
  picturePath: string;
  readOnly: boolean;
}

export function useEntryDialogLayout({
  entry,
  mode,
  picturePath,
  readOnly,
}: UseEntryDialogLayoutOptions) {
  const isLargeViewport = useMediaQuery(LARGE_VIEWPORT_QUERY);
  const showsSidebarActions = mode === "edit" && Boolean(entry) && isLargeViewport;
  const hasPicturePath = Boolean(picturePath);

  return {
    showInlinePicturePreview: (!showsSidebarActions && !readOnly) || (!showsSidebarActions && hasPicturePath),
    showSidebarPicturePreview: showsSidebarActions && (!readOnly || hasPicturePath),
    showsSidebarActions,
  };
}
