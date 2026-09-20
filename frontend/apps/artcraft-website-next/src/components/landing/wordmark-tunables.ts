import { defineTunables } from "@/lib/tuner";

// The hero wordmark's type: variable Archivo, so HeadingFlow can morph the
// letters down to the headings' display setting during the hero flip
// (one family, weight and width genuinely interpolate).
export const WORDMARK_FONT = "var(--font-archivo), system-ui, sans-serif";
// Poster width — the Archivo Black stretch — held constant; the resting
// weight is a live knob (see below).
export const WORDMARK_STRETCH = 125;
// The headings' display cut the flip lands on (mirrors .font-display).
export const DISPLAY_WEIGHT = 620;
export const DISPLAY_STRETCH = 118;

// The logo's optical fit against the glyphs is dialed by eye: the browser
// aligns an inline SVG's BOX bottom to the text baseline, but glyph ink is
// placed from font metrics (baseline overshoot, cap forms sitting a hair
// off the geometric lines in heavy masters) — there is no CSS auto-sync
// between SVG geometry and type ink, so cap height, baseline lift, side
// bearing and stroke weight are live knobs to tune and bake.
export const wordmarkTuner = defineTunables("wordmark", "Wordmark", {
  weight: {
    label: "Type weight",
    min: 400,
    max: 900,
    step: 10,
    default: 750,
    info: "Resting font weight of the letters. The logo-A is a fixed-stroke SVG (about 0.22 of its cap height), so the letters' stems are dialed down from the 900 poster extreme to match it.",
  },
  logoCap: {
    label: "Logo cap em",
    min: 0.5,
    max: 1,
    step: 0.005,
    default: 0.7,
    info: "Height of the logo-A relative to the font size — match it to the caps' visual height.",
  },
  logoLift: {
    label: "Logo lift em",
    min: -0.1,
    max: 0.1,
    step: 0.002,
    default: -0.006,
    info: "Vertical baseline correction of the logo-A: positive raises it. Syncs the SVG's box-bottom alignment with the glyphs' ink baseline.",
  },
  logoPad: {
    label: "Logo pad em",
    min: 0,
    max: 0.15,
    step: 0.005,
    default: 0.05,
    info: "Side bearing between the logo-A and the R, standing in for the glyph spacing the SVG doesn't have.",
  },
  bladeTuck: {
    label: "Blade tuck em",
    min: 0,
    max: 0.5,
    step: 0.01,
    default: 0.12,
    info: "Extra distance the sliding word starts tucked behind the blade edge — kills any sliver peeking past the clip at rest.",
  },
  scrimPadX: {
    label: "Scrim pad x",
    min: 0,
    max: 320,
    step: 8,
    default: 128,
    info: "How far the contrast scrim behind the center stack extends horizontally beyond the content.",
  },
  scrimPadY: {
    label: "Scrim pad y",
    min: 0,
    max: 240,
    step: 8,
    default: 88,
    info: "How far the contrast scrim extends vertically beyond the content.",
  },
  scrimBg: {
    label: "Scrim peak %",
    min: 40,
    max: 95,
    step: 1,
    default: 78,
    info: "Peak page-background strength at the scrim's center — the contrast pocket the wordmark and copy sit in over the galaxy.",
  },
});

export function wordmarkDefaults(): {
  [K in keyof typeof wordmarkTuner.defs]: number;
} {
  const out = {} as { [K in keyof typeof wordmarkTuner.defs]: number };
  for (const key of Object.keys(wordmarkTuner.defs) as Array<
    keyof typeof wordmarkTuner.defs
  >) {
    out[key] = wordmarkTuner.defs[key].default;
  }
  return out;
}
