import { describe, expect, test } from "vitest";

import { clampRenderedTickRange, getRulerConfig } from "../ruler-utils";
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

describe("clampRenderedTickRange", () => {
  test("leaves a range within budget unchanged", () => {
    expect(
      clampRenderedTickRange({
        startTickIndex: 10,
        endTickIndex: 200,
        viewportStartTickIndex: 50,
        viewportEndTickIndex: 150,
        maxTicks: 2000,
      }),
    ).toEqual({ startTickIndex: 10, endTickIndex: 200 });
  });

  test("keeps the viewport covered when the scroll buffer is over budget", () => {
    // Scrolled far right: the buffer (15% of scrollLeft) spans many more
    // ticks than the budget, on both sides of the viewport.
    const { startTickIndex, endTickIndex } = clampRenderedTickRange({
      startTickIndex: 40_000,
      endTickIndex: 70_000,
      viewportStartTickIndex: 55_000,
      viewportEndTickIndex: 55_200,
      maxTicks: 2000,
    });
    expect(endTickIndex - startTickIndex + 1).toBe(2000);
    expect(startTickIndex).toBeLessThanOrEqual(55_000);
    expect(endTickIndex).toBeGreaterThanOrEqual(55_200);
  });

  test("shifts the budget toward the buffer when the viewport is at the end", () => {
    const { startTickIndex, endTickIndex } = clampRenderedTickRange({
      startTickIndex: 0,
      endTickIndex: 10_000,
      viewportStartTickIndex: 9_950,
      viewportEndTickIndex: 10_050,
      maxTicks: 2000,
    });
    expect(endTickIndex).toBe(10_000);
    expect(startTickIndex).toBe(8_001);
  });

  test("bounds the range when the viewport alone exceeds the budget", () => {
    const { startTickIndex, endTickIndex } = clampRenderedTickRange({
      startTickIndex: 0,
      endTickIndex: Number.POSITIVE_INFINITY,
      viewportStartTickIndex: 100,
      viewportEndTickIndex: 1_000_000,
      maxTicks: 2000,
    });
    expect(startTickIndex).toBe(100);
    expect(endTickIndex).toBe(2_099);
  });
});
