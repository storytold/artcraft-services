import { ArrowUpRightIcon, CheckIcon } from "lucide-react";
import { twMerge } from "tailwind-merge";
import {
  CRAFT_LAUNCHER_NAME,
  CRAFT_LAUNCHER_REPO,
  craftLauncherDownloads,
  craftLauncherReleasePageUrl,
} from "@/lib/craft-launcher";
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

// Leads an app's Get-it section, above that app's own installers. Renders
// nothing while the launcher has no release.
export function LauncherCallout({ app }: { app: CraftApp }) {
  const recommended = craftLauncherRecommended();
  if (recommended.length === 0) return null;
  const name = craftAppName(app);
  return (
    <article data-reveal className="bg-bg">
      <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-10">
        <p className="hud-label text-(--app-ink,var(--accent-ink))">Recommended</p>
        <p className="hud-label text-faint">{CRAFT_LAUNCHER_NAME}</p>
      </div>
      <div className="flex flex-col gap-8 p-6 md:p-10 lg:flex-row lg:items-center lg:gap-10">
        <LauncherIcon className="h-16 w-16 md:h-20 md:w-20" />
        <div className="min-w-0 flex-1">
          <h3 className="font-display text-3xl font-medium leading-[1.05] tracking-[-0.03em] text-ink-strong sm:text-4xl">
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
        <div className="flex shrink-0 flex-col items-start gap-3">
          <LauncherDownloadButton
            recommended={recommended}
            fallbackHref={craftLauncherReleasePageUrl()}
          />
          <a
            href={CRAFT_LAUNCHER_REPO}
            target="_blank"
            rel="noopener noreferrer"
            className="hud-label flex items-center gap-1.5 text-muted hover:text-ink"
          >
            About the Launcher
            <ArrowUpRightIcon aria-hidden className="h-3.5 w-3.5" />
          </a>
        </div>
      </div>
    </article>
  );
}

// The ArtCraft mark on an app-icon tile, so it sits beside the Crafting App
// icons. Decorative — it always sits beside the launcher's name.
export function LauncherIcon({ className }: { className?: string }) {
  return (
    <span
      aria-hidden
      className={twMerge(
        "flex h-14 w-14 shrink-0 items-center justify-center rounded-[22%] border border-line bg-bg-raised shadow-[0_6px_14px_rgba(0,0,0,0.18)]",
        className,
      )}
    >
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img src="/artcraft-icon.svg" alt="" className="h-[52%] w-[52%]" />
    </span>
  );
}
