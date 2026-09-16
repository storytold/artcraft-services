import type { Metadata } from "next";
import { notFound, permanentRedirect } from "next/navigation";
import LiteYouTube from "@/components/lite-youtube";
import RevealManager from "@/components/reveal-manager";
import { SectionShell } from "@/components/landing/section-shell";
import {
  ArticleBody,
  BackLink,
  PageHeader,
} from "@/components/page/page-header";
import { getTutorialItemBySlug, getTutorialItems } from "@/lib/content";
import { extractYouTubeId, markdownToHtml } from "@/lib/markdown";

type Params = Promise<{ slug: string }>;

export const dynamicParams = false;

// Aliases are served too, then redirected to the canonical slug below.
export function generateStaticParams() {
  return getTutorialItems().flatMap((item) =>
    [item.slug, ...item.aliases].map((slug) => ({ slug })),
  );
}

export async function generateMetadata({
  params,
}: {
  params: Params;
}): Promise<Metadata> {
  const { slug } = await params;
  const item = getTutorialItemBySlug(slug);
  if (!item) return {};
  return {
    title: item.title,
    description: item.abstract,
    alternates: { canonical: `/tutorials/${item.slug}` },
  };
}

export default async function TutorialArticlePage({
  params,
}: {
  params: Params;
}) {
  const { slug } = await params;
  const item = getTutorialItemBySlug(slug);
  if (!item) notFound();
  if (item.slug !== slug.toLowerCase()) {
    permanentRedirect(`/tutorials/${item.slug}`);
  }

  const videoId = item.youtubeId ?? (item.videoUrl ? extractYouTubeId(item.videoUrl) : null);
  const html = markdownToHtml(item.body);
  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "Article",
    headline: item.title,
    description: item.abstract,
    articleBody: item.body,
  };

  return (
    <>
      <RevealManager />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />

      <PageHeader
        id="tutorials"
        index="01"
        label="Tutorials"
        annotation={item.category ? `${item.category} editor` : "Tutorial"}
        title={item.title}
        lede={item.abstract || undefined}
      >
        <div className="mt-8">
          <BackLink href="/tutorials">Back to Tutorials</BackLink>
        </div>
      </PageHeader>

      <SectionShell id="articles">
        {videoId && (
          <figure className="border-b border-line">
            <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-10">
              <figcaption className="hud-label text-muted">Video</figcaption>
              <p className="hud-label text-faint">Click to play</p>
            </div>
            <div className="relative aspect-video w-full overflow-hidden bg-bg-sunken">
              <LiteYouTube videoId={videoId} title={item.title} />
            </div>
          </figure>
        )}
        {html.trim() && <ArticleBody html={html} />}
      </SectionShell>
    </>
  );
}
