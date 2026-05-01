import { describe, expect, it } from "vitest";

import { isWindowStateVisibleOnAnyMonitor, parseStoredWindowState } from "@/integrations/tauri/windowState";

describe("window state persistence helpers", () => {
  it("parses and clamps stored window geometry", () => {
    const state = parseStoredWindowState(
      JSON.stringify({
        height: 500,
        maximized: true,
        savedAt: "2026-04-30T00:00:00.000Z",
        width: 800,
        x: 10.4,
        y: 20.6,
      }),
    );

    expect(state).toMatchObject({
      height: 720,
      maximized: true,
      savedAt: "2026-04-30T00:00:00.000Z",
      width: 1100,
      x: 10,
      y: 21,
    });
  });

  it("ignores invalid stored window geometry", () => {
    expect(parseStoredWindowState(null)).toBeNull();
    expect(parseStoredWindowState("{not json")).toBeNull();
    expect(parseStoredWindowState(JSON.stringify({ height: 720, width: 1100, x: "10", y: 20 }))).toBeNull();
  });

  it("checks whether a restored window would still be visible", () => {
    const monitors = [
      {
        workArea: {
          position: { x: 0, y: 0 },
          size: { height: 1080, width: 1920 },
        },
      },
    ];

    expect(isWindowStateVisibleOnAnyMonitor({ height: 720, width: 1100, x: 100, y: 100 }, monitors)).toBe(true);
    expect(isWindowStateVisibleOnAnyMonitor({ height: 720, width: 1100, x: 3000, y: 100 }, monitors)).toBe(false);
    expect(isWindowStateVisibleOnAnyMonitor({ height: 720, width: 1100, x: 3000, y: 100 }, [])).toBe(true);
  });
});
