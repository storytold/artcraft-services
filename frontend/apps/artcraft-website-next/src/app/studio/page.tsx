import type { Metadata } from "next";
import CapabilityTicker from "@/components/landing/capability-ticker";
import Features from "@/components/landing/features";
import FinalCta from "@/components/landing/final-cta";
import MadeWith from "@/components/landing/made-with";
import Ownership from "@/components/landing/ownership";
import StudioHero from "@/components/landing/studio-hero";
import RevealManager from "@/components/reveal-manager";

const TITLE = "ArtCraft Studio - Capable tools for artists";
const DESCRIPTION =
  "ArtCraft Studio is the open-source desktop app for generating AI video and images — built for artists who want real control.";

export const metadata: Metadata = {
  title: { absolute: TITLE },
  description: DESCRIPTION,
  alternates: { canonical: "/studio" },
  openGraph: {
    type: "website",
    url: "/studio",
    siteName: "ArtCraft",
    title: TITLE,
    description: DESCRIPTION,
    images: [{ url: "/images/og-image.png", width: 1200, height: 630 }],
  },
};

export default function StudioPage() {
  return (
    <>
      <RevealManager />
      <StudioHero />
      <CapabilityTicker />
      <Features index="01" />
      <Ownership index="02" />
      <MadeWith index="03" />
      <FinalCta />
    </>
  );
}
