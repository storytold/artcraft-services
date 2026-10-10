import type { Metadata } from "next";
import { notFound } from "next/navigation";
import {
  AppFamily,
  AppGallery,
  AppGetIt,
} from "@/components/apps/app-sections";
import {
  AppHeroDownloadButton,
  AppHeroOtherDownloadsLink,
  LauncherDownloadButton,
} from "@/components/apps/app-downloads";
import { craftLauncherRecommended } from "@/components/apps/launcher-callout";
import {
  AppIcon,
  AppTab,
  AppWordmark,
  ColorAccent,
  appThemeClass,
} from "@/components/apps/app-wordmark";
import ShareBar from "@/components/apps/share-bar";
import { ClosingCta, CampaignSection, TipGrid } from "@/components/campaign/sections";
import CampaignHero from "@/components/campaign/campaign-hero";
import DiscordButton from "@/components/discord-button";
import { GitHubIcon } from "@/components/icons";
import RevealManager from "@/components/reveal-manager";
import { Button } from "@/components/ui";
import {
  CRAFTING_APPS,
  craftAppIndex,
  craftAppName,
  craftAppOgImage,
  craftAppPath,
  craftAppIconUrl,
  craftAppRelease,
  craftAppRepo,
  craftReleaseDownloads,
  craftReleasePageUrl,
  craftShotUrl,
  getCraftApp,
} from "@/lib/crafting-apps";
import { SITE_URL, siteUrl } from "@/lib/links";

type Params = Promise<{ slug: string }>;

export const dynamicParams = false;

export function generateStaticParams() {
  return CRAFTING_APPS.map((app) => ({ slug: app.slug }));
}

export async function generateMetadata({
  params,
}: {
  params: Params;
}): Promise<Metadata> {
  const { slug } = await params;
  const app = getCraftApp(slug);
  if (!app) return {};
  const name = craftAppName(app);
  const title = `${name}: ${app.headline.join("")}`;
  return {
    title,
    description: app.lede,
    alternates: { canonical: craftAppPath(app) },
    openGraph: {
      type: "website",
      url: craftAppPath(app),
      siteName: "ArtCraft",
      title,
      description: app.pitch,
      images: [{ url: craftAppOgImage(app), width: 1200, height: 630, alt: app.shots[0].alt }],
    },
  };
}

export default async function CraftAppPage({ params }: { params: Params }) {
  const { slug } = await params;
  const app = getCraftApp(slug);
  if (!app) notFound();

  const name = craftAppName(app);
  const release = craftAppRelease(app);
  // The hero's download is ArtCraft Launcher, which installs this app and keeps
  // it updated; the app's own installers are in #get-it, under the launcher.
  const launcherDownloads = craftLauncherRecommended();
  const hasDownload = launcherDownloads.length > 0 || release !== null;
  const [hero] = app.shots;
  const [before, accent, after] = app.headline;
  const shareUrl = siteUrl(craftAppPath(app));
  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name,
    applicationCategory: app.schemaCategory,
    operatingSystem: app.platforms.join(", "),
    description: app.lede,
    url: shareUrl,
    image: siteUrl(craftAppOgImage(app)),
    thumbnailUrl: siteUrl(craftAppIconUrl(app)),
    screenshot: app.shots.map((shot) => siteUrl(craftShotUrl(app, shot))),
    sameAs: craftAppRepo(app),
    ...(release && {
      softwareVersion: release.version,
      downloadUrl: craftReleasePageUrl(app, release),
    }),
    isAccessibleForFree: true,
    offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
    publisher: { "@type": "Organization", name: "ArtCraft", url: SITE_URL },
  };

  return (
    <div className={appThemeClass(app)}>
      <RevealManager />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />

      <CampaignHero
        id={app.slug}
        label={name}
        annotation={`${app.category} · ${app.status}`}
        kicker={
          <span className="flex flex-col items-center gap-5">
            <AppIcon app={app} priority className="h-24 w-24 md:h-28 md:w-28" />
            <span className="flex items-center gap-3">
              <AppTab>{craftAppIndex(app)}</AppTab>
              <AppWordmark app={app} className="text-2xl" />
            </span>
          </span>
        }
        title={
          <>
            {before}
            <ColorAccent>{accent}</ColorAccent>
            {after}
          </>
        }
        lede={app.lede}
        media={
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={craftShotUrl(app, hero)}
            alt={hero.alt}
            fetchPriority="high"
            className="absolute inset-0 h-full w-full object-cover"
          />
        }
        mediaCaption={hero.caption}
        mediaTag="Screenshot"
      >
        <div className="flex flex-col items-center gap-6">
          <div className="flex flex-wrap items-start justify-center gap-3">
            {launcherDownloads.length > 0 ? (
              <LauncherDownloadButton recommended={launcherDownloads} fallbackHref="#get-it" />
            ) : (
              release && (
                <AppHeroDownloadButton
                  name={name}
                  recommended={craftReleaseDownloads(app, release).filter(
                    (download) => download.recommended,
                  )}
                />
              )
            )}
            <DiscordButton
              size="lg"
              variant={hasDownload ? "secondary" : "primary"}
            >
              Join the Discord
            </DiscordButton>
            <Button
              href={craftAppRepo(app)}
              variant="secondary"
              size="lg"
              target="_blank"
              rel="noopener noreferrer"
            >
              <GitHubIcon className="h-4 w-4" />
              View on GitHub
            </Button>
          </div>
          {hasDownload && <AppHeroOtherDownloadsLink />}
          <p className="hud-label text-faint">
            {app.platforms.join(" · ")} · Free and open source
          </p>
          <ShareBar url={shareUrl} text={`${name}: ${app.pitch}`} />
        </div>
      </CampaignHero>

      <CampaignSection
        id="highlights"
        index="02"
        label="Highlights"
        annotation={app.category}
        title={
          <>
            What&apos;s <ColorAccent>inside</ColorAccent>.
          </>
        }
      >
        <TipGrid items={app.features} columns={3} />
      </CampaignSection>

      <AppGallery app={app} index="03" />
      <AppGetIt app={app} index="04" />
      <AppFamily app={app} index="05" />

      <ClosingCta
        eyebrow="Open source · Free"
        title={
          <>
            Help shape <AppWordmark app={app} />.
          </>
        }
        lede={`Report bugs, request features and get early builds of ${name} in the ArtCraft Discord.`}
      >
        <DiscordButton size="lg">Join the Discord</DiscordButton>
      </ClosingCta>
    </div>
  );
}
