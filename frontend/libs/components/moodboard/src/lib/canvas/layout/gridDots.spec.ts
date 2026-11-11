import { describe, expect, it } from "vitest";
import { MAX_GRID_DOTS, effectiveGridSpacing, gridDotLayout } from "./gridDots";

describe("effectiveGridSpacing", () => {
  it("keeps normal spacing and coarsens when zoomed out", () => {
    expect(effectiveGridSpacing(32, 1)).toBe(32);
    expect(effectiveGridSpacing(32, 0.25)).toBe(64);
  });

  it("refuses spacing or zoom that would never advance the draw loop", () => {
    for (const spacing of [0, -32, -1e-9, NaN, Infinity, -Infinity]) {
      expect(effectiveGridSpacing(spacing, 1)).toBeNull();
    }
    for (const zoom of [0, -1, NaN, Infinity]) {
      expect(effectiveGridSpacing(32, zoom)).toBeNull();
    }
  });
});

describe("gridDotLayout", () => {
  it("covers a normal viewport well under the budget", () => {
    const step = effectiveGridSpacing(32, 1) as number;
    const grid = gridDotLayout({ x: -10, y: 5, width: 3840, height: 2160 }, step);
    expect(grid).not.toBeNull();
    expect(grid?.startX).toBe(-32);
    expect(grid?.startY).toBe(0);
    expect((grid?.cols ?? 0) * (grid?.rows ?? 0)).toBeLessThan(MAX_GRID_DOTS);
  });

  it("coarsens the step instead of drawing nothing for a large viewport", () => {
    // A 4K screen at zoom 0.5: 7680 × 4320 stage units, 32,776 dots at step 32.
    const grid = gridDotLayout({ x: 0, y: 0, width: 7680, height: 4320 }, 32);
    expect(grid?.step).toBe(64);
    expect((grid?.cols ?? 0) * (grid?.rows ?? 0)).toBeLessThanOrEqual(MAX_GRID_DOTS);
  });

  it("keeps the dot count bounded for an oversized viewport, or refuses it", () => {
    const grid = gridDotLayout({ x: 0, y: 0, width: 1e9, height: 1e9 }, 1);
    expect((grid?.cols ?? 0) * (grid?.rows ?? 0)).toBeLessThanOrEqual(MAX_GRID_DOTS);
    expect(gridDotLayout({ x: 0, y: 0, width: 1e300, height: 1e300 }, 1)).toBeNull();
  });

  it("refuses non-finite input", () => {
    expect(gridDotLayout({ x: NaN, y: 0, width: 10, height: 10 }, 1)).toBeNull();
    expect(gridDotLayout({ x: 0, y: 0, width: Infinity, height: 10 }, 1)).toBeNull();
    expect(gridDotLayout({ x: 0, y: 0, width: 10, height: 10 }, -1)).toBeNull();
  });

  it("returns a bounded count at pan offsets beyond float step precision", () => {
    // At 1e20, adding 32 does not change the value; an accumulating loop
    // would never terminate.
    const grid = gridDotLayout({ x: 1e20, y: 1e20, width: 1000, height: 1000 }, 32);
    expect(grid).not.toBeNull();
    expect((grid?.cols ?? 0) * (grid?.rows ?? 0)).toBeLessThanOrEqual(MAX_GRID_DOTS);
  });
});
