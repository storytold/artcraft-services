import type { ReactNode } from "react";
import { AppleIcon, LayoutGridIcon, MonitorIcon } from "lucide-react";
import AppIconRow from "@/components/apps/app-icon-row";
import { LauncherDownloadButton } from "@/components/apps/app-downloads";
import { craftLauncherRecommended } from "@/components/apps/launcher-callout";
import { Accent } from "@/components/page/page-header";
import { Button } from "@/components/ui";
import { CRAFT_LAUNCHER_NAME, craftLauncherDownloads, craftLauncherReleasePageUrl } from "@/lib/craft-launcher";
import { CRAFTING_APPS_COUNT_WORD } from "@/lib/crafting-apps";
import { WEBAPP_URL } from "@/lib/links";
import { SectionShell } from "./section-shell";

// The page's closing choice: ArtCraft makes two things, so the CTA names both
// side by side (the Crafting Apps first, matching the page order), each with
// its own downloads, instead of one ambiguous "download" button.
export default function FinalCta() {
  const launcherDownloads = craftLauncherRecommended();
  const hasLauncher = launcherDownloads.length > 0;
  return (
    <SectionShell id="start">
      <div className="flex flex-col items-center px-6 pt-20 pb-14 text-center md:pt-28 md:pb-16">
        <p className="hud-label text-faint">Free · Open source · No subscription</p>
        <h2
          data-reveal
          className="mt-6 max-w-4xl font-display text-5xl font-medium leading-[0.98] tracking-[-0.04em] text-ink-strong sm:text-6xl md:text-7xl"
        >
          Start <Accent>crafting</Accent>.
        </h2>
        <p data-reveal className="mt-6 max-w-lg text-lg leading-relaxed text-muted">
          ArtCraft makes two things: the Crafting Apps and ArtCraft Studio.
          Pick the one that fits your work, or get both.
        </p>
      </div>

      <div data-reveal-group className="grid gap-px border-t border-line bg-line md:grid-cols-2">
        <Choice
          label="Crafting Apps"
          note="Native desktop apps"
          visual={<AppIconRow />}
          title="Desktop apps for every craft."
          body={`${CRAFTING_APPS_COUNT_WORD} native, open-source apps for image editing, vector illustration, video, page layout, audio and more. ${CRAFT_LAUNCHER_NAME} installs them all and keeps them up to date.`}
        >
          {hasLauncher && (
            <LauncherDownloadButton
              recommended={launcherDownloads}
              allDownloads={craftLauncherDownloads()}
              fallbackHref={craftLauncherReleasePageUrl()}
            />
          )}
          <Button href="/apps" variant={hasLauncher ? "secondary" : "primary"} size="lg">
            <LayoutGridIcon aria-hidden className="h-4 w-4" />
            Explore the apps
          </Button>
        </Choice>

        <Choice
          label="ArtCraft Studio"
          note="Generative AI"
          visual={
            <span className="flex h-9 w-9 items-center justify-center border border-line">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img src="/artcraft-icon.svg" alt="" className="h-5 w-5" />
            </span>
          }
          title="Generative AI for images and video."
          body="Compose scenes on the canvas and in 3D, then render them with the latest image and video models. On your desktop or in the browser."
        >
          <Button href="/download" size="lg">
            <AppleIcon aria-hidden className="h-4 w-4" />
            <MonitorIcon aria-hidden className="h-4 w-4" />
            Download the studio
          </Button>
          <Button href={WEBAPP_URL} variant="secondary" size="lg">
            Use on web
          </Button>
          {/* Same height as the launcher button's platform line, so both
              cells' buttons line up. */}
          <p className="hud-label w-full text-faint">macOS · Windows · Web</p>
        </Choice>
      </div>
    </SectionShell>
  );
}

// One option cell: a header row like the feature cells, then the pitch, then
// its buttons pinned to the bottom so both cells' actions line up.
function Choice({
  label,
  note,
  visual,
  title,
  body,
  children,
}: {
  label: string;
  note: string;
  visual: ReactNode;
  title: string;
  body: string;
  children: ReactNode;
}) {
  return (
    <article data-reveal className="flex flex-col bg-bg">
      <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-10">
        <p className="hud-label text-ink">{label}</p>
        <p className="hud-label text-faint">{note}</p>
      </div>
      <div className="flex flex-1 flex-col p-6 md:p-10">
        <div aria-hidden>{visual}</div>
        <h3 className="mt-6 font-display text-3xl font-medium leading-[1.05] tracking-[-0.03em] text-ink-strong">
          {title}
        </h3>
        <p className="mt-3 max-w-md leading-relaxed text-muted">{body}</p>
        <div className="mt-auto flex flex-wrap items-start gap-x-3 gap-y-2 pt-8">{children}</div>
      </div>
    </article>
  );
}
