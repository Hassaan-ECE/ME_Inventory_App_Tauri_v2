import { useEffect, useState } from "react";

import type { ColumnKey, ThemeMode } from "@/features/inventory/types";

import {
  COLOR_ROWS_STORAGE_KEY,
  COLUMN_VISIBILITY_STORAGE_KEY,
  THEME_STORAGE_KEY,
  readColorRows,
  readColumnVisibility,
  readTheme,
} from "./helpers";

export function useInventoryPreferences() {
  const [theme, setTheme] = useState<ThemeMode>(() => readTheme());
  const [colorRows, setColorRows] = useState<boolean>(() => readColorRows());
  const [columnVisibility, setColumnVisibility] = useState<Record<ColumnKey, boolean>>(() => readColumnVisibility());

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  }, [theme]);

  useEffect(() => {
    localStorage.setItem(COLOR_ROWS_STORAGE_KEY, JSON.stringify(colorRows));
  }, [colorRows]);

  useEffect(() => {
    localStorage.setItem(COLUMN_VISIBILITY_STORAGE_KEY, JSON.stringify(columnVisibility));
  }, [columnVisibility]);

  function handleThemeToggle(): void {
    setTheme((current) => (current === "light" ? "dark" : "light"));
  }

  return {
    colorRows,
    columnVisibility,
    handleThemeToggle,
    setColorRows,
    setColumnVisibility,
    theme,
  };
}
