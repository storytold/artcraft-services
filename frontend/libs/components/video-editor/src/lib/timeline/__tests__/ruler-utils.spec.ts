import { describe, expect, test } from "vitest";

import { getRulerConfig } from "../ruler-utils";
import { BASE_TIMELINE_PIXELS_PER_SECOND } from "../scale";
import { DEFAULT_FPS } from "../../fps/defaults";

describe("getRulerConfig", () => {
  test("keeps tick spacing when zoomed out past the largest interval", () => {
    // A timeline thousands of hours long fitted to a ~2000px viewport.
    const zoomLevel = 1e-6;
    const pixelsPerSecond = BASE_TIMELINE_PIXELS_PER_SECOND * zoomLevel;

    const { tickIntervalSeconds, labelIntervalSeconds } = getRulerConfig({
      zoomLevel,
      fps: DEFAULT_FPS,
    });

    expect(tickIntervalSeconds * pixelsPerSecond).toBeGreaterThanOrEqual(18);
    expect(labelIntervalSeconds * pixelsPerSecond).toBeGreaterThanOrEqual(120);
    expect(tickIntervalSeconds % 3600).toBe(0);
    // Ticks across a 2000px viewport stay bounded by the spacing rule.
    expect(2000 / pixelsPerSecond / tickIntervalSeconds).toBeLessThanOrEqual(
      2000 / 18 + 1,
    );
  });

  test("returns finite intervals for a degenerate zoom", () => {
    for (const zoomLevel of [0, -1, Number.NaN]) {
      const { tickIntervalSeconds, labelIntervalSeconds } = getRulerConfig({
        zoomLevel,
        fps: DEFAULT_FPS,
      });
      expect(Number.isFinite(tickIntervalSeconds)).toBe(true);
      expect(tickIntervalSeconds).toBeGreaterThan(0);
      expect(Number.isFinite(labelIntervalSeconds)).toBe(true);
    }
  });

  test("still uses the fixed intervals at normal zoom levels", () => {
    const { tickIntervalSeconds } = getRulerConfig({
      zoomLevel: 0.1,
      fps: DEFAULT_FPS,
    });
    expect(tickIntervalSeconds).toBeLessThan(3600);
  });
});
