import type { KeyboardEvent as ReactKeyboardEvent } from "react";

import { toSafeExternalUrl } from "@/shared/lib/externalUrl";

export type PicturePreviewState = "empty" | "loading" | "loaded" | "missing";

export function buildPicturePreviewSource(picturePath: string): string | null {
  const trimmedPath = picturePath.trim();
  if (!trimmedPath) {
    return null;
  }

  if (/^(?:https?:|file:|data:)/i.test(trimmedPath)) {
    return trimmedPath;
  }

  const normalizedPath = trimmedPath.replaceAll("\\", "/");
  if (/^(?:[a-zA-Z]:\/|\/\/)/.test(normalizedPath)) {
    return encodeURI(`file:${normalizedPath.startsWith("//") ? normalizedPath : `///${normalizedPath}`}`);
  }

  return null;
}

export function shouldLoadNativePicturePreview(picturePath: string): boolean {
  return !/^(?:https?:|mailto:|data:|file:)/i.test(picturePath.trim());
}

export function handlePreviewKeyDown(
  event: ReactKeyboardEvent<HTMLDivElement>,
  canOpen: boolean,
  onOpen: () => void,
): void {
  if (!canOpen) {
    return;
  }

  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    onOpen();
  }
}

export async function openPictureTarget(targetPath: string): Promise<boolean> {
  const trimmedTargetPath = targetPath.trim();
  if (!trimmedTargetPath) {
    return false;
  }

  const externalUrl = toSafeExternalUrl(trimmedTargetPath);
  if (externalUrl) {
    if (window.inventoryDesktop?.openExternal) {
      return window.inventoryDesktop.openExternal(externalUrl);
    }

    window.open(externalUrl, "_blank", "noopener,noreferrer");
    return true;
  }

  if (window.inventoryDesktop?.openPath) {
    return window.inventoryDesktop.openPath(trimmedTargetPath);
  }

  return false;
}
