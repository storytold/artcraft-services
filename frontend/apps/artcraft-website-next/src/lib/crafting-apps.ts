import type { Tip } from "./campaign-data";
import {
  CRAFT_APP_RELEASES,
  type CraftRelease,
  type CraftReleaseAsset,
} from "./crafting-app-releases";
import { mediaUrl } from "./links";

// The Crafting Apps family: open-source native apps from the ArtCraft team,
// each in its own storytold/<slug> repo. One entry drives the /apps hub card,
// the /apps/<slug> page, the nav, footer, sitemap, ruler and press kit.
//
// Copy rule: describe the category, never name third-party products.
// Facts (counts, timings, formats) come from each repo's README — keep them
// in sync when the READMEs change.
//
// Download versions and files live in crafting-app-releases.ts. Apps whose
// release there is null show a Discord waitlist instead of installers.
//
// Assets live in public/images/apps/<slug>/: icon.webp (the repo's 512 px
// hicolor render at 256 px), og.jpg (1200×630 share card) and the shots.

export type CraftAppSlug =
  | "photocraft"
  | "vectorcraft"
  | "filmcraft"
  | "lightcraft"
  | "pdfcraft"
  | "effectcraft"
  | "designcraft";

export type CraftPlatform = "macOS" | "Windows" | "Linux" | "Web";

export type CraftShot = {
  /** File under public/images/apps/<slug>/. */
  file: string;
  /** Full-resolution original in the repo's docs/images/ (press kit). */
  source: string;
  caption: string;
  alt: string;
  /** Portrait page (7:9) rather than a 16:10 window capture. */
  portrait?: boolean;
};

export type CraftApp = {
  slug: CraftAppSlug;
  /** Prefix before "Craft" — the wordmark colors "Craft". */
  prefix: string;
  category: string;
  status: string;
  /** Headline split around its one color-accented contrast word. */
  headline: [before: string, accent: string, after: string];
  /** One line for cards and share text. */
  pitch: string;
  lede: string;
  /** schema.org applicationCategory. */
  schemaCategory: "DesignApplication" | "MultimediaApplication" | "BusinessApplication";
  platforms: CraftPlatform[];
  rustVersion: string;
  features: Tip[];
  /** First shot is the hero. */
  shots: [CraftShot, ...CraftShot[]];
};

/** One release file, resolved to a URL for a specific app. */
export type CraftDownload = Omit<CraftReleaseAsset, "file"> & {
  fileName: string;
  href: string;
};

export const CRAFTING_APPS_GITHUB_ORG = "https://github.com/storytold";

export const CRAFTING_APPS: CraftApp[] = [
  {
    slug: "photocraft",
    prefix: "Photo",
    category: "Image editor",
    status: "Early alpha",
    headline: ["The image editor you already ", "know", " how to use."],
    pitch: "The open-source image editor you already know how to use.",
    lede: "Layers, masks, adjustment layers, layer styles, type, vectors and brushes in a native app written entirely in Rust. Open source, offline, and yours.",
    schemaCategory: "DesignApplication",
    platforms: ["macOS", "Windows", "Linux", "Web"],
    rustVersion: "1.90",
    features: [
      {
        title: "Familiar by design",
        body: "The menus, shortcuts, panels and tools your hands already know. Productive on day one, no retraining.",
      },
      {
        title: "Native GPU canvas",
        body: "Renders on Metal, Vulkan, DirectX 12 and WebGPU. No Electron and no web view, just a fast native compositor.",
      },
      {
        title: "Real layered files",
        body: "Opens and saves layered PSD and PSB documents. 134 of 135 real-world test files round-trip byte for byte.",
      },
      {
        title: "Non-destructive editing",
        body: "16 adjustment layers, 70+ live-preview filters, smart objects with smart filters, and a full set of layer styles.",
      },
      {
        title: "On-device selection",
        body: "Select subjects and objects and refine masks with tools that run on your machine, not someone else's server.",
      },
      {
        title: "Serious color",
        body: "8, 16 and 32-bit documents in RGB, CMYK, Lab and Grayscale, with ICC color management and soft proofing.",
      },
    ],
    shots: [
      {
        file: "hero.webp",
        source: "photocraft-demo.jpg",
        caption: "Curves and type layers — The Great Wave",
        alt: "PhotoCraft editing The Great Wave off Kanagawa with a Curves adjustment and type layers in the Layers panel",
      },
      {
        file: "adjustments.webp",
        source: "photocraft-adjustments.jpg",
        caption: "Levels with a live histogram",
        alt: "PhotoCraft applying a Levels adjustment layer to Impression, Sunrise with the histogram panel open",
      },
      {
        file: "layer-styles.webp",
        source: "photocraft-layer-styles.jpg",
        caption: "Layer styles on live type",
        alt: "PhotoCraft's Layer Style dialog adding an outer glow and stroke to the word Earthrise",
      },
      {
        file: "masks.webp",
        source: "photocraft-masks.jpg",
        caption: "Selections and masked adjustments",
        alt: "PhotoCraft with an elliptical selection and a Hue/Saturation adjustment on Girl with a Pearl Earring",
      },
    ],
  },
  {
    slug: "vectorcraft",
    prefix: "Vector",
    category: "Vector illustration",
    status: "In development",
    headline: ["Vector illustration, ", "reimagined", " in pure Rust."],
    pitch: "Fast, open-source vector illustration, reimagined in pure Rust.",
    lede: "The pen, panels and shortcuts working illustrators expect, in a native app that keeps up with you. Open source, and in your browser too.",
    schemaCategory: "DesignApplication",
    platforms: ["macOS", "Windows", "Linux", "Web"],
    rustVersion: "1.90",
    features: [
      {
        title: "The workflow you know",
        body: "Pen, direct selection, shape booleans, snapping guides and an appearance panel, laid out the way you expect.",
      },
      {
        title: "Genuinely fast",
        body: "20,000 shapes render in about 27 ms at retina resolution, and the interface runs at 120 fps.",
      },
      {
        title: "Booleans that never fail",
        body: "Exact curve booleans mean no more “cannot perform operation”. Unlimited undo means no more fear.",
      },
      {
        title: "Live, editable effects",
        body: "Blends, envelope distort, gradient mesh, live radial, grid and mirror repeats, glows and drop shadows — all still editable.",
      },
      {
        title: "Image tracing",
        body: "Turn raster images into clean vector paths with 12 tracing presets.",
      },
      {
        title: "Open formats",
        body: "Native JSON documents, SVG and PDF import and export, PNG, JPEG and WebP output, and multi-size export for screens.",
      },
    ],
    shots: [
      {
        file: "hero.webp",
        source: "shot-1-neon.png",
        caption: "Neon Drive — made in VectorCraft",
        alt: "VectorCraft editing the Neon Drive poster, with the title's live outer glow settings in the Appearance panel and its character settings in the Properties panel",
      },
      {
        file: "ribbons.webp",
        source: "shot-2-ribbons.png",
        caption: "Live blends and layers",
        alt: "VectorCraft showing three live blend ribbons clipped to the artboard, one selected with its key paths, and the Layers panel listing the clip group and blends",
      },
      {
        file: "bezier.webp",
        source: "shot-4-bezier.png",
        caption: "Pen and direct selection",
        alt: "VectorCraft direct-selecting the anchor points and bezier handles of a crescent moon, with the contextual task bar below it",
      },
      {
        file: "perspective.webp",
        source: "shot-5-perspective.png",
        caption: "Perspective grid, still editable",
        alt: "VectorCraft with two lit building facades drawn on the left and right planes of a two-point perspective grid at sunset",
      },
      {
        file: "sheet.webp",
        source: "shot-3-sheet.png",
        caption: "Booleans, mesh, repeat and envelope",
        alt: "VectorCraft in its light theme with four artboards: shape booleans, a gradient mesh, a live radial repeat and an envelope distort",
      },
    ],
  },
  {
    slug: "filmcraft",
    prefix: "Film",
    category: "Video editor",
    status: "In development",
    headline: ["Professional video editing, ", "rebuilt", " from scratch."],
    pitch: "Professional, open-source video editing, rebuilt from scratch in Rust.",
    lede: "Source and program monitors, a real timeline and every trim mode, in a native editor written in pure Rust — down to its own codecs.",
    schemaCategory: "MultimediaApplication",
    platforms: ["macOS", "Windows", "Linux"],
    rustVersion: "1.95",
    features: [
      {
        title: "A timeline you already know",
        body: "Source and program monitors, three-point editing, every trim mode, and J/K/L dynamic trimming.",
      },
      {
        title: "Frame-exact time",
        body: "An integer timebase keeps 23.976 and 29.97 drop-frame exact. No drift, no rounding, no mystery frames.",
      },
      {
        title: "Its own codecs",
        body: "No FFmpeg inside. Native H.264, HEVC, ProRes, VP9, AAC and Opus, with bit-exact decoders.",
      },
      {
        title: "Color grading",
        body: "Curves, color wheels, HSL secondaries, LUTs, HDR and log workflows, plus scopes to check your work.",
      },
      {
        title: "Effects and keyframes",
        body: "About 55 video effects and 30 transitions, keyframes with value and velocity graphs, and a linear-light GPU compositor.",
      },
      {
        title: "Broadcast-ready audio",
        body: "EBU R128 loudness meters, a mixer with automation, and native DSP effects.",
      },
    ],
    shots: [
      {
        file: "color.webp",
        source: "filmcraft-color.png",
        caption: "Scopes and timeline — Charade (1963)",
        alt: "FilmCraft with video scopes, the program monitor showing Charade (1963), and a multi-track timeline",
      },
      {
        file: "hero.webp",
        source: "filmcraft-hero.png",
        caption: "Effect controls — Night of the Living Dead (1968)",
        alt: "FilmCraft's effect controls and timeline editing Night of the Living Dead (1968)",
      },
      {
        file: "keyframes.webp",
        source: "filmcraft-keyframes.png",
        caption: "Keyframes with value and velocity graphs",
        alt: "FilmCraft keyframing scale with value and velocity graphs on a title shot",
      },
      {
        file: "export.webp",
        source: "filmcraft-export.png",
        caption: "Export with destination presets",
        alt: "FilmCraft export screen with H.264 settings and destination presets",
      },
    ],
  },
  {
    slug: "lightcraft",
    prefix: "Light",
    category: "Photo library & raw developer",
    status: "In development",
    headline: ["Your photos. Your pixels. Your ", "machine", "."],
    pitch: "An open-source photo library and raw developer that runs on your machine.",
    lede: "A fast photo library and non-destructive raw developer, written from scratch in pure Rust. No account, no cloud, no telemetry, no subscription.",
    schemaCategory: "MultimediaApplication",
    platforms: ["macOS", "Windows", "Linux", "Web"],
    rustVersion: "1.90",
    features: [
      {
        title: "Non-destructive by design",
        body: "A scene-referred, wide-gamut, 32-bit float pipeline. Your originals are never touched.",
      },
      {
        title: "Every control you reach for",
        body: "Light, color, effects, tone curve, an 8-band color mixer, 3-way color grading wheels and 18 built-in presets.",
      },
      {
        title: "Precise masking",
        body: "Brush, linear and radial gradients, luminance and color ranges, plus sky, subject and background masks.",
      },
      {
        title: "Native raw decoding",
        body: "Pure-Rust decoders for DNG, CR2, ARW, NEF, RAF (including X-Trans), RW2, PEF and ORF, with more on the way.",
      },
      {
        title: "A library that keeps up",
        body: "Albums, ratings, flags, field search and virtualized grids built for big catalogs.",
      },
      {
        title: "Instant feedback",
        body: "The GPU develop pipeline updates a 24 MP raw in about 4 ms per slider move.",
      },
    ],
    shots: [
      {
        file: "hero.webp",
        source: "hero-tetons.jpg",
        caption: "Develop — The Tetons and the Snake River",
        alt: "LightCraft developing Ansel Adams' The Tetons and the Snake River with light and effects sliders",
      },
      {
        file: "grading.webp",
        source: "grading-migrant-mother.jpg",
        caption: "Color grading wheels — Migrant Mother",
        alt: "LightCraft color grading wheels applied to Dorothea Lange's Migrant Mother",
      },
      {
        file: "masking.webp",
        source: "masking.jpg",
        caption: "Sky and radial gradient masks",
        alt: "LightCraft masking panel with a radial gradient mask warming a sunset sky",
      },
      {
        file: "library.webp",
        source: "grid-demo.jpg",
        caption: "Library with albums and ratings",
        alt: "LightCraft library grid with albums, ratings and the develop panel",
      },
    ],
  },
  {
    slug: "pdfcraft",
    prefix: "Pdf",
    category: "PDF workbench",
    status: "Early alpha",
    headline: ["The ", "open-source", " PDF workbench."],
    pitch: "The open-source PDF workbench: read, organize, combine, split and secure.",
    lede: "Read, organize, combine, split and secure PDFs in a fast, native app written in Rust from the ground up. No account, no telemetry, no cloud.",
    schemaCategory: "BusinessApplication",
    platforms: ["macOS", "Windows", "Linux", "Web"],
    rustVersion: "1.90",
    features: [
      {
        title: "Faithful rendering",
        body: "World scripts, vertical Japanese, color emoji and transparency. Zero crashes across a 983-file test corpus.",
      },
      {
        title: "Search as you type",
        body: "Instant find, and text selection that follows the document's real reading order.",
      },
      {
        title: "Organize like cards",
        body: "Rotate, delete, insert and reorder pages, with deep undo behind every change.",
      },
      {
        title: "Combine, extract, split",
        body: "Assemble and break apart documents while keeping links, form fields, layers and bookmarks intact.",
      },
      {
        title: "Fearless saves",
        body: "Incremental, atomic saves with autosave and crash recovery.",
      },
      {
        title: "Built-in security",
        body: "Opens everything from RC4 to AES-256 encryption and honors author permissions.",
      },
    ],
    shots: [
      {
        file: "hero.webp",
        source: "pdfcraft-viewer.png",
        caption: "Viewer with threaded comments",
        alt: "PdfCraft showing a showcase PDF cover with the All tools panel and threaded comments",
      },
      {
        file: "organize.webp",
        source: "pdfcraft-organize.png",
        caption: "Organize pages like cards",
        alt: "PdfCraft page organizer with three pages selected and the page tools in the toolbar above",
      },
      {
        file: "palette.webp",
        source: "pdfcraft-palette.png",
        caption: "A command palette for every tool",
        alt: "PdfCraft command palette searching for page and listing the page tools",
      },
      {
        file: "twoup.webp",
        source: "pdfcraft-twoup.png",
        caption: "Two-up Read mode, dark theme",
        alt: "PdfCraft two-up Read mode in the dark theme showing typeset specimen pages",
      },
    ],
  },
  {
    slug: "effectcraft",
    prefix: "Effect",
    category: "Motion graphics & VFX",
    status: "In development",
    headline: ["Motion graphics and visual ", "effects", ", in pure Rust."],
    pitch: "Open-source motion graphics and visual effects, built in pure Rust.",
    lede: "Compositions, layers, keyframes, 259 effects, expressions, 3D cameras and lights, and a render queue in a native compositor written in pure Rust. Young, moving fast, and already usable.",
    schemaCategory: "MultimediaApplication",
    platforms: ["macOS", "Windows", "Linux", "Web"],
    rustVersion: "1.95",
    features: [
      {
        title: "Animate the way you know",
        body: "Familiar panels and keyframes: linear, bezier, hold and eased keys, roving keys, and a graph editor with value and speed curves.",
      },
      {
        title: "Layers of every kind",
        body: "Solids, shapes, text, footage, nested compositions, nulls, adjustment layers, cameras and lights, with parenting, track mattes and 38 blend modes.",
      },
      {
        title: "259 effects",
        body: "Blur, color correction, distortion, generators, keying, particles and simulation, stylize and time effects, each with sensible defaults.",
      },
      {
        title: "Real 3D",
        body: "3D layers with cameras, depth of field and lights that cast soft ray-traced shadows, plus orbit, pan and dolly tools.",
      },
      {
        title: "Expressions",
        body: "JavaScript expressions with wiggle, loops and layer references, edited inline and linked straight to any property.",
      },
      {
        title: "Export anywhere",
        body: "H.264 and ProRes with alpha, PNG, TIFF and 32-bit EXR sequences, GIF, and Lottie import and export for the web. No FFmpeg inside.",
      },
    ],
    shots: [
      {
        file: "hero.webp",
        source: "effectcraft-hero.png",
        caption: "An animated title on the timeline",
        alt: "EffectCraft's composition panel showing an animated EFFECTCRAFT title, with the project panel, a timeline of text, shape and solid layers, and the properties panel",
      },
      {
        file: "graph-editor.webp",
        source: "effectcraft-graph-editor.png",
        caption: "Graph editor with eased keyframes",
        alt: "EffectCraft's graph editor showing an eased value curve for a text animator",
      },
      {
        file: "effects.webp",
        source: "effectcraft-effects.png",
        caption: "Particle simulation effects",
        alt: "EffectCraft's effect controls for a particle simulation bursting from the center of the composition",
      },
      {
        file: "3d.webp",
        source: "effectcraft-3d.png",
        caption: "3D cameras, lights and soft shadows",
        alt: "EffectCraft's 3D showcase: intersecting cards lit by a spot light with soft shadows on a gridded floor, seen from a custom camera view",
      },
    ],
  },
  {
    slug: "designcraft",
    prefix: "Design",
    category: "Page layout & publishing",
    status: "In development",
    headline: ["Page layout and ", "publishing", ", rebuilt in pure Rust."],
    pitch: "Open-source page layout and publishing, rebuilt in pure Rust.",
    lede: "Spreads and parent pages, threaded stories, styles, swatches and text wrap in a native layout app with a professional paragraph composer. No subscription, no licence server, no telemetry.",
    schemaCategory: "DesignApplication",
    platforms: ["macOS", "Windows", "Linux", "Web"],
    rustVersion: "1.90",
    features: [
      {
        title: "The layout tools you know",
        body: "Spreads and parent pages, frames and threaded stories, paragraph and character styles, swatches and text wrap, laid out the way you expect.",
      },
      {
        title: "Beautiful type",
        body: "A Knuth–Plass paragraph composer, dictionary hyphenation, optical margin alignment and baseline grids. Line breaks match on screen and in PDF.",
      },
      {
        title: "Print-ready PDF",
        body: "Real selectable text with embedded fonts, CMYK and spot colors, bleed, crop marks and archival PDF/A output.",
      },
      {
        title: "Fast",
        body: "Multithreaded SIMD rendering, copy-on-write documents with instant undo, and cached composition.",
      },
      {
        title: "Open formats",
        body: "A documented native format, PNG export, and layout interchange import and export for moving work between tools.",
      },
      {
        title: "Agent-native",
        body: "Every menu item, tool, panel and dialog can be driven over a JSON control channel and an MCP server.",
      },
    ],
    shots: [
      {
        file: "hero.webp",
        source: "ui-spread.png",
        caption: "Threaded columns and text wrap — Quarterly",
        alt: "DesignCraft showing a magazine spread: a threaded three-column story with its thread line, a wrapped pull quote, and the Properties panel's text frame options",
      },
      {
        file: "cover.webp",
        source: "page-cover.png",
        caption: "Cover",
        alt: "Magazine cover for The Spring Issue: a dusk landscape of layered purple hills under a pale sun, with the serif headline The Quiet Art of Layout",
        portrait: true,
      },
      {
        file: "feature.webp",
        source: "page-2.png",
        caption: "Styles",
        alt: "Feature opener titled Notes on the Grid with a kicker rule, italic deck, landscape picture, caption and two columns of justified body text",
        portrait: true,
      },
      {
        file: "columns.webp",
        source: "page-3.png",
        caption: "Threading and wrap",
        alt: "Three columns of justified, hyphenated body text wrapping around a shaded pull quote",
        portrait: true,
      },
      {
        file: "swatches.webp",
        source: "page-4.png",
        caption: "Swatches",
        alt: "Coming Next page titled Color, Ink and Paper on a plum background with four labeled color swatches",
        portrait: true,
      },
    ],
  },
];

const COUNT_WORDS = ["Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten"];

/** "Seven" — the family size as a capitalized word, for headlines. */
export const CRAFTING_APPS_COUNT_WORD =
  COUNT_WORDS[CRAFTING_APPS.length] ?? String(CRAFTING_APPS.length);

const LIST_FORMAT = new Intl.ListFormat("en", { type: "conjunction" });

/** "a, b and c" — Oxford comma per the en locale. */
export function formatList(items: string[]): string {
  return LIST_FORMAT.format(items);
}

const BROWSER_APP_NAMES = CRAFTING_APPS.filter((app) =>
  app.platforms.includes("Web"),
).map(craftAppName);

// Family-wide rules, shown on the /apps hub.
export const CRAFTING_APPS_PRINCIPLES: Tip[] = [
  {
    title: "Open source",
    body: "Every line is on GitHub under permissive licenses. Read it, fork it, build on it.",
  },
  {
    title: "Native, not wrapped",
    body: "Pure Rust compiled to real desktop apps for macOS, Windows and Linux. No Electron, no web views.",
  },
  {
    title: "Familiar from day one",
    body: "Layouts, tools and shortcuts that working professionals already know, so there's nothing to relearn.",
  },
  {
    title: "Your files, your machine",
    body: "Everything runs locally on your own files. No cloud round-trips between you and your work.",
  },
  {
    title: "Agent-ready",
    body: "Drive every app from a CLI, a JSON control channel or an MCP server, built for automation and AI agents.",
  },
  {
    title: "Also in your browser",
    body: `${formatList(BROWSER_APP_NAMES)} also compile to WebAssembly and run in a browser tab.`,
  },
];

export const CRAFTING_APPS_OG_IMAGE = mediaUrl("/images/apps/og.jpg");

export function getCraftApp(slug: string): CraftApp | undefined {
  return CRAFTING_APPS.find((app) => app.slug === slug);
}

export function craftAppName(app: CraftApp): string {
  return `${app.prefix}Craft`;
}

/** Two-digit lineup position, e.g. "03". */
export function craftAppIndex(app: CraftApp): string {
  return String(CRAFTING_APPS.indexOf(app) + 1).padStart(2, "0");
}

export function craftAppPath(app: CraftApp): string {
  return `/apps/${app.slug}`;
}

export function craftAppRepo(app: CraftApp): string {
  return `${CRAFTING_APPS_GITHUB_ORG}/${app.slug}`;
}

/** Site-relative path; use craftShotUrl for an <img> src. */
export function craftShotPath(app: CraftApp, shot: CraftShot): string {
  return `/images/apps/${app.slug}/${shot.file}`;
}

export function craftShotUrl(app: CraftApp, shot: CraftShot): string {
  return mediaUrl(craftShotPath(app, shot));
}

export function craftShotSourceUrl(app: CraftApp, shot: CraftShot): string {
  return `https://raw.githubusercontent.com/storytold/${app.slug}/main/docs/images/${shot.source}`;
}

export function craftAppIconPath(app: CraftApp): string {
  return `/images/apps/${app.slug}/icon.webp`;
}

export function craftAppIconUrl(app: CraftApp): string {
  return mediaUrl(craftAppIconPath(app));
}

/** The repo's 1024 px icon render (press kit download). */
export function craftAppIconSourceUrl(app: CraftApp): string {
  return `https://raw.githubusercontent.com/storytold/${app.slug}/main/assets/app-icon/${app.slug}-1024.png`;
}

export function craftAppRelease(app: CraftApp): CraftRelease | null {
  return CRAFT_APP_RELEASES[app.slug];
}

export function craftReleaseTag(release: CraftRelease): string {
  return release.tag ?? `v${release.version}`;
}

export function craftReleasePageUrl(app: CraftApp, release: CraftRelease): string {
  return `${craftAppRepo(app)}/releases/tag/${craftReleaseTag(release)}`;
}

export function craftReleasesUrl(app: CraftApp): string {
  return `${craftAppRepo(app)}/releases`;
}

/** Every file in the release, in catalog order. */
export function craftReleaseDownloads(
  app: CraftApp,
  release: CraftRelease,
): CraftDownload[] {
  return releaseDownloads(craftAppRepo(app), app.slug, release);
}

/** Every file in a release of `repo`, in catalog order; `{slug}` defaults to `fileSlug`. */
export function releaseDownloads(
  repo: string,
  fileSlug: string,
  release: CraftRelease,
): CraftDownload[] {
  const base = `${repo}/releases/download/${craftReleaseTag(release)}`;
  return release.assets.map(({ file, ...asset }) => {
    const fileName = file
      .replaceAll("{slug}", release.fileSlug ?? fileSlug)
      .replaceAll("{version}", release.version);
    return { ...asset, fileName, href: `${base}/${fileName}` };
  });
}

export function craftAppOgImage(app: CraftApp): string {
  return mediaUrl(`/images/apps/${app.slug}/og.jpg`);
}

export function craftAppBuildCommand(app: CraftApp): string {
  return [
    `git clone ${craftAppRepo(app)}.git`,
    `cd ${app.slug}`,
    `cargo run --release -p ${app.slug}`,
  ].join("\n");
}
