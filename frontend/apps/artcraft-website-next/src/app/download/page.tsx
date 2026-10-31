import type { Metadata } from "next";
import RevealManager from "@/components/reveal-manager";
import CraftingAppsCallout from "@/components/download/crafting-apps-callout";
import DownloadPage from "@/components/download/download-page";

export const metadata: Metadata = {
  title: "Download ArtCraft Studio for Windows and macOS",
  description:
    "Download ArtCraft Studio, the generative AI app for images and video, for Windows and macOS.",
  alternates: { canonical: "/download" },
};

export default function Download() {
  return (
    <>
      <RevealManager />
      <DownloadPage aside={<CraftingAppsCallout />} />
    </>
  );
}
