import Link from "next/link";
import type { ReactNode } from "react";
import { ArrowUpRightIcon, TerminalIcon } from "lucide-react";
import { twMerge } from "tailwind-merge";
import DiscordButton from "@/components/discord-button";
import { GitHubIcon } from "@/components/icons";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Badge, Button, CopyButton } from "@/components/ui";
import { trackAttrs } from "@/lib/analytics";
import { CRAFT_LAUNCHER_RELEASE } from "@/lib/crafting-app-releases";
import {
  CRAFTING_APPS,
  craftAppBuildCommand,
  craftAppIndex,
  craftAppName,
  craftAppRelease,
  craftAppRepo,
  craftReleaseDownloads,
  craftReleasePageUrl,
  craftReleasesUrl,
  craftShotSourceUrl,
  craftShotUrl,
  formatList,
  type CraftApp,
} from "@/lib/crafting-apps";
import AppCard from "./app-card";
import { AppPlatformDownloads, DownloadRow } from "./app-downloads";
import { ColorAccent } from "./app-wordmark";
import { LauncherCallout } from "./launcher-callout";

// Body sections of an /apps/<slug> page, in page order. Server components;
// the client islands are the CopyButton and the OS-aware platform cards.

const CELL_HEADING_CLASSES =
  "mt-4 font-display text-3xl font-medium leading-[1.05] tracking-[-0.03em] text-ink-strong sm:text-4xl";

// The hero carries shot 01. Window captures pair up, and an odd one out
// leads at full width; portrait pages (layout spreads) sit four across.
export function AppGallery({ app, index }: { app: CraftApp; index: string }) {
  const [, ...shots] = app.shots;
  const portrait = shots.every((shot) => shot.portrait);
  const leadFullWidth = !portrait && shots.length % 2 === 1;
  return (
    <SectionShell id="gallery">
      <SectionEyebrow
        index={index}
        label="Screenshots"
        annotation={`Captured in ${craftAppName(app)}`}
      />
      <div
        data-reveal-group
        className={twMerge(
          "grid gap-px bg-line",
          portrait ? "grid-cols-2 lg:grid-cols-4" : "md:grid-cols-2",
        )}
      >
        {shots.map((shot, i) => (
          <figure
            key={shot.file}
            data-reveal
            className={twMerge("bg-bg", leadFullWidth && i === 0 && "md:col-span-2")}
          >
            <a
              href={craftShotSourceUrl(app, shot)}
              target="_blank"
              rel="noopener noreferrer"
              className={twMerge(
                "group relative block overflow-hidden bg-bg-sunken",
                shot.portrait ? "aspect-[7/9]" : "aspect-[16/10]",
              )}
            >
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={craftShotUrl(app, shot)}
                alt={shot.alt}
                loading="lazy"
                decoding="async"
                className="h-full w-full object-cover"
              />
              <span className="hud-label absolute right-3 bottom-3 flex items-center gap-1 bg-bg/80 px-2 py-1 text-ink opacity-0 backdrop-blur-sm transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100">
                Full size
                <ArrowUpRightIcon aria-hidden className="h-3 w-3" />
              </span>
            </a>
            <figcaption
              className={twMerge(
                "flex items-center justify-between gap-4 border-t border-line py-3",
                portrait ? "px-4 md:px-6" : "px-6 md:px-10",
              )}
            >
              <span className="hud-label text-muted">{shot.caption}</span>
              <span className="hud-label text-faint">
                {String(i + 2).padStart(2, "0")}
              </span>
            </figcaption>
          </figure>
        ))}
      </div>
    </SectionShell>
  );
}

// ArtCraft Launcher first (installs and updates every app), then this app's
// own platform cards (your system first-class, every other build one click
// away) above release details and build-from-source. Versions and files come
// from crafting-app-releases.ts; a null release there drops the cards and
// shows the Discord waitlist instead.
export function AppGetIt({ app, index }: { app: CraftApp; index: string }) {
  const name = craftAppName(app);
  const command = craftAppBuildCommand(app);
  const desktop = app.platforms.filter((platform) => platform !== "Web");
  const release = craftAppRelease(app);
  const downloads = release ? craftReleaseDownloads(app, release) : [];
  const extras = downloads.filter((download) => download.group === "Other");

  return (
    <SectionShell id="get-it">
      <SectionEyebrow
        index={index}
        label={`Get ${name}`}
        annotation="Free · Open source"
      />
      {release && (
        <>
          {CRAFT_LAUNCHER_RELEASE && (
            <>
              <LauncherCallout app={app} />
              <div data-reveal className="border-y border-line bg-bg p-6 md:p-10">
                <p className="hud-label text-faint">Option 2 · Standalone</p>
                <h3 className={CELL_HEADING_CLASSES}>
                  Just {name}? Download it <ColorAccent>standalone</ColorAccent>.
                </h3>
                <p className="mt-4 max-w-xl leading-relaxed text-muted">
                  These installers add {name} on its own, without the
                  launcher. Come back here for new versions, or install the
                  launcher later and it will find {name} and keep it updated.
                </p>
              </div>
            </>
          )}
          <div className="border-b border-line">
            <AppPlatformDownloads name={name} downloads={downloads} />
          </div>
        </>
      )}
      <div className="grid gap-px bg-line md:grid-cols-2">
        <div data-reveal className="flex flex-col bg-bg p-6 md:p-10">
          {release ? (
            <>
              <p className="hud-label text-faint">Release</p>
              <h3 className={CELL_HEADING_CLASSES}>
                Version <ColorAccent>{release.version}</ColorAccent>.
              </h3>
              <p className="mt-4 max-w-md leading-relaxed text-muted">
                Free for every platform above. Questions or feedback? The
                team is in the ArtCraft Discord.
              </p>
              {extras.length > 0 && (
                <>
                  <p className="hud-label mt-8 text-faint">Other downloads</p>
                  <ul className="mt-2 border-t border-line">
                    {extras.map((download) => (
                      <li key={download.href} className="border-b border-line">
                        <DownloadRow download={download} />
                      </li>
                    ))}
                  </ul>
                </>
              )}
              <div className="mt-6 flex flex-wrap gap-x-6 gap-y-2">
                <ExternalLink href={craftReleasePageUrl(app, release)}>
                  Release notes on GitHub
                </ExternalLink>
                <ExternalLink href={craftReleasesUrl(app)}>
                  All releases
                </ExternalLink>
              </div>
            </>
          ) : (
            <>
              <p className="hud-label text-faint">Installers</p>
              <h3 className={CELL_HEADING_CLASSES}>
                Installers are <ColorAccent>on the way</ColorAccent>.
              </h3>
              <p className="mt-4 max-w-md leading-relaxed text-muted">
                Native installers for {formatList(desktop)} are coming. Join
                the ArtCraft Discord to get them first, try early builds and
                talk with the team building {name}.
              </p>
              <DiscordButton size="lg" className="mt-8">
                Get notified on Discord
              </DiscordButton>
            </>
          )}
          <div className="mt-auto flex flex-wrap gap-1.5 pt-10">
            {app.platforms.map((platform) => (
              <Badge key={platform} label={platform} />
            ))}
          </div>
        </div>

        <div data-reveal className="flex min-w-0 flex-col bg-bg p-6 md:p-10">
          <p className="hud-label text-faint">Build from source</p>
          <h3 className={CELL_HEADING_CLASSES}>
            Run it <ColorAccent>today</ColorAccent>.
          </h3>
          <p className="mt-4 max-w-md leading-relaxed text-muted">
            Needs Rust {app.rustVersion} or newer. New to Rust? Install it with{" "}
            <a
              href="https://rustup.rs"
              target="_blank"
              rel="noopener noreferrer"
              className="text-ink underline decoration-line-strong underline-offset-4 hover:decoration-current"
            >
              rustup
            </a>
            , then run:
          </p>
          <figure className="mt-6 border border-line bg-bg-sunken">
            <div className="flex items-center justify-between border-b border-line pl-4">
              <figcaption className="hud-label flex items-center gap-2 text-faint">
                <TerminalIcon aria-hidden className="h-3.5 w-3.5" />
                Terminal
              </figcaption>
              <CopyButton
                value={command}
                variant="ghost"
                className="h-9 border-l border-line px-3"
                {...trackAttrs("copy_command", { app_name: app.slug })}
              />
            </div>
            <pre className="overflow-x-auto p-4 font-mono text-[13px] leading-relaxed text-ink">
              <code>{command}</code>
            </pre>
          </figure>
          <Button
            href={craftAppRepo(app)}
            variant="secondary"
            target="_blank"
            rel="noopener noreferrer"
            className="mt-8"
          >
            <GitHubIcon className="h-4 w-4" />
            View source on GitHub
          </Button>
        </div>
      </div>
    </SectionShell>
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

// The other apps, compact, linking back to the hub.
export function AppFamily({ app, index }: { app: CraftApp; index: string }) {
  const siblings = CRAFTING_APPS.filter((other) => other.slug !== app.slug);
  return (
    <SectionShell id="family">
      <SectionEyebrow index={index} label="The family" annotation="Crafting Apps" />
      <div className="flex flex-col gap-4 px-6 py-12 md:flex-row md:items-end md:justify-between md:px-10 md:py-16">
        <h2
          data-reveal
          className="max-w-2xl font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl"
        >
          More from the <ColorAccent>family</ColorAccent>.
        </h2>
        <Link
          href="/apps"
          className="hud-label flex items-center gap-1.5 text-muted hover:text-ink"
        >
          All Crafting Apps
          <ArrowUpRightIcon aria-hidden className="h-3.5 w-3.5" />
        </Link>
      </div>
      <div
        data-reveal-group
        className="grid gap-px border-t border-line bg-line sm:grid-cols-2 lg:grid-cols-3"
      >
        {siblings.map((sibling) => (
          <AppCard
            key={sibling.slug}
            app={sibling}
            index={craftAppIndex(sibling)}
            compact
          />
        ))}
      </div>
    </SectionShell>
  );
}
