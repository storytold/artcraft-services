import type { ReactNode } from "react";
import Link from "next/link";
import { LayoutGridIcon } from "lucide-react";
import { craftLauncherDownloads, craftLauncherReleasePageUrl } from "@/lib/craft-launcher";
import { CRAFTING_APPS_GITHUB_ORG } from "@/lib/crafting-apps";
import { LauncherDownloadButton } from "@/components/apps/app-downloads";
import { craftLauncherRecommended } from "@/components/apps/launcher-callout";
import { GitHubIcon } from "@/components/icons";
import { Button } from "@/components/ui";
import HeroMasthead, { HeroScrim } from "./hero-wordmark";
import HeroGalaxy from "./hero-galaxy";
import type { GalaxyVariant } from "./hero-galaxy-tunables";

export const HERO_LINK_CLASSES =
  "text-ink underline decoration-line-strong underline-offset-4 hover:decoration-current";

// Single-stage radial poster: the wordmark at the center is the emanation
// point of the galaxy, which bleeds full-bleed past the content rails.
// Everything that sells (headline, CTAs, proof) stacks beneath the mark, all
// above the fold.
export function HeroPoster({
  variant,
  title,
  lede,
  actions,
  note,
  githubHref,
  facts,
}: {
  variant: GalaxyVariant;
  title: ReactNode;
  lede: ReactNode;
  actions: ReactNode;
  /** The line under the CTAs pointing at the other product. */
  note: ReactNode;
  githubHref: string;
  facts: string[];
}) {
  return (
    <section id="hero" className="relative">
      <div className="relative flex min-h-[calc(100svh-3rem)] flex-col">
        <HeroGalaxy variant={variant} />

        {/* z-40 (not z-10): this container is a stacking context, so the
            masthead's own z-40 is capped by it — the hero letters must
            outrank the ruler's fixed z-39 contrast pools at root level.
            No border-x here: the poster viewport stays immersive edge to
            edge — the structural rails begin below the poster. */}
        {/* Icons can't pass under the copy, so on phones, where the stack
            fills the width, the app galaxy gets bands above and below it. */}
        <div
          className={`pointer-events-none relative z-40 mx-auto flex w-full max-w-[1280px] flex-1 flex-col items-center justify-center px-6 md:px-10 ${variant === "apps" ? "py-36 md:py-10" : "py-10"}`}
        >
          <div
            data-reveal-group
            className="relative flex w-full max-w-2xl flex-col items-center text-center"
          >
            {/* Soft pocket in the nebula so the sales layer always reads
                (tunable in the Wordmark tuner group). */}
            <HeroScrim />

            <HeroMasthead />

            <h1
              data-reveal
              className="relative mt-8 font-display text-3xl font-medium leading-[1.05] tracking-[-0.03em] text-ink-strong sm:text-4xl"
            >
              {title}
            </h1>

            <p
              data-reveal
              className="relative mt-4 max-w-md text-lg leading-relaxed text-muted"
            >
              {lede}
            </p>

            <div
              data-reveal
              className="pointer-events-auto relative mt-7 flex w-full justify-center"
            >
              {actions}
            </div>

            <p
              data-reveal
              className="pointer-events-auto relative mt-5 text-balance leading-relaxed text-muted"
            >
              {note}
            </p>

            <div
              data-reveal
              className="pointer-events-auto relative mt-7 flex flex-wrap items-center justify-center gap-x-6 gap-y-2"
            >
              <a
                href={githubHref}
                target="_blank"
                rel="noopener noreferrer"
                className="hud-label flex items-center gap-1.5 text-faint hover:text-ink"
              >
                <GitHubIcon className="h-3.5 w-3.5" />
                Open source on GitHub
              </a>
              {facts.map((fact) => (
                <p key={fact} className="hud-label text-faint">
                  {fact}
                </p>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

// The company landing: the Crafting Apps' icons ride the galaxy, and the
// Launcher that installs them all leads the CTAs.
export default function Hero() {
  const launcherDownloads = craftLauncherRecommended();
  const hasLauncher = launcherDownloads.length > 0;
  const exploreButton = (
    <Button href="/apps" variant={hasLauncher ? "secondary" : "primary"} size="lg">
      <LayoutGridIcon aria-hidden className="h-4 w-4" />
      Explore Crafting Apps
    </Button>
  );
  return (
    <HeroPoster
      variant="apps"
      title={
        <>
          Capable{" "}
          <span className="font-serif italic font-normal text-muted">
            open source
          </span>{" "}
          tools.
        </>
      }
      lede="ArtCraft builds open source tools, free to use. We pride ourselves on freedom of control and expression."
      actions={
        hasLauncher ? (
          <LauncherDownloadButton
            recommended={launcherDownloads}
            allDownloads={craftLauncherDownloads()}
            fallbackHref={craftLauncherReleasePageUrl()}
            centered
          >
            {exploreButton}
          </LauncherDownloadButton>
        ) : (
          exploreButton
        )
      }
      note={
        <>
          Looking for AI images and video?{" "}
          <Link href="/studio" className={HERO_LINK_CLASSES}>
            See ArtCraft Studio
          </Link>
          .
        </>
      }
      githubHref={CRAFTING_APPS_GITHUB_ORG}
      facts={["macOS · Windows · Linux", "No subscription required"]}
    />
  );
}
