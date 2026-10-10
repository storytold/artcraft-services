import { ArrowUpRightIcon, CheckIcon } from "lucide-react";
import {
  CRAFT_LAUNCHER_NAME,
  CRAFT_LAUNCHER_REPO,
  craftLauncherDownloads,
  craftLauncherReleasePageUrl,
} from "@/lib/craft-launcher";
import { CRAFT_LAUNCHER_RELEASE } from "@/lib/crafting-app-releases";
import { craftAppName, type CraftApp, type CraftDownload } from "@/lib/crafting-apps";
import { LauncherDownloadButton } from "./app-downloads";
import { ColorAccent } from "./app-wordmark";

// ArtCraft Launcher on the Crafting Apps pages: the "Recommended" callout
// that leads every app's Get-it section. The /apps hub header uses
// craftLauncherRecommended for its main download button. Server component;
// the download button is the client island.

const CALLOUT_POINTS = ["Every Crafting App", "One-click updates", "Verified downloads"];

/** The launcher's main download per platform, for the OS-aware buttons. */
export function craftLauncherRecommended(): CraftDownload[] {
  return craftLauncherDownloads().filter((download) => download.recommended);
}

// Leads an app's Get-it section, above that app's own installers: the pitch
// beside a sunken download panel, matching the release and build-from-source
// cells below it. Renders nothing while the launcher has no release.
export function LauncherCallout({ app }: { app: CraftApp }) {
  const recommended = craftLauncherRecommended();
  if (!CRAFT_LAUNCHER_RELEASE || recommended.length === 0) return null;
  const name = craftAppName(app);
  return (
    <div
      data-reveal
      className="grid gap-px bg-line lg:grid-cols-[minmax(0,7fr)_minmax(0,4fr)]"
    >
      <div className="flex flex-col bg-bg p-6 md:p-10">
        <div className="flex items-center gap-3">
          <LauncherMark />
          <p className="hud-label text-(--app-ink,var(--accent-ink))">Recommended</p>
        </div>
        <h3 className="mt-6 font-display text-3xl font-medium leading-[1.05] tracking-[-0.03em] text-ink-strong sm:text-4xl">
          Get {name} with the <ColorAccent>Launcher</ColorAccent>.
        </h3>
        <p className="mt-4 max-w-xl leading-relaxed text-muted">
          {CRAFT_LAUNCHER_NAME} installs {name} and every other Crafting App
          from one place, picks the right build for your computer and keeps
          them all up to date. Install it once and every update is a click
          away.
        </p>
        <ul className="mt-6 flex flex-wrap gap-x-6 gap-y-2">
          {CALLOUT_POINTS.map((point) => (
            <li key={point} className="hud-label flex items-center gap-1.5 text-muted">
              <CheckIcon aria-hidden className="h-3.5 w-3.5 text-(--app-ink,var(--accent-ink))" />
              {point}
            </li>
          ))}
        </ul>
      </div>
      <div className="flex flex-col justify-center bg-bg-sunken p-6 md:p-10">
        <p className="hud-label text-faint">
          {CRAFT_LAUNCHER_NAME} · v{CRAFT_LAUNCHER_RELEASE.version}
        </p>
        <LauncherDownloadButton
          recommended={recommended}
          fallbackHref={craftLauncherReleasePageUrl()}
          fullWidth
          className="mt-4"
        />
        <a
          href={CRAFT_LAUNCHER_REPO}
          target="_blank"
          rel="noopener noreferrer"
          className="hud-label mt-4 flex w-fit items-center gap-1.5 text-muted hover:text-ink"
        >
          About the Launcher
          <ArrowUpRightIcon aria-hidden className="h-3.5 w-3.5" />
        </a>
      </div>
    </div>
  );
}

// The ArtCraft mark in a hairline square, like the platform icons in the
// download cards. Decorative — it always sits beside the launcher's name.
function LauncherMark() {
  return (
    <span
      aria-hidden
      className="flex h-10 w-10 shrink-0 items-center justify-center border border-line"
    >
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img src="/artcraft-icon.svg" alt="" className="h-5 w-5" />
    </span>
  );
}
