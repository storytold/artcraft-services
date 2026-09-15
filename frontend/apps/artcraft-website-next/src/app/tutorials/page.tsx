import type { Metadata } from "next";
import RevealManager from "@/components/reveal-manager";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Accent, PageHeader } from "@/components/page/page-header";
import TutorialGrid from "@/components/tutorials/tutorial-grid";
import { getTutorialItems } from "@/lib/content";

export const metadata: Metadata = {
  title: "Tutorials",
  description: "Learn tips, tricks, and workflows for ArtCraft.",
  alternates: { canonical: "/tutorials" },
};

export default function TutorialsPage() {
  // Only the fields the client grid renders cross the server boundary.
  const items = getTutorialItems().map(({ slug, title, category, thumbnail }) => ({
    slug,
    title,
    category,
    thumbnail,
  }));

  return (
    <>
      <RevealManager />

      <PageHeader
        id="tutorials"
        index="01"
        label="Tutorials"
        annotation="Video walkthroughs"
        title={
          <>
            Learn the <Accent>craft</Accent>.
          </>
        }
        lede="Learn tips, tricks, and workflows for ArtCraft."
      />

      <SectionShell id="articles">
        <SectionEyebrow index="02" label="Library" annotation="Filter by editor" />
        <TutorialGrid items={items} />
      </SectionShell>
    </>
  );
}
