import type { Metadata } from "next";
import Link from "next/link";
import { FilmIcon, LayersIcon, PlayIcon, ScrollTextIcon } from "lucide-react";
import { InstagramIcon } from "@/components/icons";
import CampaignCta from "@/components/campaign/campaign-cta";
import CampaignHero from "@/components/campaign/campaign-hero";
import { CampaignSection, ClosingCta, TipGrid } from "@/components/campaign/sections";
import { Accent } from "@/components/page/page-header";
import RevealManager from "@/components/reveal-manager";
import { Button } from "@/components/ui";
import {
  JBOOGX,
  JBOOGX_CRAFT,
  JBOOGX_CREDITS,
  JBOOGX_FEATURED,
  JBOOGX_FEED,
  type InstagramPost,
} from "@/lib/campaign-data";

export const metadata: Metadata = {
  title: "Jboogx Creative × ArtCraft. Seedance AI Video Creator Spotlight.",
  description:
    "Tyler Bernabe (@jboogxcreative) makes AI, VFX, and mixed-media work for nearly a million followers, and he directs his Seedance videos in ArtCraft, home of Seedance 2.5. Watch the work, then make your own with the same tools.",
  alternates: { canonical: "/creators/jboogxcreative" },
  openGraph: { title: "Jboogx Creative × ArtCraft" },
};

// Person schema so search engines connect this page to his own profiles.
const JSON_LD = {
  "@context": "https://schema.org",
  "@type": "ProfilePage",
  mainEntity: {
    "@type": "Person",
    name: "Tyler M. Bernabe",
    alternateName: "jboogxcreative",
    description: "Digital taste maker working across AI, VFX, and mixed media.",
    sameAs: [JBOOGX.instagramUrl, JBOOGX.siteUrl],
  },
};

const CRAFT_ICONS = [ScrollTextIcon, FilmIcon, LayersIcon].map((Icon, i) => (
  <Icon key={i} aria-hidden className="h-5 w-5" />
));

export default function CreatorJboogxPage() {
  return (
    <>
      <RevealManager />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(JSON_LD) }}
      />

      <CampaignHero
        id="creator"
        label="Creator spotlight"
        annotation="Made with ArtCraft"
        title={
          <>
            jboogxcreative, digital <Accent>taste maker</Accent>.
          </>
        }
        lede="Nearly a million people follow Tyler Bernabe for AI, VFX, and mixed-media work that treats the feed like a screening room. His Seedance pieces are made in ArtCraft, the same app you can open right now."
      >
        <div className="flex flex-wrap items-center justify-center gap-3">
          <CampaignCta webLabel="Create now" webIcon="sparkles" />
          <Button
            href={JBOOGX.instagramUrl}
            target="_blank"
            rel="noopener noreferrer"
            variant="secondary"
            size="lg"
          >
            <InstagramIcon className="h-4 w-4" />
            Follow @jboogxcreative
          </Button>
        </div>
      </CampaignHero>

      {/* Reel triptych: his three newest reels; the center card is the newest. */}
      <section className="border-t border-line">
        <div className="mx-auto max-w-[1280px] border-x border-line">
          <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-10">
            <p className="hud-label text-muted">Fresh from the feed</p>
            <p className="hud-label text-faint">Tap a frame to watch on Instagram</p>
          </div>
          <ul data-reveal-group className="grid gap-px bg-line sm:grid-cols-3">
            {JBOOGX_FEATURED.map((post, i) => (
              <li key={post.path} data-reveal className="bg-bg p-6 md:p-8">
                <div className="relative aspect-[9/16] w-full overflow-hidden bg-bg-sunken">
                  <ReelCard post={post} eager index={String(i + 1).padStart(2, "0")} />
                </div>
              </li>
            ))}
          </ul>
        </div>
      </section>

      {/* Selected credits marquee, names from his Instagram bio. */}
      <div className="marquee border-t border-line">
        {[0, 1].map((copy) => (
          <ul key={copy} aria-hidden={copy === 1 || undefined} className="marquee-track items-center">
            {[...JBOOGX_CREDITS, ...JBOOGX_CREDITS].map((name, i) => (
              <li key={`${name}-${i}`} className="flex items-center whitespace-nowrap px-6 py-4">
                <span
                  className={
                    i % 2
                      ? "font-serif text-3xl italic text-muted"
                      : "font-display text-3xl font-medium tracking-[-0.02em] text-faint"
                  }
                >
                  {name}
                </span>
                <span aria-hidden className="ml-12 h-1.5 w-1.5 bg-accent" />
              </li>
            ))}
          </ul>
        ))}
      </div>

      <CampaignSection
        id="the-work"
        index="02"
        label="The work"
        annotation="Each one opens on Instagram"
        title={
          <>
            Straight from the <Accent>feed</Accent>.
          </>
        }
        lede="Recent drops from @jboogxcreative, all made with ArtCraft."
      >
        <ul className="grid grid-cols-2 gap-px border-t border-line bg-line lg:grid-cols-4">
          {JBOOGX_FEED.map((post, i) => (
            <li key={post.path} className="bg-bg p-3 md:p-4">
              <div className="relative aspect-[9/16] w-full overflow-hidden bg-bg-sunken">
                <ReelCard post={post} index={String(i + 1).padStart(2, "0")} />
              </div>
            </li>
          ))}
        </ul>
        <div className="flex justify-center border-t border-line px-6 py-6">
          <Button
            href={JBOOGX.instagramUrl}
            target="_blank"
            rel="noopener noreferrer"
            variant="secondary"
          >
            <InstagramIcon className="h-4 w-4" />
            See more on Instagram
          </Button>
        </div>
      </CampaignSection>

      <CampaignSection
        id="craft"
        index="03"
        label="The craft"
        annotation="Seedance 2.5 in ArtCraft"
        title={
          <>
            The prompt is the <Accent>craft</Accent>.
          </>
        }
        lede="Tyler's videos are generated with Seedance 2.5 in ArtCraft, and the magic is in the writing. His prompts run long and read like shot lists, because the model gives back exactly as much direction as you put in. Learn the writing and the same studio answers to you."
      >
        <TipGrid items={JBOOGX_CRAFT} columns={3} icons={CRAFT_ICONS} />
        <p className="border-t border-line px-6 py-5 text-center text-sm leading-relaxed text-muted md:px-10">
          Everything above runs in the same ArtCraft anyone can open: same{" "}
          <Link href="/seedance2-5" className="text-ink underline underline-offset-4 decoration-line-strong hover:decoration-current">
            Seedance 2.5
          </Link>
          , same tools, no private build. The writing is a skill, and you can
          start learning it today.
        </p>
      </CampaignSection>

      <ClosingCta
        eyebrow="Free to start · Web or desktop"
        title={
          <>
            Make something worth a <Accent>follow</Accent>.
          </>
        }
        lede="Everything in Tyler's toolkit ships in ArtCraft: Seedance 2.5, the canvas, the whole studio."
        footnote="Your first drop is one prompt away."
      >
        <CampaignCta webLabel="Create now" webIcon="sparkles" />
      </ClosingCta>
    </>
  );
}

// Reel card: hosted thumbnail linking out to the post on Instagram.
function ReelCard({
  post,
  index,
  eager = false,
}: {
  post: InstagramPost;
  index: string;
  eager?: boolean;
}) {
  return (
    <a
      href={`https://www.instagram.com/${post.path}/`}
      target="_blank"
      rel="noopener noreferrer"
      aria-label={`${post.title}. Watch on Instagram.`}
      className="group absolute inset-0 block"
    >
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img
        src={post.image}
        alt={post.title}
        loading={eager ? "eager" : "lazy"}
        decoding="async"
        className="absolute inset-0 h-full w-full object-cover transition-transform duration-500 group-hover:scale-[1.02]"
      />
      <span className="absolute inset-0 bg-black/20 transition-colors group-hover:bg-black/10" />
      <span className="hud-label absolute top-3 right-3 text-white/80">{index}</span>
      <span className="hud-label absolute bottom-3 left-3 flex items-center gap-1.5 bg-invert-bg px-3 py-1.5 font-bold text-invert-fg">
        <PlayIcon aria-hidden className="h-3.5 w-3.5" />
        Instagram
      </span>
    </a>
  );
}
