import type { Metadata } from "next";
import Link from "next/link";
import {
  BotIcon,
  CodeXmlIcon,
  CpuIcon,
  GlobeIcon,
  HardDriveIcon,
  KeyboardIcon,
} from "lucide-react";
import AppCard from "@/components/apps/app-card";
import { LauncherDownloadButton } from "@/components/apps/app-downloads";
import { craftLauncherRecommended } from "@/components/apps/launcher-callout";
import ShareBar from "@/components/apps/share-bar";
import {
  CampaignSection,
  ClosingCta,
  TipGrid,
} from "@/components/campaign/sections";
import DiscordButton from "@/components/discord-button";
import { GitHubIcon } from "@/components/icons";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { AppIcon, ColorAccent, appThemeClass } from "@/components/apps/app-wordmark";
import { PageHeader } from "@/components/page/page-header";
import RevealManager from "@/components/reveal-manager";
import { Button } from "@/components/ui";
import { trackAttrs } from "@/lib/analytics";
import {
  CRAFT_LAUNCHER_NAME,
  CRAFT_LAUNCHER_REPO,
  craftLauncherReleasePageUrl,
} from "@/lib/craft-launcher";
import {
  CRAFTING_APPS,
  CRAFTING_APPS_COUNT_WORD,
  CRAFTING_APPS_GITHUB_ORG,
  CRAFTING_APPS_OG_IMAGE,
  CRAFTING_APPS_PRINCIPLES,
  craftAppIndex,
  craftAppName,
  craftAppPath,
} from "@/lib/crafting-apps";
import { siteUrl } from "@/lib/links";

const TITLE = "Crafting Apps: open-source creative tools";
const COVERAGE =
  "Image editing, vector illustration, video, photography, PDFs, motion graphics, page layout, word processing, audio, presentations, CAD and spreadsheets";
const DESCRIPTION = `${COVERAGE}: ${CRAFTING_APPS_COUNT_WORD.toLowerCase()} native, open-source apps from the ArtCraft team, built in Rust and free to use.`;

export const metadata: Metadata = {
  title: TITLE,
  description: DESCRIPTION,
  alternates: { canonical: "/apps" },
  openGraph: {
    type: "website",
    url: "/apps",
    siteName: "ArtCraft",
    title: TITLE,
    description: DESCRIPTION,
    images: [{ url: CRAFTING_APPS_OG_IMAGE, width: 1200, height: 630 }],
  },
};

// Order matches CRAFTING_APPS_PRINCIPLES.
const PRINCIPLE_ICONS = [
  <CodeXmlIcon key="open" aria-hidden className="h-5 w-5" />,
  <CpuIcon key="native" aria-hidden className="h-5 w-5" />,
  <KeyboardIcon key="familiar" aria-hidden className="h-5 w-5" />,
  <HardDriveIcon key="local" aria-hidden className="h-5 w-5" />,
  <BotIcon key="agents" aria-hidden className="h-5 w-5" />,
  <GlobeIcon key="web" aria-hidden className="h-5 w-5" />,
];

export default function CraftingAppsPage() {
  const launcherDownloads = craftLauncherRecommended();
  return (
    <>
      <RevealManager />

      <PageHeader
        id="apps"
        index="01"
        label="Crafting Apps"
        annotation={`${CRAFTING_APPS_COUNT_WORD} apps · Open source · Pure Rust`}
        title={
          <>
            {CRAFTING_APPS_COUNT_WORD} apps. One <ColorAccent>craft</ColorAccent>.
          </>
        }
        lede={`${COVERAGE}. Native, open-source apps from the ArtCraft team, built in Rust and free to use.`}
      >
        <div data-reveal className="mt-8 flex flex-wrap items-start gap-3">
          {launcherDownloads.length > 0 && (
            <LauncherDownloadButton
              recommended={launcherDownloads}
              fallbackHref={craftLauncherReleasePageUrl()}
            />
          )}
          <DiscordButton
            size="lg"
            variant={launcherDownloads.length > 0 ? "secondary" : "primary"}
          >
            Join the Discord
          </DiscordButton>
          <Button
            href={CRAFTING_APPS_GITHUB_ORG}
            variant="secondary"
            size="lg"
            target="_blank"
            rel="noopener noreferrer"
          >
            <GitHubIcon className="h-4 w-4" />
            Browse on GitHub
          </Button>
        </div>
        {launcherDownloads.length > 0 && (
          <p data-reveal className="mt-4 max-w-xl leading-relaxed text-muted">
            <span className="text-ink">Recommended:</span> {CRAFT_LAUNCHER_NAME}{" "}
            installs every app below and keeps them all up to date. Each app
            also has its own installers on its page.{" "}
            <a
              href={CRAFT_LAUNCHER_REPO}
              target="_blank"
              rel="noopener noreferrer"
              className="text-ink underline decoration-line-strong underline-offset-4 hover:decoration-current"
            >
              Learn more
            </a>
          </p>
        )}
        <ShareBar
          url={siteUrl("/apps")}
          text={`Crafting Apps: ${CRAFTING_APPS_COUNT_WORD.toLowerCase()} open-source creative apps from ArtCraft`}
          className="mt-6"
        />
        <nav
          aria-label="Crafting Apps"
          data-reveal
          className="mt-12 flex flex-wrap gap-x-3 gap-y-6 sm:gap-x-5"
        >
          {CRAFTING_APPS.map((app) => (
            <Link
              key={app.slug}
              href={craftAppPath(app)}
              className={`group flex w-18 flex-col items-center gap-2 sm:w-20 ${appThemeClass(app)}`}
              {...trackAttrs("app_select", { app_name: app.slug })}
            >
              <AppIcon
                app={app}
                priority
                className="h-14 w-14 transition-transform duration-300 ease-out group-hover:-translate-y-1 sm:h-20 sm:w-20"
              />
              <span className="text-xs font-medium text-muted group-hover:text-(--app-ink)">
                {craftAppName(app)}
              </span>
            </Link>
          ))}
        </nav>
      </PageHeader>

      <SectionShell id="lineup">
        <SectionEyebrow
          index="02"
          label="The lineup"
          annotation="Pick a craft to explore"
        />
        <div
          data-reveal-group
          className="grid gap-px bg-line sm:grid-cols-2 xl:grid-cols-4"
        >
          {CRAFTING_APPS.map((app) => (
            <AppCard key={app.slug} app={app} index={craftAppIndex(app)} />
          ))}
          <div data-reveal className="flex flex-col bg-bg p-6 md:p-8">
            <p className="hud-label text-faint">Next up</p>
            <h3 className="mt-4 font-display text-3xl font-medium leading-[1.05] tracking-[-0.03em] text-ink-strong">
              More crafts <ColorAccent>on the way</ColorAccent>.
            </h3>
            <p className="mt-3 leading-relaxed text-muted">
              Tell us what to rebuild next, test early builds and follow
              development with the team in the ArtCraft Discord.
            </p>
            <div className="mt-auto pt-8">
              <DiscordButton />
            </div>
          </div>
        </div>
      </SectionShell>

      <CampaignSection
        id="principles"
        index="03"
        label="Principles"
        annotation="Shared by every craft"
        title={
          <>
            Built the <ColorAccent>hard</ColorAccent> way.
          </>
        }
        lede="Every Crafting App is written from scratch in Rust and held to the same rules."
      >
        <TipGrid
          items={CRAFTING_APPS_PRINCIPLES}
          columns={3}
          icons={PRINCIPLE_ICONS}
        />
      </CampaignSection>

      <ClosingCta
        eyebrow="Open source · Free"
        title={
          <>
            Build it <ColorAccent>with us</ColorAccent>.
          </>
        }
        lede="Early builds, roadmaps and the people making the Crafting Apps all live in the ArtCraft Discord."
      >
        <DiscordButton size="lg">Join the Discord</DiscordButton>
      </ClosingCta>
    </>
  );
}
