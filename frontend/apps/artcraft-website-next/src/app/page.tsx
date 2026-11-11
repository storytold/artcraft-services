import {
  CraftingAppsClosing,
  CraftingAppsOverview,
} from "@/components/apps/crafting-apps-overview";
import Hero from "@/components/landing/hero";
import StudioIntro from "@/components/landing/studio-intro";
import RevealManager from "@/components/reveal-manager";

export default function Home() {
  return (
    <>
      <RevealManager />
      <Hero />
      <CraftingAppsOverview headingAs="h2" />
      <StudioIntro index="04" />
      <CraftingAppsClosing />
    </>
  );
}
