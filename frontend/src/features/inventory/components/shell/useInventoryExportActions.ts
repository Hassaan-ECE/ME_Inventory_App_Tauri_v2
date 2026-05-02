interface UseInventoryExportActionsOptions {
  announceStatus: (message: string) => void;
}

export function useInventoryExportActions({ announceStatus }: UseInventoryExportActionsOptions) {
  async function handleExportExcel(): Promise<void> {
    if (!window.inventoryDesktop?.exportExcel) {
      announceStatus("Excel export is only available in the desktop app.");
      return;
    }

    try {
      const result = await window.inventoryDesktop.exportExcel();
      if (result.canceled) {
        return;
      }
      if (result.error) {
        announceStatus("Excel export failed.");
        return;
      }

      announceStatus("Excel export completed.");
    } catch {
      announceStatus("Excel export failed.");
    }
  }

  function handleExportHtml(): void {
    announceStatus("HTML export is not implemented yet.");
  }

  return { handleExportExcel, handleExportHtml };
}
