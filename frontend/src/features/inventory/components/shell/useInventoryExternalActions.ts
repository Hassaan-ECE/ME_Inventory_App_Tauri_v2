import type { InventoryEntry } from "@/features/inventory/types";
import { toSafeExternalUrl } from "@/shared/lib/externalUrl";

interface UseInventoryExternalActionsOptions {
  announceStatus: (message: string) => void;
  entriesById: Map<string, InventoryEntry>;
}

export function useInventoryExternalActions({ announceStatus, entriesById }: UseInventoryExternalActionsOptions) {
  async function handleOpenExternalLink(url: string, successMessage = "Opened link."): Promise<void> {
    const opened = await openExternalUrl(url);
    if (!opened) {
      announceStatus("Could not open the saved link.");
      return;
    }

    announceStatus(successMessage);
  }

  async function handleOpenEntryLink(entryId: string): Promise<void> {
    const entry = entriesById.get(entryId);
    if (!entry) {
      return;
    }

    const linkText = entry.links.trim();
    if (!linkText) {
      announceStatus("No link is saved for this entry.");
      return;
    }

    const externalUrl = toSafeExternalUrl(linkText);
    if (!externalUrl) {
      announceStatus("This link is not in a valid format.");
      return;
    }

    await handleOpenExternalLink(externalUrl, `Opened link: ${linkText}`);
  }

  async function handleSearchOnline(entryId: string): Promise<void> {
    const entry = entriesById.get(entryId);
    if (!entry) {
      return;
    }

    const queryText = [entry.manufacturer, entry.model, entry.description].filter((value) => value.trim()).join(" ").trim();
    if (!queryText) {
      announceStatus("No searchable entry details were found.");
      return;
    }

    const searchUrl = toSafeExternalUrl(`https://www.google.com/search?q=${encodeURIComponent(queryText)}`, {
      allowImplicitHttps: false,
    });
    if (!searchUrl) {
      announceStatus("Could not build a safe browser search URL.");
      return;
    }
    const opened = await openExternalUrl(searchUrl);
    if (!opened) {
      announceStatus("Could not open the browser for this search.");
      return;
    }

    announceStatus(`Opened web search: ${queryText}`);
  }

  return { handleOpenEntryLink, handleOpenExternalLink, handleSearchOnline };
}

async function openExternalUrl(url: string): Promise<boolean> {
  const externalUrl = toSafeExternalUrl(url, { allowImplicitHttps: false });
  if (!externalUrl) {
    return false;
  }

  if (window.inventoryDesktop?.openExternal) {
    return window.inventoryDesktop.openExternal(externalUrl);
  }

  window.open(externalUrl, "_blank", "noopener,noreferrer");
  return true;
}
