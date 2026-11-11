import type { Metadata } from "next";
import {
  CRAFTING_APPS_COVERAGE,
  CraftingAppsClosing,
  CraftingAppsOverview,
} from "@/components/apps/crafting-apps-overview";
import RevealManager from "@/components/reveal-manager";
import {
  CRAFTING_APPS_COUNT_WORD,
  CRAFTING_APPS_OG_IMAGE,
} from "@/lib/crafting-apps";

const TITLE = "Crafting Apps: open-source creative tools";
const DESCRIPTION = `${CRAFTING_APPS_COVERAGE}: ${CRAFTING_APPS_COUNT_WORD.toLowerCase()} native, open-source apps from the ArtCraft team, built in Rust and free to use.`;

export const metadata: Metadata = {
  title: TITLE,
  description: DESCRIPTION,
  alternates: { canonical: "/apps" },
  openGraph: {
    type: "website",
    url: "/apps",
    siteName: "ArtCraft",
    title: TITLE,
    description: DESCRIPTION,
    images: [{ url: CRAFTING_APPS_OG_IMAGE, width: 1200, height: 630 }],
  },
};

export default function CraftingAppsPage() {
  return (
    <>
      <RevealManager />
      <CraftingAppsOverview />
      <CraftingAppsClosing />
    </>
  );
}
