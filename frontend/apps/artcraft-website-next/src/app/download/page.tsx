import type { Metadata } from "next";
import RevealManager from "@/components/reveal-manager";
import DownloadPage from "@/components/download/download-page";

export const metadata: Metadata = {
  title: "Download ArtCraft — Windows and macOS",
  description:
    "Download ArtCraft for Windows and macOS. Start creating AI artwork today.",
  alternates: { canonical: "/download" },
};

export default function Download() {
  return (
    <>
      <RevealManager />
      <DownloadPage />
    </>
  );
}
