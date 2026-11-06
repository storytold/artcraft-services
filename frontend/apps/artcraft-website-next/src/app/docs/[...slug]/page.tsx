import type { Metadata } from "next";
import { notFound } from "next/navigation";
import DocsNavigation from "@/components/docs/docs-navigation";
import { ArticleBody, BackLink } from "@/components/page/page-header";
import { docsSourceUrl, getCraftDoc, getCraftDocs } from "@/lib/craft-docs";
import { renderDocsMarkdown } from "@/lib/docs-markdown";

export const revalidate = 300;

type Params = Promise<{ slug: string[] }>;

export async function generateMetadata({ params }: { params: Params }): Promise<Metadata> {
  const { slug } = await params;
  const doc = await getCraftDoc(slug);
  if (!doc) return {};
  return {
    title: doc.title,
    description: doc.description || `Documentation for ${doc.title}.`,
    alternates: { canonical: `/docs/${slug.join("/")}` },
  };
}

export default async function DocsPage({ params }: { params: Params }) {
  const { slug } = await params;
  const [doc, docs] = await Promise.all([getCraftDoc(slug), getCraftDocs()]);
  if (!doc) notFound();

  const html = await renderDocsMarkdown(doc.body, doc.sourcePath);
  return (
    <main>
      <header className="border-b border-line px-6 py-10 md:px-10 md:py-14">
        <p className="hud-label mb-5 text-faint">ARTCRAFT / DOCUMENTATION</p>
        <h1 className="max-w-4xl font-display text-3xl font-medium tracking-[-0.035em] text-ink-strong sm:text-4xl">
          {doc.title}
        </h1>
        {doc.description && (
          <p className="mt-4 max-w-2xl text-lg leading-relaxed text-muted">
            {doc.description}
          </p>
        )}
        <div className="mt-6 flex flex-wrap gap-x-6 gap-y-3">
          <BackLink href="/docs">All documentation</BackLink>
          <a
            href={docsSourceUrl(doc.sourcePath)}
            target="_blank"
            rel="noreferrer"
            className="hud-label text-muted underline decoration-line-strong underline-offset-4 hover:text-ink"
          >
            Edit this page ↗
          </a>
        </div>
      </header>
      <div className="grid lg:grid-cols-[16rem_minmax(0,1fr)]">
        <DocsNavigation docs={docs} activeSlug={doc.slug} />
        <section className="min-w-0">
          <ArticleBody html={html} />
        </section>
      </div>
    </main>
  );
}
