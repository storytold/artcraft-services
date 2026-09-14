// Live readouts from the hero galaxy for the slate's flank annotations.
//
// A plain mutable record, written by the galaxy's frame loop (cheap field
// stores, no React) and read by DOM consumers on their own throttled
// tickers via direct textContent writes — the same no-rerender pattern as
// the heroWordmark channel. Values freeze whenever the galaxy parks its
// frameloop (hero offscreen, hidden tab), which is honest: the instrument
// really is holding still.
export type HeroTelemetry = {
  /** Smoothed frames per second (1 / frame EMA). */
  fps: number;
  /** Cards currently riding the arms (after the perf governor's shed). */
  cards: number;
  /** Clips holding live decoders right now (playing videos). */
  clipsLive: number;
  /** Total clips in the showcase pool. */
  clipPool: number;
  /** Accumulated rig spin, degrees (signed, unwrapped). */
  spinDeg: number;
  /** Monotonic scene time, seconds — drives the slate timecode. */
  time: number;
  /** True while a card is click-boosted. */
  boost: boolean;
};

export const heroTelemetry: HeroTelemetry = {
  fps: 0,
  cards: 0,
  clipsLive: 0,
  clipPool: 0,
  spinDeg: 0,
  time: 0,
  boost: false,
};
