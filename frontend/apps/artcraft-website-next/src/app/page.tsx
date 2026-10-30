import RevealManager from "@/components/reveal-manager";
import Hero from "@/components/landing/hero";
import CapabilityTicker from "@/components/landing/capability-ticker";
import CraftingApps from "@/components/landing/crafting-apps";
import Features from "@/components/landing/features";
import Ownership from "@/components/landing/ownership";
import StudioIntro from "@/components/landing/studio-intro";
import MadeWith from "@/components/landing/made-with";
import FinalCta from "@/components/landing/final-cta";

export default function Home() {
  return (
    <>
      <RevealManager />
      <Hero />
      <CraftingApps index="01" />
      {/* Everything from here down is ArtCraft Studio. */}
      <StudioIntro index="02" />
      <CapabilityTicker />
      <Features index="03" />
      <Ownership index="04" />
      <MadeWith index="05" />
      <FinalCta />
    </>
  );
}
