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
import { Button } from "@/components/ui";
import { trackAttrs } from "@/lib/analytics";
import {
  CRAFT_LAUNCHER_NAME,
  CRAFT_LAUNCHER_REPO,
  craftLauncherDownloads,
  craftLauncherReleasePageUrl,
} from "@/lib/craft-launcher";
import {
  CRAFTING_APPS,
  CRAFTING_APPS_COUNT_WORD,
  CRAFTING_APPS_GITHUB_ORG,
  CRAFTING_APPS_PRINCIPLES,
  craftAppIndex,
  craftAppName,
  craftAppPath,
} from "@/lib/crafting-apps";
import { siteUrl } from "@/lib/links";

export const CRAFTING_APPS_COVERAGE =
  "Image editing, vector illustration, video, photography, PDFs, motion graphics, page layout, interface design, 3D texture painting, procedural materials, word processing, audio, presentations, CAD and spreadsheets";

// Order matches CRAFTING_APPS_PRINCIPLES.
const PRINCIPLE_ICONS = [
  <CodeXmlIcon key="open" aria-hidden className="h-5 w-5" />,
  <CpuIcon key="native" aria-hidden className="h-5 w-5" />,
  <KeyboardIcon key="familiar" aria-hidden className="h-5 w-5" />,
  <HardDriveIcon key="local" aria-hidden className="h-5 w-5" />,
  <BotIcon key="agents" aria-hidden className="h-5 w-5" />,
  <GlobeIcon key="web" aria-hidden className="h-5 w-5" />,
];

// The Crafting Apps story, shared by the /apps hub and the company landing:
// header with the icon directory, the lineup, then the principles.
export function CraftingAppsOverview({
  headingAs,
}: {
  /** "h2" where a hero above already owns the page's h1. */
  headingAs?: "h1" | "h2";
}) {
  const launcherDownloads = craftLauncherRecommended();
  const allLauncherDownloads = craftLauncherDownloads();
  const communityButtons = (
    <>
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
    </>
  );
  return (
    <>
      <PageHeader
        id="apps"
        index="01"
        label="Crafting Apps"
        annotation={`${CRAFTING_APPS_COUNT_WORD} apps · Open source · Pure Rust`}
        headingAs={headingAs}
        title={
          <>
            {CRAFTING_APPS_COUNT_WORD} apps.{" "}
            <span className="whitespace-nowrap">
              One <ColorAccent>craft</ColorAccent>.
            </span>
          </>
        }
        lede={`${CRAFTING_APPS_COVERAGE}. Native, open-source apps from the ArtCraft team, built in Rust and free to use.`}
        aside={
            <nav
              aria-label="Crafting Apps"
              data-reveal
              // Sixteen apps in even rows. Under the text: four across, then two
              // rows of eight from lg up. Beside it from xl up: four across.
              className="mt-12 grid w-fit grid-cols-4 gap-x-3 gap-y-6 sm:gap-x-5 lg:grid-cols-8 xl:mt-0 xl:shrink-0 xl:grid-cols-4"
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
        }
      >
        <div data-reveal className="mt-8">
          {launcherDownloads.length > 0 ? (
            <LauncherDownloadButton
              recommended={launcherDownloads}
              allDownloads={allLauncherDownloads}
              fallbackHref={craftLauncherReleasePageUrl()}
            >
              {communityButtons}
            </LauncherDownloadButton>
          ) : (
            <div className="flex flex-wrap items-start gap-3">{communityButtons}</div>
          )}
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
          {/* Sixteen apps fill every row two and four across, so Next up
              closes the grid as a full-width row of its own. */}
          <div
            data-reveal
            className="flex flex-col gap-8 bg-bg p-6 sm:col-span-2 md:flex-row md:items-end md:justify-between md:p-8 xl:col-span-4"
          >
            <div>
              <p className="hud-label text-faint">Next up</p>
              <h3 className="mt-4 font-display text-3xl font-medium leading-[1.05] tracking-[-0.03em] text-ink-strong">
                More crafts <ColorAccent>on the way</ColorAccent>.
              </h3>
              <p className="mt-3 max-w-xl leading-relaxed text-muted">
                Tell us what to rebuild next, test early builds and follow
                development with the team in the ArtCraft Discord.
              </p>
            </div>
            <DiscordButton className="shrink-0 self-start md:self-auto" />
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
    </>
  );
}

export function CraftingAppsClosing() {
  return (
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
  );
}
