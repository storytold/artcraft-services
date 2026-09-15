// Live readout from the hero galaxy for the slate's timecode.
//
// A plain mutable record, written by the galaxy's frame loop (cheap field
// store, no React) and read by DOM consumers on their own throttled
// tickers via direct textContent writes — the same no-rerender pattern as
// the heroWordmark channel. The value freezes whenever the galaxy parks
// its frameloop (hero offscreen, hidden tab), which is honest: the
// instrument really is holding still. It also doubles as the slate's
// liveness signal — time stuck at 0 means the galaxy never ran (reduced
// motion, WebGL unavailable) and the slate chrome stays hidden.
//
// (A fuller telemetry set — fps, card count, spin, boost — once fed
// flank readout stacks; those were cut as filler on a product site.)
export type HeroTelemetry = {
  /** Monotonic scene time, seconds — drives the slate timecode. */
  time: number;
};

export const heroTelemetry: HeroTelemetry = {
  time: 0,
};
