import Link from "next/link";
import { LayoutGridIcon } from "lucide-react";
import { WEBAPP_URL } from "@/lib/links";
import { craftLauncherDownloads, craftLauncherReleasePageUrl } from "@/lib/craft-launcher";
import { CRAFTING_APPS_GITHUB_ORG } from "@/lib/crafting-apps";
import { LauncherDownloadButton } from "@/components/apps/app-downloads";
import { craftLauncherRecommended } from "@/components/apps/launcher-callout";
import { GitHubIcon } from "@/components/icons";
import { Button } from "@/components/ui";
import HeroMasthead, { HeroScrim } from "./hero-wordmark";
import HeroGalaxy from "./hero-galaxy";

const TEXT_LINK_CLASSES =
  "text-ink underline decoration-line-strong underline-offset-4 hover:decoration-current";

export default function Hero() {
  const launcherDownloads = craftLauncherRecommended();
  const hasLauncher = launcherDownloads.length > 0;
  return (
    <section id="hero" className="relative">
      {/* Single-stage radial poster: the wordmark at the center is the
          emanation point of the galaxy — showcase cards born blurred behind
          the brand mark, swirling out along the arms. Everything that sells
          (headline, CTAs, proof) stacks beneath the mark, all above the
          fold. The galaxy bleeds full-bleed past the content rails. */}
      <div className="relative flex min-h-[calc(100svh-3rem)] flex-col">
        <HeroGalaxy />

        {/* z-40 (not z-10): this container is a stacking context, so the
            masthead's own z-40 is capped by it — the hero letters must
            outrank the ruler's fixed z-39 contrast pools at root level.
            No border-x here: the poster viewport stays immersive edge to
            edge — the structural rails begin below the poster. */}
        <div className="pointer-events-none relative z-40 mx-auto flex w-full max-w-[1280px] flex-1 flex-col items-center justify-center px-6 py-10 md:px-10">
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
              Capable tools{" "}
              <span className="font-serif italic font-normal text-muted">
                for artists.
              </span>
            </h1>

            <p
              data-reveal
              className="relative mt-4 max-w-md text-lg leading-relaxed text-muted"
            >
              ArtCraft builds open source tools for artists. We pride ourselves
              on freedom of control and expression.
            </p>

            <div
              data-reveal
              className="pointer-events-auto relative mt-7 flex flex-wrap items-start justify-center gap-3"
            >
              {/* The Crafting Apps lead: ArtCraft Launcher installs and
                  updates every one of them. */}
              {hasLauncher && (
                <LauncherDownloadButton
                  recommended={launcherDownloads}
                  allDownloads={craftLauncherDownloads()}
                  fallbackHref={craftLauncherReleasePageUrl()}
                />
              )}
              <Button
                href="/apps"
                variant={hasLauncher ? "secondary" : "primary"}
                size="lg"
              >
                <LayoutGridIcon aria-hidden className="h-4 w-4" />
                Explore Crafting Apps
              </Button>
            </div>

            {/* The AI studio stays one click away, below the apps. */}
            <p
              data-reveal
              className="pointer-events-auto relative mt-5 text-balance leading-relaxed text-muted"
            >
              Looking for ArtCraft Studio?{" "}
              <Link href="/download" className={TEXT_LINK_CLASSES}>
                Download it
              </Link>{" "}
              or{" "}
              <a href={WEBAPP_URL} className={TEXT_LINK_CLASSES}>
                use it on the web
              </a>
              .
            </p>

            <div
              data-reveal
              className="pointer-events-auto relative mt-7 flex flex-wrap items-center justify-center gap-x-6 gap-y-2"
            >
              <a
                href={CRAFTING_APPS_GITHUB_ORG}
                target="_blank"
                rel="noopener noreferrer"
                className="hud-label flex items-center gap-1.5 text-faint hover:text-ink"
              >
                <GitHubIcon className="h-3.5 w-3.5" />
                Open source on GitHub
              </a>
              <p className="hud-label text-faint">macOS · Windows · Linux</p>
              <p className="hud-label text-faint">No subscription required</p>
            </div>
          </div>
        </div>
      </div>

      {/* The poster IS the hero — the studio demo (announce strip +
          blocking↔render comparator) now leads the Features section. */}
    </section>
  );
}
