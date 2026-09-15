import type { Metadata } from "next";
import CampaignCta from "@/components/campaign/campaign-cta";
import CampaignHero, { VimeoEmbed } from "@/components/campaign/campaign-hero";
import Manifesto from "@/components/campaign/manifesto";
import Reasons from "@/components/campaign/reasons";
import {
  CampaignSection,
  ClosingCta,
  CommunityCta,
  FaqAccordion,
  Prose,
  TipGrid,
} from "@/components/campaign/sections";
import Features from "@/components/landing/features";
import MadeWith from "@/components/landing/made-with";
import Ownership from "@/components/landing/ownership";
import { Accent } from "@/components/page/page-header";
import RevealManager from "@/components/reveal-manager";
import {
  HERO_VIMEO_URL,
  SD25_DISCLAIMER,
  SD25_FAQ,
  SD25_OVERVIEW,
  SD25_TIPS,
} from "@/lib/campaign-data";

export const metadata: Metadata = {
  title: "Seedance 2.5 in ArtCraft. AI Video Generation. Fast and Open Desktop App.",
  description:
    "Seedance 2.5 is ByteDance's anticipated next-generation AI video model. Reports point to 4K, real-time generation, longer clips, and persistent characters. See what's expected, and create with Seedance in ArtCraft today.",
  alternates: { canonical: "/seedance2-5" },
  openGraph: { title: "Seedance 2.5" },
};

export default function SeedanceTwoFivePage() {
  const faqJsonLd = {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: SD25_FAQ.map((item) => ({
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
        id="seedance-2-5"
        label="Seedance 2.5"
        annotation="Coming to ArtCraft first"
        title={
          <>
            Seedance 2.5, soon in <Accent>ArtCraft</Accent>.
          </>
        }
        lede="Generate jaw-dropping AI videos with Seedance 2.5 before it's available anywhere else."
        media={<VimeoEmbed src={HERO_VIMEO_URL} title="Seedance 2.5 in ArtCraft" />}
        mediaCaption="Seedance in ArtCraft today"
      >
        <CampaignCta />
      </CampaignHero>

      <CampaignSection
        id="overview"
        index="02"
        label="Overview"
        annotation="Rumor vs. fact, flagged"
        title={
          <>
            What is <Accent>Seedance 2.5</Accent>?
          </>
        }
      >
        <Prose paragraphs={SD25_OVERVIEW} note={SD25_DISCLAIMER} />
      </CampaignSection>

      <CampaignSection
        id="tips"
        index="03"
        label="Prompt tips"
        annotation="Carries over to 2.5"
        title={
          <>
            Get more out of <Accent>Seedance</Accent>.
          </>
        }
        lede="A few habits that pay off with Seedance today, and will carry right over to 2.5 when it arrives."
      >
        <TipGrid items={SD25_TIPS} />
      </CampaignSection>

      <CampaignSection
        id="campaign-faq"
        index="04"
        label="FAQ"
        title={
          <>
            Frequently asked <Accent>questions</Accent>.
          </>
        }
      >
        <FaqAccordion items={SD25_FAQ} />
      </CampaignSection>

      <Manifesto />

      <Features index="05" />
      <Ownership index="06" />
      <Reasons index="07" />
      <MadeWith index="08" />
      <CommunityCta />

      <ClosingCta
        eyebrow="Free to download"
        title={
          <>
            Ready to <Accent>craft</Accent>?
          </>
        }
        lede="Join thousands of artists and filmmakers using ArtCraft to bring their vision to life."
      >
        <CampaignCta />
      </ClosingCta>
    </>
  );
}
