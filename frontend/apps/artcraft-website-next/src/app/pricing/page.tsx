import type { Metadata } from "next";
import { TagIcon } from "lucide-react";
import RevealManager from "@/components/reveal-manager";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Accent, PageHeader } from "@/components/page/page-header";
import PricingTable from "@/components/pricing/pricing-table";
import { Badge, Button } from "@/components/ui";
import { webappUrl } from "@/lib/links";
import { PROMO_PCT } from "@/lib/pricing-data";

export const metadata: Metadata = {
  title: "Pricing",
  description:
    "Simple pricing for ArtCraft. Generate AI video and images with every model in one open desktop app. No subscriptions needed — you own ArtCraft.",
  alternates: { canonical: "/pricing" },
};

// Referral code that swaps the header for the Seedance early-access pitch
// (kept from the Vite site's ?ref=sd2fakeyou campaign links).
const SEEDANCE_REF = "sd2fakeyou";
const SEEDANCE_VIMEO_URL =
  "https://player.vimeo.com/video/1169289718?autoplay=1&muted=1&loop=1&background=0&byline=0&portrait=0&title=0";

type SearchParams = Promise<Record<string, string | string[] | undefined>>;

export default async function PricingPage({
  searchParams,
}: {
  searchParams: SearchParams;
}) {
  const params = await searchParams;
  const ref = typeof params.ref === "string" ? params.ref : "";
  const isSeedanceRef = ref === SEEDANCE_REF;
  const checkoutQuery = ref ? `ref=${encodeURIComponent(ref)}` : "";

  return (
    <>
      <RevealManager />

      {isSeedanceRef ? <SeedanceHeader /> : <DefaultHeader />}

      <SectionShell id="plans" className="scroll-mt-12">
        <SectionEyebrow
          index="02"
          label="Choose your plan"
          annotation="Every paid plan includes video credits"
        />
        <PricingTable showSeedanceFeatures checkoutQuery={checkoutQuery} />
      </SectionShell>

      <SectionShell id="credits">
        <div className="grid gap-px bg-line md:grid-cols-2">
          <div className="bg-bg p-6 md:p-10">
            <p className="hud-label text-faint">Already have an account?</p>
            <h2 className="mt-4 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
              One-time credit packs
            </h2>
            <p className="mt-2 max-w-md leading-relaxed text-muted">
              Top up without changing your plan. Credit packs are purchased
              inside the app and never expire.
            </p>
            <Button
              href={webappUrl("/pricing")}
              variant="secondary"
              className="mt-6"
            >
              Buy credits in the app
            </Button>
          </div>
          <div className="bg-bg p-6 md:p-10">
            <p className="hud-label text-faint">† Footnote</p>
            <p className="mt-4 max-w-md leading-relaxed text-muted">
              ArtCraft can be used without paying for a subscription. You can
              bring your own compute and third-party subscriptions. We hope
              you&rsquo;ll subscribe, though, as that helps accelerate our
              development.
            </p>
          </div>
        </div>
      </SectionShell>
    </>
  );
}

function DefaultHeader() {
  return (
    <>
      {/* Promo strip: the offer is the first thing on the page. */}
      <div className="border-t border-line bg-accent text-white">
        <div className="mx-auto flex max-w-[1280px] flex-wrap items-center justify-between gap-x-6 gap-y-1 px-6 py-2.5 md:px-10">
          <p className="hud-label flex items-center gap-2 font-bold">
            <TagIcon aria-hidden className="h-3.5 w-3.5" />
            Limited-time offer
          </p>
          <p className="text-sm">
            Save {PROMO_PCT}% on every plan, monthly or yearly — lock in the
            lowest price today.
          </p>
        </div>
      </div>
      <PageHeader
        index="01"
        label="Pricing"
        annotation="Free & open source · Subscriptions optional†"
        title={
          <>
            Every model. One studio. <Accent>Yours</Accent> forever.
          </>
        }
        lede="Thousands of credits for Seedance, Nano Banana, Kling and more — inside an open-source app you keep even if you never pay again."
      >
        <div data-reveal className="mt-8 flex flex-wrap items-center gap-3">
          <Button href="#plans" size="lg">
            See plans
          </Button>
          <Badge
            label={`${PROMO_PCT}% off · ends soon`}
            className="border-transparent bg-accent text-white"
          />
        </div>
      </PageHeader>
    </>
  );
}

function SeedanceHeader() {
  return (
    <>
      <PageHeader
        index="01"
        label="Early access"
        annotation="Available today in ArtCraft"
        title={
          <>
            Seedance 2.0 is <Accent>here</Accent>.
          </>
        }
        lede="Generate jaw-dropping AI videos with Seedance 2.0 before it's available anywhere else. Every paid plan includes video credits, so you can start creating right now."
      />
      <SectionShell>
        <div className="grid gap-px bg-line lg:grid-cols-[3fr_2fr]">
          <figure className="bg-bg">
            <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-8">
              <figcaption className="hud-label text-muted">
                Seedance in ArtCraft
              </figcaption>
              <p className="hud-label text-faint">Reel</p>
            </div>
            <div className="relative aspect-video w-full overflow-hidden bg-bg-sunken">
              <iframe
                src={SEEDANCE_VIMEO_URL}
                className="absolute inset-0 h-full w-full"
                allow="autoplay; fullscreen; picture-in-picture"
                allowFullScreen
                title="Seedance in ArtCraft"
              />
            </div>
          </figure>
          <div className="grid gap-px bg-line sm:grid-cols-2 lg:grid-cols-1">
            <div className="bg-bg p-6 md:p-8">
              <p className="hud-label text-accent-ink">Seedance video credits</p>
              <p className="mt-3 leading-relaxed text-muted">
                Included with every paid ArtCraft plan.
              </p>
            </div>
            <div className="bg-bg p-6 md:p-8">
              <p className="hud-label text-muted">First in the world</p>
              <p className="mt-3 leading-relaxed text-muted">
                Seedance launches in ArtCraft ahead of anywhere else.
              </p>
            </div>
          </div>
        </div>
      </SectionShell>
    </>
  );
}
