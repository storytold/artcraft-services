"use client";

import { useEffect, useState, type ReactNode } from "react";
import {
  AppleIcon,
  ArrowDownIcon,
  ArrowDownToLineIcon,
  SquareTerminalIcon,
} from "lucide-react";
import { twMerge } from "tailwind-merge";
import { WindowsIcon } from "@/components/icons";
import { Button, type ButtonProps } from "@/components/ui";
import {
  CRAFT_DESKTOP_PLATFORMS,
  type CraftDesktopPlatform,
} from "@/lib/crafting-app-releases";
import type { CraftDownload } from "@/lib/crafting-apps";

// OS-aware downloads for /apps/<slug>, modeled on the ArtCraft /download
// page. Detection runs after mount (no user agent on the server), so the
// first paint treats every platform equally and "Your system" arrives with
// hydration. Phones, tablets and Chromebooks keep the neutral layout.

type DetectedDesktop = {
  platform: CraftDesktopPlatform;
  arch?: CraftDownload["arch"];
};

const PLATFORM_ICONS: Record<CraftDesktopPlatform, ReactNode> = {
  macOS: <AppleIcon aria-hidden className="h-5 w-5" />,
  Windows: <WindowsIcon className="h-5 w-5" />,
  Linux: <SquareTerminalIcon aria-hidden className="h-5 w-5" />,
};

/**
 * One card per desktop platform: its recommended build as the main button,
 * then every other file for that platform. The visitor's platform is tagged
 * "Your system" and gets the solid button. These are the app's standalone
 * installers (the launcher above them installs the whole family), so each
 * button says "Standalone for <platform>" and each file list names the app.
 */
export function AppPlatformDownloads({
  name,
  downloads,
}: {
  name: string;
  downloads: CraftDownload[];
}) {
  const detected = useDetectedDesktop();

  return (
    <div className="grid gap-px bg-line md:grid-cols-3">
      {CRAFT_DESKTOP_PLATFORMS.map((platform, i) => {
        const files = downloads.filter((download) => download.group === platform);
        const isDetected = detected?.platform === platform;
        const main = pickDownload(files, isDetected ? detected : { platform });
        if (!main) return null;
        const others = files.filter((download) => download !== main);

        return (
          <article key={platform} data-reveal className="flex min-w-0 flex-col bg-bg">
            <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-8">
              <p className="hud-label text-muted">
                {isDetected ? (
                  <span className="text-(--app-ink,var(--accent-ink))">Your system</span>
                ) : (
                  "Platform"
                )}
              </p>
              <p className="hud-label text-faint">{String(i + 1).padStart(2, "0")}</p>
            </div>
            <div className="flex flex-1 flex-col p-6 md:p-8">
              <div className="flex items-center gap-3">
                <span className="flex h-10 w-10 items-center justify-center border border-line text-ink">
                  {PLATFORM_ICONS[platform]}
                </span>
                <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
                  {platform}
                </h3>
              </div>
              <p className="mt-5 text-sm text-ink">{main.label}</p>
              <p className="mt-1 text-sm text-muted">{main.description}</p>
              <Button
                href={main.href}
                variant={isDetected ? "primary" : "secondary"}
                className="mt-6 w-full whitespace-normal text-center"
              >
                <ArrowDownToLineIcon aria-hidden className="h-4 w-4" />
                Standalone for {platform}
              </Button>
              {others.length > 0 && (
                <>
                  <p className="hud-label mt-8 text-faint">
                    Other {name} files for {platform}
                  </p>
                  <ul className="mt-2 border-t border-line">
                    {others.map((download) => (
                      <li key={download.href} className="border-b border-line">
                        <DownloadRow download={download} />
                      </li>
                    ))}
                  </ul>
                </>
              )}
            </div>
          </article>
        );
      })}
    </div>
  );
}

/** A compact file link: label, description and filename. */
export function DownloadRow({ download, className }: { download: CraftDownload; className?: string }) {
  return (
    <a
      href={download.href}
      title={download.fileName}
      className={twMerge(
        "group/file flex items-center justify-between gap-4 py-3 hover:bg-bg-sunken",
        className,
      )}
    >
      <span className="min-w-0">
        <span className="block text-sm text-ink">{download.label}</span>
        <span className="block text-sm text-muted">{download.description}</span>
        <span className="mt-1 block truncate font-mono text-[11px] text-faint">
          {download.fileName}
        </span>
      </span>
      <ArrowDownToLineIcon
        aria-hidden
        className="h-4 w-4 shrink-0 text-muted group-hover/file:text-ink"
      />
    </a>
  );
}

/**
 * The app's own hero CTA, used only while ArtCraft Launcher has no release: a
 * direct download once the OS is known, else a jump to #get-it.
 */
export function AppHeroDownloadButton({
  recommended,
  name,
}: {
  recommended: CraftDownload[];
  name: string;
}) {
  const detected = useDetectedDesktop();
  const mine = detected && pickDownload(recommended, detected);
  return (
    <Button href={mine ? mine.href : "#get-it"} size="lg">
      <ArrowDownToLineIcon aria-hidden className="h-4 w-4" />
      {mine ? `Download for ${mine.group}` : `Download ${name}`}
    </Button>
  );
}

/**
 * ArtCraft Launcher's CTA: a direct download for the visitor's system once
 * it is known (with the file's platform and build underneath), else a jump
 * to `fallbackHref`, where every platform is listed.
 */
export function LauncherDownloadButton({
  recommended,
  fallbackHref,
  size = "lg",
  variant = "primary",
  fullWidth = false,
  className,
}: {
  recommended: CraftDownload[];
  fallbackHref: string;
  size?: ButtonProps["size"];
  variant?: ButtonProps["variant"];
  fullWidth?: boolean;
  className?: string;
}) {
  const detected = useDetectedDesktop();
  const mine = detected && pickDownload(recommended, detected);
  return (
    <span className={twMerge("flex flex-col gap-2", className)}>
      <Button
        href={mine ? mine.href : fallbackHref}
        size={size}
        variant={variant}
        className={fullWidth ? "w-full" : undefined}
      >
        <ArrowDownToLineIcon aria-hidden className="h-4 w-4" />
        Download Launcher
      </Button>
      {/* Reserved line, so the layout doesn't shift when detection lands. It
          takes the button's width rather than setting it (w-0 min-w-full), so
          a long platform line can't push the buttons beside it away. */}
      <span aria-hidden={!mine} className="hud-label min-h-4 w-0 min-w-full whitespace-nowrap text-faint">
        {mine ? `For ${mine.group} · ${mine.label}` : "\u00a0"}
      </span>
    </span>
  );
}

/**
 * Shown under the hero once the direct download is live, so visitors on
 * another machine, CPU or package format can still find their build.
 */
export function AppHeroOtherDownloadsLink() {
  const detected = useDetectedDesktop();
  if (!detected) return null;
  return (
    <a
      href="#get-it"
      className="hud-label flex items-center gap-1.5 text-muted hover:text-ink"
    >
      Other platforms and downloads
      <ArrowDownIcon aria-hidden className="h-3.5 w-3.5" />
    </a>
  );
}

function useDetectedDesktop(): DetectedDesktop | null {
  const [detected, setDetected] = useState<DetectedDesktop | null>(null);
  useEffect(() => {
    setDetected(detectDesktop());
  }, []);
  return detected;
}

/** The platform's first recommended file, or one matching the CPU if any. */
function pickDownload(
  downloads: CraftDownload[],
  { platform, arch }: DetectedDesktop,
): CraftDownload | undefined {
  const candidates = downloads.filter(
    (download) => download.group === platform && download.recommended,
  );
  return candidates.find((download) => arch && download.arch === arch) ?? candidates[0];
}

function detectDesktop(): DetectedDesktop | null {
  const ua = navigator.userAgent;
  if (/Android|iPhone|iPad|iPod|Mobile|CrOS/i.test(ua)) return null;
  if (/Windows/i.test(ua)) return { platform: "Windows" };
  if (/Macintosh|Mac OS X/i.test(ua)) {
    // iPadOS Safari reports itself as a Mac; touch support gives it away.
    return navigator.maxTouchPoints > 1 ? null : { platform: "macOS" };
  }
  if (/Linux/i.test(ua)) {
    return {
      platform: "Linux",
      arch: /aarch64|arm64|armv8/i.test(ua) ? "aarch64" : "x86_64",
    };
  }
  return null;
}
