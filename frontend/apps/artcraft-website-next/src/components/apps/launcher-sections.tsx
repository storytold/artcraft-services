import Link from "next/link";
import type { ReactNode } from "react";
import {
  AppWindowIcon,
  ArrowDownIcon,
  ArrowUpRightIcon,
  CheckIcon,
  PackageCheckIcon,
  RefreshCwIcon,
} from "lucide-react";
import { twMerge } from "tailwind-merge";
import { TipGrid } from "@/components/campaign/sections";
import { GitHubIcon } from "@/components/icons";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import {
  CRAFT_LAUNCHER_DOWNLOADS_ID,
  CRAFT_LAUNCHER_FEATURES,
  CRAFT_LAUNCHER_NAME,
  CRAFT_LAUNCHER_PATH,
  CRAFT_LAUNCHER_REPO,
  CRAFT_LAUNCHER_STATUS,
  craftLauncherDownloads,
  craftLauncherReleasePageUrl,
} from "@/lib/craft-launcher";
import { CRAFT_LAUNCHER_RELEASE } from "@/lib/crafting-app-releases";
import { craftAppName, type CraftApp, type CraftDownload } from "@/lib/crafting-apps";
import { AppPlatformDownloads, LauncherDownloadButton } from "./app-downloads";
import { ColorAccent } from "./app-wordmark";

// ArtCraft Launcher on the Crafting Apps pages: the hub's launcher section
// (pitch beside a download panel, features and per-platform downloads) and the "Recommended" callout that leads every
// app's Get-it section. Server components; the download buttons are the
// client islands.

// Order matches CRAFT_LAUNCHER_FEATURES.
const FEATURE_ICONS = [
  <PackageCheckIcon key="install" aria-hidden className="h-5 w-5" />,
  <RefreshCwIcon key="update" aria-hidden className="h-5 w-5" />,
  <AppWindowIcon key="open" aria-hidden className="h-5 w-5" />,
];

const LAUNCHER_PLATFORMS = [
  "macOS 11 or later · Apple silicon and Intel",
  "Windows 10 and 11 · x64, ARM64 and 32-bit",
  "Linux · x86_64 and ARM64",
];

const CALLOUT_POINTS = ["Every Crafting App", "One-click updates", "Verified downloads"];

/** The launcher's main download per platform, for the OS-aware buttons. */
export function craftLauncherRecommended(): CraftDownload[] {
  return craftLauncherDownloads().filter((download) => download.recommended);
}

// /apps hub, directly under the header: why the launcher beside its
// download, then every platform's installer.
export function LauncherShowcase({ index }: { index: string }) {
  const downloads = craftLauncherDownloads();
  const recommended = downloads.filter((download) => download.recommended);
  return (
    <SectionShell id="launcher">
      <SectionEyebrow
        index={index}
        label={CRAFT_LAUNCHER_NAME}
        annotation="Recommended · One install for every app"
      />
      <div className="grid gap-px bg-line lg:grid-cols-[minmax(0,7fr)_minmax(0,5fr)]">
        <div data-reveal className="flex flex-col bg-bg p-6 md:p-10">
          <div className="flex items-center gap-4">
            <LauncherIcon className="h-14 w-14" />
            <div>
              <p className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
                ArtCraft <ColorAccent>Launcher</ColorAccent>
              </p>
              <p className="hud-label mt-1 text-faint">
                {CRAFT_LAUNCHER_RELEASE
                  ? `Version ${CRAFT_LAUNCHER_RELEASE.version} · ${CRAFT_LAUNCHER_STATUS}`
                  : CRAFT_LAUNCHER_STATUS}
              </p>
            </div>
          </div>
          <h2 className="mt-10 max-w-lg font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl">
            Install once. <ColorAccent>Update</ColorAccent> everything.
          </h2>
          <p className="mt-5 max-w-lg text-lg leading-relaxed text-muted">
            The easiest way to get the Crafting Apps. One small native app
            installs the whole family, keeps every app up to date and opens
            any of them from your menu bar or tray.
          </p>
          <p className="mt-4 max-w-lg leading-relaxed text-muted">
            Prefer to pick and choose? Every app still has its own
            installers on its page, and you can update each one on its own.
          </p>
          <div className="mt-auto flex flex-wrap gap-x-6 gap-y-2 pt-10">
            <ExternalLink href={craftLauncherReleasePageUrl()}>Release notes</ExternalLink>
            <ExternalLink href={CRAFT_LAUNCHER_REPO}>
              <GitHubIcon className="h-3.5 w-3.5" />
              Source on GitHub
            </ExternalLink>
          </div>
        </div>
        <div data-reveal className="flex flex-col justify-center bg-bg-sunken p-6 md:p-10">
          {recommended.length > 0 ? (
            <>
              <p className="hud-label text-(--app-ink,var(--accent-ink))">
                Start here · Free
              </p>
              <LauncherDownloadButton
                recommended={recommended}
                fallbackHref={`#${CRAFT_LAUNCHER_DOWNLOADS_ID}`}
                fullWidth
                className="mt-5"
              />
              <ul className="mt-6 border-t border-line">
                {LAUNCHER_PLATFORMS.map((platform) => (
                  <li
                    key={platform}
                    className="flex items-center justify-between gap-4 border-b border-line py-3"
                  >
                    <span className="text-sm text-ink">{platform}</span>
                    <CheckIcon aria-hidden className="h-4 w-4 text-(--app-ink,var(--accent-ink))" />
                  </li>
                ))}
              </ul>
              <a
                href={`#${CRAFT_LAUNCHER_DOWNLOADS_ID}`}
                className="hud-label mt-6 flex w-fit items-center gap-1.5 text-muted hover:text-ink"
              >
                Every platform and file
                <ArrowDownIcon aria-hidden className="h-3.5 w-3.5" />
              </a>
            </>
          ) : (
            <>
              <p className="hud-label text-faint">Installers</p>
              <p className="mt-4 font-display text-3xl font-medium leading-[1.05] tracking-[-0.03em] text-ink-strong">
                Coming <ColorAccent>soon</ColorAccent>.
              </p>
            </>
          )}
        </div>
      </div>
      <TipGrid items={CRAFT_LAUNCHER_FEATURES} columns={3} icons={FEATURE_ICONS} />
      {downloads.length > 0 && (
        <div id={CRAFT_LAUNCHER_DOWNLOADS_ID} className="scroll-mt-28 border-t border-line">
          <AppPlatformDownloads downloads={downloads} collapseOthers />
        </div>
      )}
    </SectionShell>
  );
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
            fallbackHref={`/apps#${CRAFT_LAUNCHER_DOWNLOADS_ID}`}
          />
          <Link
            href={CRAFT_LAUNCHER_PATH}
            className="hud-label flex items-center gap-1.5 text-muted hover:text-ink"
          >
            About the Launcher
            <ArrowUpRightIcon aria-hidden className="h-3.5 w-3.5" />
          </Link>
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

function ExternalLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="hud-label flex w-fit items-center gap-1.5 text-muted hover:text-ink"
    >
      {children}
      <ArrowUpRightIcon aria-hidden className="h-3.5 w-3.5" />
    </a>
  );
}
