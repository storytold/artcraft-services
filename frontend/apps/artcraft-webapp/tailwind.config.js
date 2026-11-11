const { createGlobPatternsForDependencies } = require("@nx/react/tailwind");
const { join } = require("path");
import colors from "tailwindcss/colors";

// The brand blue. Themeable: each shade a theme may change reads its own
// --st-accent-* variable (defaults in src/styles.css are the values below).
// Fills use DEFAULT; text, rings and lines get their own variables further
// down, so a theme can give each the shade that reads in its place.
const primaryScale = {
  DEFAULT: "rgb(var(--st-accent-rgb) / <alpha-value>)",
  50: "#eef6ff",
  100: "#d9ebff",
  200: "#bcdcff",
  300: "rgb(var(--st-accent-300-rgb) / <alpha-value>)",
  400: "rgb(var(--st-accent-400-rgb) / <alpha-value>)",
  500: "rgb(var(--st-accent-rgb) / <alpha-value>)",
  600: "rgb(var(--st-accent-600-rgb) / <alpha-value>)",
  700: "rgb(var(--st-accent-700-rgb) / <alpha-value>)",
  800: "#173fb6",
  900: "#19398f",
  950: "#142457",
};

// Faded text (text-white/40 ...) draws at 1 - (1 - alpha) * --st-text-fade.
// The fade is 1 by default, which is exactly the declared alpha; a light
// theme lowers it so its captions stay readable.
const fadedText = (rgbVar) =>
  `rgb(var(${rgbVar}) / calc(1 - (1 - <alpha-value>) * var(--st-text-fade, 1)))`;
// Faded elements (opacity-40 ... opacity-90, mostly captions) follow the same
// fade. 0, 100 and the decorative 5-30 steps stay exact: they show, hide and
// animate things.
const fadedOpacity = Object.fromEntries(
  [40, 50, 60, 70, 75, 80, 90].map((n) => [
    n,
    `calc(1 - (1 - ${n / 100}) * var(--st-text-fade, 1))`,
  ]),
);
// Text in the palette's status colours (text-red-400 errors, amber BETA
// badges ...) reads --st-<family>-<shade>-rgb, which only a theme sets; until
// then the fallback is Tailwind's own colour.
const MEANING_FAMILIES = [
  "rose",
  "indigo",
  "red",
  "blue",
  "orange",
  "amber",
  "green",
  "emerald",
  "teal",
  "cyan",
  "yellow",
  "pink",
  "purple",
];
const MEANING_SHADES = [200, 300, 400, 500, 600];
const COLOR_DEFAULTS = { red: "#D33242", blue: "#2E70FF", green: "#308752" };
const triplet = (hex) =>
  [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16)).join(" ");
const meaningText = Object.fromEntries(
  MEANING_FAMILIES.map((family) => [
    family,
    {
      ...colors[family],
      ...(COLOR_DEFAULTS[family] ? { DEFAULT: COLOR_DEFAULTS[family] } : {}),
      ...Object.fromEntries(
        MEANING_SHADES.map((shade) => [
          shade,
          `rgb(var(--st-${family}-${shade}-rgb, ${triplet(colors[family][shade])}) / <alpha-value>)`,
        ]),
      ),
    },
  ]),
);
// .st-keep-dark: a branded dark surface (the pricing cards) that keeps
// ArtCraft's own colours whatever the theme. Inside it every theme variable
// is reset to its default; when a theme is on it also gets the default page
// colour beneath it, since its see-through gradients were drawn for a dark page.
const KEEP_DARK = {
  "--st-white-rgb": "255 255 255",
  "--st-black-rgb": "0 0 0",
  "--st-text-rgb": "242 241 238",
  "--st-bg-rgb": "18 19 22",
  "--st-panel-rgb": "16 16 20",
  "--st-control-rgb": "30 31 34",
  "--st-sunken-rgb": "6 6 7",
  "--st-accent-rgb": "45 129 255",
  "--st-accent-300-rgb": "142 199 255",
  "--st-accent-400-rgb": "89 167 255",
  "--st-accent-600-rgb": "27 99 245",
  "--st-accent-700-rgb": "20 78 225",
  "--st-accent-ink-rgb": "116 170 255",
  "--st-accent-text-rgb": "45 129 255",
  "--st-accent-line-rgb": "45 129 255",
  "--st-text-fade": "1",
  // The desktop app's names, which shared libs read with their own fallbacks.
  "--st-bg": "initial",
  "--st-fg": "initial",
  "--st-fg-rgb": "initial",
  "--st-divider": "initial",
  "--st-controls-rgb": "initial",
  "--st-icon-filter": "invert(1)",
  "--st-wordmark-filter": "none",
  ...Object.fromEntries(
    MEANING_FAMILIES.flatMap((family) =>
      MEANING_SHADES.map((shade) => [`--st-${family}-${shade}-rgb`, "initial"]),
    ),
  ),
};
const keepDark = ({ addComponents }) =>
  addComponents({
    ".st-keep-dark": KEEP_DARK,
    // Text that sets no colour of its own inherits the page's, which was
    // worked out before the reset above, so the island sets it again.
    // :where() keeps this at zero specificity, so a card's own background
    // and text colour (bg-[#1b1b20], .website-plan-card) still win.
    ':where(:root[data-theme]:not([data-theme="default"]) .st-keep-dark)': {
      backgroundColor: "rgb(18 19 22)",
      color: "rgb(242 241 238)",
      colorScheme: "dark",
    },
  });
const accentFor = (rgbVar) => ({
  ...primaryScale,
  DEFAULT: `rgb(var(${rgbVar}) / <alpha-value>)`,
});

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    join(
      __dirname,
      "{src,pages,components,app}/**/*!(*.stories|*.spec).{ts,tsx,html}",
    ),
    ...createGlobPatternsForDependencies(__dirname),
    join(__dirname, "../../libs/login/src/**/*.{ts,tsx}"),
    "./app/index.html",
    "./app/src/**/*.{js,jsx,ts,tsx}",
    "./app/src/*.{js,jsx,ts,tsx}",
    "../../libs/components/**/*.{js,jsx,ts,tsx}",
  ],
  theme: {
    extend: {
      opacity: fadedOpacity,
      textColor: {
        ...meaningText,
        white: fadedText("--st-white-rgb"),
        "base-fg": fadedText("--st-text-rgb"),
        primary: accentFor("--st-accent-text-rgb"),
        "brand-primary": accentFor("--st-accent-text-rgb"),
      },
      ringColor: {
        primary: accentFor("--st-accent-line-rgb"),
        "brand-primary": accentFor("--st-accent-line-rgb"),
      },
      borderColor: {
        primary: accentFor("--st-accent-line-rgb"),
        "brand-primary": accentFor("--st-accent-line-rgb"),
      },
      outlineColor: {
        primary: accentFor("--st-accent-line-rgb"),
        "brand-primary": accentFor("--st-accent-line-rgb"),
      },
      fill: {
        primary: accentFor("--st-accent-line-rgb"),
        "brand-primary": accentFor("--st-accent-line-rgb"),
      },
      stroke: {
        primary: accentFor("--st-accent-line-rgb"),
        "brand-primary": accentFor("--st-accent-line-rgb"),
      },
      fontFamily: {
        mono: [
          "Geist Mono",
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Consolas",
          "monospace",
        ],
        display: [
          "Archivo",
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Roboto",
          "Helvetica Neue",
          "Arial",
          "sans-serif",
        ],
        sans: [
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Roboto",
          "Helvetica Neue",
          "Arial",
          "sans-serif",
        ],
      },
    },
    colors: {
      transparent: "transparent",
      current: "currentColor",
      // Themeable: "white" is the ink every surface is drawn
      // against and "black" the deepest surface, so a theme swaps them as a
      // pair. The default values are exactly Tailwind's white and black.
      white: "rgb(var(--st-white-rgb) / <alpha-value>)",
      gray: colors.gray,
      black: "rgb(var(--st-black-rgb) / <alpha-value>)",
      rose: colors.rose,
      indigo: colors.indigo,
      red: { ...colors.red, DEFAULT: "#D33242" },
      blue: { ...colors.blue, DEFAULT: "#2E70FF" },
      orange: colors.orange,
      amber: colors.amber,
      green: { ...colors.green, DEFAULT: "#308752" },
      emerald: colors.emerald,
      teal: colors.teal,
      cyan: colors.cyan,
      yellow: colors.yellow,
      pink: colors.pink,
      purple: colors.purple,
      primary: primaryScale,
      danger: {
        // copper rust — aligned with artcraft app's tailwind.config.ts
        DEFAULT: "#8f4951",
        50: "#fbf5f5",
        100: "#f7ecec",
        200: "#efdcdc",
        300: "#e2c0bf",
        400: "#d19b9c",
        500: "#bd7679",
        600: "#a5595f",
        700: "#8f4951",
        800: "#743d45",
        900: "#64373f",
        950: "#361b1e",
      },
      "brand-primary": primaryScale,
      "brand-secondary": {
        DEFAULT: "#3E3E41",
        50: "#f6f6f6",
        100: "#e7e7e7",
        200: "#d1d1d1",
        300: "#b0b0b0",
        400: "#888888",
        500: "#6d6d6d",
        600: "#5d5d5d",
        700: "#4f4f4f",
        800: "#454545",
        900: "#3d3d3d",
        950: "#3E3E41",
      },
      "brand-tertiary": {
        DEFAULT: "#1cb6be",
        50: "#eefdfc",
        100: "#d4f9f8",
        200: "#aef3f3",
        300: "#62e4e6",
        400: "#38d3d8",
        500: "#1cb6be",
        600: "#1a94a0",
        700: "#1c7682",
        800: "#1f616b",
        900: "#1e505b",
        950: "#0e353e",
      },
      action: {
        DEFAULT: "#3E3E41",
        50: "#f6f6f6",
        100: "#e7e7e7",
        200: "#d1d1d1",
        300: "#b0b0b0",
        400: "#888888",
        500: "#6d6d6d",
        600: "#5d5d5d",
        700: "#4f4f4f",
        800: "#454545",
        900: "#3d3d3d",
        950: "#3E3E41",
      },
      secondary: {
        DEFAULT: "#3E3E41",
        50: "#f6f6f6",
        100: "#e7e7e7",
        200: "#d1d1d1",
        300: "#b0b0b0",
        400: "#888888",
        500: "#6d6d6d",
        600: "#5d5d5d",
        700: "#4f4f4f",
        800: "#454545",
        900: "#3d3d3d",
        950: "#3E3E41",
      },
      "base-fg": "rgb(var(--st-text-rgb) / <alpha-value>)",
      sidebar: {
        DEFAULT: "var(--sidebar-bg)",
        foreground: "var(--sidebar-fg)",
        border: "var(--sidebar-border)",
        accent: "var(--sidebar-accent)",
        "accent-foreground": "var(--sidebar-accent-fg)",
        ring: "var(--sidebar-ring)",
      },
      ui: {
        // Brutalist system: near-black surfaces with white/15 hairline
        // borders, aligned with artcraft-website + artcraft-website-next.
        background: "rgb(var(--st-bg-rgb) / <alpha-value>)",
        panel: "rgb(var(--st-panel-rgb) / <alpha-value>)",
        "panel-border": "rgb(var(--st-white-rgb) / 0.15)",
        border: "rgb(var(--st-white-rgb) / 0.15)",
        // Raised controls stay distinct from the website-matched panel surface.
        controls: "rgb(var(--st-control-rgb) / <alpha-value>)",
        "controls-button": "rgb(var(--st-control-rgb) / <alpha-value>)",
        "controls-border": "rgb(var(--st-white-rgb) / 0.15)",
        divider: "rgb(var(--st-white-rgb) / 0.15)",
        modal: "rgb(var(--st-panel-rgb) / <alpha-value>)",
        ink: "rgb(var(--st-text-rgb) / <alpha-value>)",
        "accent-ink": "rgb(var(--st-accent-ink-rgb) / <alpha-value>)",
        sunken: "rgb(var(--st-sunken-rgb) / <alpha-value>)",
      },
      "axis-x": "#D33242",
      "axis-y": "#308752",
      "axis-z": "#2E70FF",

      // shadcn tokens consumed by @storyteller/ui-video-editor. CSS
      // variables live in apps/artcraft-webapp/src/styles.css.
      background: "hsl(var(--background))",
      foreground: "hsl(var(--foreground))",
      card: {
        DEFAULT: "hsl(var(--card))",
        foreground: "hsl(var(--card-foreground))",
      },
      popover: {
        DEFAULT: "hsl(var(--popover))",
        foreground: "hsl(var(--popover-foreground))",
      },
      muted: {
        DEFAULT: "hsl(var(--muted))",
        foreground: "hsl(var(--muted-foreground))",
      },
      accent: {
        DEFAULT: "hsl(var(--accent))",
        foreground: "hsl(var(--accent-foreground))",
      },
      destructive: {
        DEFAULT: "hsl(var(--destructive))",
        foreground: "hsl(var(--destructive-foreground))",
      },
      border: "hsl(var(--border))",
      input: "hsl(var(--input))",
      ring: "hsl(var(--ring))",
    },
  },
  plugins: [keepDark],
};
