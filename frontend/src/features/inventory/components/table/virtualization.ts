export const ROW_HEIGHT = 45;
export const OVERSCAN_ROWS = 10;

interface VisibleRange {
  end: number;
  start: number;
}

export function getVisibleRange(entryCount: number, scrollTop: number, viewportHeight: number): VisibleRange {
  const safeEntryCount = getSafeEntryCount(entryCount);
  if (safeEntryCount === 0) {
    return { end: 0, start: 0 };
  }

  const safeViewportHeight = getSafeViewportHeight(viewportHeight);
  const safeScrollTop = clampScrollTop(scrollTop, safeEntryCount, safeViewportHeight);
  const firstVisibleRow = Math.min(safeEntryCount - 1, Math.floor(safeScrollTop / ROW_HEIGHT));
  const visibleRowCount = Math.max(1, Math.ceil(safeViewportHeight / ROW_HEIGHT));
  const start = Math.max(0, firstVisibleRow - OVERSCAN_ROWS);

  return {
    end: Math.min(safeEntryCount, firstVisibleRow + visibleRowCount + OVERSCAN_ROWS),
    start,
  };
}

export function clampScrollTop(scrollTop: number, entryCount: number, viewportHeight: number): number {
  const safeScrollTop = Number.isFinite(scrollTop) ? Math.max(0, scrollTop) : 0;
  return Math.min(safeScrollTop, getMaxScrollTop(entryCount, viewportHeight));
}

function getMaxScrollTop(entryCount: number, viewportHeight: number): number {
  const safeEntryCount = getSafeEntryCount(entryCount);
  return Math.max(0, safeEntryCount * ROW_HEIGHT - getSafeViewportHeight(viewportHeight));
}

function getSafeEntryCount(entryCount: number): number {
  return Number.isFinite(entryCount) ? Math.max(0, Math.floor(entryCount)) : 0;
}

function getSafeViewportHeight(viewportHeight: number): number {
  return Number.isFinite(viewportHeight) && viewportHeight > 0 ? viewportHeight : ROW_HEIGHT;
}
