import type { Metadata } from "next";
import {
  AudioLinesIcon,
  CameraIcon,
  ClockIcon,
  FilmIcon,
  LayersIcon,
  PaintbrushIcon,
} from "lucide-react";
import CampaignHero from "@/components/campaign/campaign-hero";
import ExampleVideos from "@/components/campaign/example-videos";
import H3PromptBox from "@/components/campaign/h3-promptbox";
import {
  CampaignSection,
  ClosingCta,
  FaqAccordion,
  Prose,
  TipGrid,
} from "@/components/campaign/sections";
import { Accent } from "@/components/page/page-header";
import RevealManager from "@/components/reveal-manager";
import { Button } from "@/components/ui";
import {
  H3,
  H3_EXAMPLES,
  H3_FAQ,
  H3_HIGHLIGHTS,
  H3_OVERVIEW,
  H3_TIPS,
} from "@/lib/campaign-data";
import { webappUrl } from "@/lib/links";

export const metadata: Metadata = {
  title: "MiniMax H3 Free in ArtCraft. AI Video with Native Sound. Try It Now.",
  description:
    "Generate MiniMax H3 videos for free in ArtCraft. Up to 15 seconds of 2K AI video with native stereo sound. Type a prompt and try MiniMax's new multimodal model right on the page.",
  alternates: { canonical: "/minimax-h3" },
  openGraph: { title: "MiniMax H3, Free in ArtCraft" },
};

const HIGHLIGHT_ICONS = [
  CameraIcon,
  ClockIcon,
  AudioLinesIcon,
  LayersIcon,
  PaintbrushIcon,
  FilmIcon,
].map((Icon, i) => <Icon key={i} aria-hidden className="h-5 w-5" />);

export default function MinimaxH3Page() {
  const faqJsonLd = {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: H3_FAQ.map((item) => ({
      "@type": "Question",
      name: item.question,
      acceptedAnswer: { "@type": "Answer", text: item.answer },
    })),
  };

  return (
    <>
      <RevealManager />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(faqJsonLd) }}
      />

      <CampaignHero
        id="minimax-h3"
        label={H3.name}
        annotation="Generations are free right now"
        title={
          <>
            Try {H3.name}, <Accent>free</Accent> in ArtCraft.
          </>
        }
        lede="MiniMax's new multimodal model generates up to 15 seconds of 2K video with native stereo sound. Type a prompt and see for yourself: no credits, no charge."
      >
        <div className="flex justify-center">
          <H3PromptBox />
        </div>
      </CampaignHero>

      <CampaignSection
        id="overview"
        index="02"
        label="Overview"
        annotation="From MiniMax's announcement"
        title={
          <>
            What is <Accent>{H3.name}</Accent>?
          </>
        }
      >
        <Prose paragraphs={H3_OVERVIEW} />
      </CampaignSection>

      <CampaignSection
        id="highlights"
        index="03"
        label="Highlights"
        title={
          <>
            Built as <Accent>one</Accent> model.
          </>
        }
      >
        <TipGrid items={H3_HIGHLIGHTS} columns={3} icons={HIGHLIGHT_ICONS} />
      </CampaignSection>

      <CampaignSection
        id="examples"
        index="04"
        label="Examples"
        annotation="Turn the sound on"
        title={
          <>
            See it <Accent>(and hear it)</Accent> yourself.
          </>
        }
        lede="Sample generations from MiniMax's official H3 announcement. The audio is generated with the picture."
      >
        <ExampleVideos
          examples={H3_EXAMPLES}
          creditHref={H3.blogUrl}
          creditLabel="Video: MiniMax"
        />
      </CampaignSection>

      <CampaignSection
        id="tips"
        index="05"
        label="Prompt tips"
        title={
          <>
            Get more out of <Accent>{H3.name}</Accent>.
          </>
        }
        lede="A few habits that pay off with a model that listens this closely."
      >
        <TipGrid items={H3_TIPS} />
      </CampaignSection>

      <CampaignSection
        id="campaign-faq"
        index="06"
        label="FAQ"
        title={
          <>
            Frequently asked <Accent>questions</Accent>.
          </>
        }
      >
        <FaqAccordion items={H3_FAQ} />
      </CampaignSection>

      <ClosingCta
        eyebrow="Every model, one library"
        title={
          <>
            One model down. <Accent>Plenty</Accent> to go.
          </>
        }
        lede={`What you just used is the real thing: the ${H3.name} promptbox, settings and all. The ArtCraft app takes it from there. Run the same shot through Seedance, Veo, Sora, Kling, and every other leading model, keep every take in one library, and pick the winner.`}
      >
        <div className="flex flex-wrap items-center justify-center gap-3">
          <Button href={webappUrl("/create-video")} size="lg">
            Try more models
          </Button>
          <Button href="/" variant="secondary" size="lg">
            Learn more about ArtCraft
          </Button>
        </div>
      </ClosingCta>
    </>
  );
}
