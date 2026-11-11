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
  // The step the dots are laid at: the requested one, or a power-of-2 multiple
  // of it when the viewport would otherwise need more than MAX_GRID_DOTS.
  step: number;
  startX: number;
  startY: number;
  cols: number;
  rows: number;
}

// Dot positions covering `viewport` at `step`, as a first dot plus a column
// and row count, so the caller iterates by index rather than accumulating
// floats (at very large pan offsets `x += step` can stop changing `x`).
// When the viewport would need more than MAX_GRID_DOTS dots at `step`, the
// step doubles until it fits (a large canvas at modest zoom still gets a grid).
// Returns null when the viewport is not finite or no step within reach fits.
export function gridDotLayout(viewport: Rect, step: number): GridDotLayout | null {
  const { x, y, width, height } = viewport;
  if (![x, y, width, height, step].every(Number.isFinite) || step <= 0) return null;
  if (width < 0 || height < 0) return { step, startX: 0, startY: 0, cols: 0, rows: 0 };
  // 64 doublings reach past any finite double's useful range; a bound, not a tuning knob.
  for (let doublings = 0; doublings < 64 && Number.isFinite(step); doublings++, step *= 2) {
    const firstCol = Math.floor(x / step);
    const firstRow = Math.floor(y / step);
    const cols = Math.floor((x + width) / step) - firstCol + 1;
    const rows = Math.floor((y + height) / step) - firstRow + 1;
    if (!Number.isFinite(cols) || !Number.isFinite(rows)) return null;
    if (cols * rows <= MAX_GRID_DOTS) {
      return { step, startX: firstCol * step, startY: firstRow * step, cols, rows };
    }
  }
  return null;
}
