import { AppleIcon, MonitorIcon } from "lucide-react";
import { Accent } from "@/components/page/page-header";
import { Button } from "@/components/ui";
import { WEBAPP_URL } from "@/lib/links";
import { SectionShell, SectionEyebrow } from "./section-shell";

// Chapter break on the landing page: everything above it is the Crafting
// Apps, everything below it (features, ownership, community, the closing
// CTA) is ArtCraft Studio, the generative image and video app. A sunken,
// centered title card so the switch of product reads at a glance.
export default function StudioIntro({ index }: { index: string }) {
  return (
    <SectionShell id="studio">
      <SectionEyebrow
        index={index}
        label="ArtCraft Studio"
        annotation="Generative images & video"
      />
      <div className="flex flex-col items-center bg-bg-sunken px-6 py-16 text-center md:px-10 md:py-24">
        <p data-reveal className="hud-label text-accent-ink">
          Also from ArtCraft
        </p>
        <h2
          data-reveal
          className="mt-6 max-w-3xl text-balance font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl md:text-6xl"
        >
          The AI studio for <Accent>images and video</Accent>.
        </h2>
        <p data-reveal className="mt-5 max-w-xl text-lg leading-relaxed text-muted">
          ArtCraft Studio is our generative app: compose scenes yourself, then
          render them with the latest image and video models. On your desktop
          or in the browser.
        </p>
        <div data-reveal className="mt-8 flex flex-wrap items-center justify-center gap-3">
          <Button href="/download" size="lg">
            <AppleIcon aria-hidden className="h-4 w-4" />
            <MonitorIcon aria-hidden className="h-4 w-4" />
            Download the studio
          </Button>
          <Button href={WEBAPP_URL} variant="secondary" size="lg">
            Use on web
          </Button>
        </div>
      </div>
    </SectionShell>
  );
}
