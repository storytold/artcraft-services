import type { Metadata } from "next";
import { notFound } from "next/navigation";
import RevealManager from "@/components/reveal-manager";
import { SectionShell } from "@/components/landing/section-shell";
import {
  ArticleBody,
  BackLink,
  PageHeader,
} from "@/components/page/page-header";
import { getFaqItemBySlug, getFaqItems } from "@/lib/content";
import { mediaUrl } from "@/lib/links";
import { markdownToHtml } from "@/lib/markdown";

type Params = Promise<{ slug: string }>;

export const dynamicParams = false;

export function generateStaticParams() {
  return getFaqItems().map((item) => ({ slug: item.slug }));
}

export async function generateMetadata({
  params,
}: {
  params: Params;
}): Promise<Metadata> {
  const { slug } = await params;
  const item = getFaqItemBySlug(slug);
  if (!item) return {};
  return {
    title: item.title,
    description: item.description,
    alternates: { canonical: `/faq/${item.slug}` },
  };
}

export default async function FaqArticlePage({ params }: { params: Params }) {
  const { slug } = await params;
  const item = getFaqItemBySlug(slug);
  if (!item) notFound();

  const html = markdownToHtml(item.body, "/faq/");
  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "Article",
    headline: item.title,
    description: item.description,
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
        id="faq"
        index="01"
        label="FAQ"
        annotation="Guide"
        title={item.title}
        lede={item.description || undefined}
      >
        <div className="mt-8">
          <BackLink href="/faq">Back to FAQ</BackLink>
        </div>
      </PageHeader>

      <SectionShell id="articles">
        {item.thumbnail && (
          <figure className="border-b border-line bg-bg-sunken">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={mediaUrl(item.thumbnail)}
              alt={item.title}
              className="mx-auto block h-auto w-full max-w-3xl"
            />
          </figure>
        )}
        <ArticleBody html={html} />
      </SectionShell>
    </>
  );
}
