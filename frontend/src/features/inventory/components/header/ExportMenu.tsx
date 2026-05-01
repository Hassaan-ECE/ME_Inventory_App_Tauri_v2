import { ChevronDownIcon, FileCodeIcon, FileSpreadsheetIcon, UploadIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Button } from "@/shared/components/ui/button";

interface ExportMenuProps {
  onExportExcel: () => void;
  onExportHtml: () => void;
}

export function ExportMenu({ onExportExcel, onExportHtml }: ExportMenuProps) {
  const [open, setOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) {
      return undefined;
    }

    function handlePointerDown(event: MouseEvent): void {
      if (!menuRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    }

    function handleKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape") {
        setOpen(false);
      }
    }

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  function handleExportExcel(): void {
    setOpen(false);
    onExportExcel();
  }

  function handleExportHtml(): void {
    setOpen(false);
    onExportHtml();
  }

  return (
    <div className="relative" ref={menuRef}>
      <Button aria-expanded={open} aria-haspopup="menu" size="sm" variant="outline" onClick={() => setOpen((current) => !current)}>
        <UploadIcon className="size-3.5" />
        Export
        <ChevronDownIcon className="size-3.5" />
      </Button>
      {open ? (
        <div className="absolute right-0 z-20 mt-2 w-44 rounded-2xl border border-border/70 bg-card p-2 shadow-lg" role="menu">
          <button
            className="flex w-full items-center gap-2 rounded-xl px-3 py-2 text-left text-sm text-foreground hover:bg-accent/60"
            role="menuitem"
            type="button"
            onClick={handleExportExcel}
          >
            <FileSpreadsheetIcon className="size-4" />
            Excel
          </button>
          <button
            className="flex w-full items-center gap-2 rounded-xl px-3 py-2 text-left text-sm text-foreground hover:bg-accent/60"
            role="menuitem"
            type="button"
            onClick={handleExportHtml}
          >
            <FileCodeIcon className="size-4" />
            HTML
          </button>
        </div>
      ) : null}
    </div>
  );
}
