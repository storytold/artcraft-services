import type { Rect } from "../types";

// Minimum on-screen gap between dots before we drop to a coarser grid.
// At zoom 1× with spacing 32, dots are 32px apart; as you zoom out, the
// LOD step doubles `spacing` so density never drops below this bound.
const MIN_SCREEN_SPACING = 16;

// Hard cap on dots drawn per frame. The LOD step keeps a normal viewport far
// below this; it only matters if spacing, zoom, or the viewport come from a
// malformed document.
export const MAX_GRID_DOTS = 20_000;

// Grid step in stage units for the given spacing and zoom, or null when no
// grid should be drawn. Loaded documents can carry zero, negative, or
// non-finite spacing; any of those would keep the background's draw loop
// from ever reaching the end of the viewport.
export function effectiveGridSpacing(spacing: number, zoom: number): number | null {
  if (!Number.isFinite(spacing) || spacing <= 0) return null;
  if (!Number.isFinite(zoom) || zoom <= 0) return null;
  const baseOnScreen = spacing * zoom;
  const lodStep =
    baseOnScreen >= MIN_SCREEN_SPACING
      ? 0
      : Math.ceil(Math.log2(MIN_SCREEN_SPACING / Math.max(baseOnScreen, 1e-6)));
  const step = spacing * Math.pow(2, lodStep);
  return Number.isFinite(step) && step > 0 ? step : null;
}

export interface GridDotLayout {
  startX: number;
  startY: number;
  cols: number;
  rows: number;
}

// Dot positions covering `viewport` at `step`, as a first dot plus a column
// and row count, so the caller iterates by index rather than accumulating
// floats (at very large pan offsets `x += step` can stop changing `x`).
// Returns null when the viewport is not finite or would need more than
// MAX_GRID_DOTS dots.
export function gridDotLayout(viewport: Rect, step: number): GridDotLayout | null {
  const { x, y, width, height } = viewport;
  if (![x, y, width, height, step].every(Number.isFinite) || step <= 0) return null;
  if (width < 0 || height < 0) return { startX: 0, startY: 0, cols: 0, rows: 0 };
  const firstCol = Math.floor(x / step);
  const firstRow = Math.floor(y / step);
  const cols = Math.floor((x + width) / step) - firstCol + 1;
  const rows = Math.floor((y + height) / step) - firstRow + 1;
  if (!Number.isFinite(cols) || !Number.isFinite(rows)) return null;
  if (cols * rows > MAX_GRID_DOTS) return null;
  return { startX: firstCol * step, startY: firstRow * step, cols, rows };
}
