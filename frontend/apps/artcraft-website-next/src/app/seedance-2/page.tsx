import type { Metadata } from "next";
import CampaignCta from "@/components/campaign/campaign-cta";
import CampaignHero, { VimeoEmbed } from "@/components/campaign/campaign-hero";
import Reasons from "@/components/campaign/reasons";
import { ClosingCta, CommunityCta } from "@/components/campaign/sections";
import Features from "@/components/landing/features";
import MadeWith from "@/components/landing/made-with";
import Ownership from "@/components/landing/ownership";
import { Accent } from "@/components/page/page-header";
import RevealManager from "@/components/reveal-manager";
import { HERO_VIMEO_URL } from "@/lib/campaign-data";

export const metadata: Metadata = {
  title: "Seedance 2.0 in ArtCraft. AI Video Generation. Fast and Open Desktop App.",
  description:
    "Seedance 2.0 is available now in ArtCraft. Generate jaw-dropping AI videos with the fastest open desktop app for artists.",
  alternates: { canonical: "/seedance-2" },
};

export default function SeedanceTwoPage() {
  return (
    <>
      <RevealManager />

      <CampaignHero
        id="seedance-2"
        label="Seedance 2.0"
        annotation="Available today in ArtCraft"
        title={
          <>
            Seedance 2.0, now in <Accent>ArtCraft</Accent>.
          </>
        }
        lede="Generate jaw-dropping AI videos with Seedance 2.0 before it's available anywhere else."
        media={<VimeoEmbed src={HERO_VIMEO_URL} title="Seedance in ArtCraft" />}
        mediaCaption="Seedance 2.0 in ArtCraft"
      >
        <CampaignCta webLabel="Get now" webIcon="sparkles" />
      </CampaignHero>

      <Features index="02" />
      <Ownership index="03" />
      <Reasons index="04" />
      <MadeWith index="05" />
      <CommunityCta />

      <ClosingCta
        eyebrow="Free to use"
        title={
          <>
            Ready to <Accent>create</Accent>?
          </>
        }
        lede="Join thousands of artists and filmmakers using ArtCraft to bring their vision to life."
      >
        <CampaignCta webLabel="Get now" webIcon="sparkles" />
      </ClosingCta>
    </>
  );
}
