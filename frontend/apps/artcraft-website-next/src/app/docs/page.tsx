import type { Metadata } from "next";
import Link from "next/link";
import { ArrowRightIcon } from "lucide-react";
import DocsNavigation from "@/components/docs/docs-navigation";
import { getCraftDocs, docsSlugHref } from "@/lib/craft-docs";
import { renderDocsMarkdown } from "@/lib/docs-markdown";
import { ArticleBody } from "@/components/page/page-header";

export const revalidate = 300;

export const metadata: Metadata = {
  title: "Documentation",
  description: "Guides and documentation for ArtCraft and the Craft Apps.",
  alternates: { canonical: "/docs" },
};

export default async function DocsIndexPage() {
  const docs = await getCraftDocs();
  const intro = docs.find((doc) => doc.slug === "");
  const html = intro ? await renderDocsMarkdown(intro.body, intro.sourcePath) : "";
  const pages = docs.filter((doc) => doc.slug !== "");

  return (
    <main>
      <header className="border-b border-line px-6 py-14 md:px-10 md:py-20">
        <p className="hud-label mb-5 text-faint">ARTCRAFT / KNOWLEDGE BASE</p>
        <h1 className="font-display text-4xl font-medium tracking-[-0.035em] text-ink-strong sm:text-5xl">
          Documentation
        </h1>
        <p className="mt-5 max-w-2xl text-lg leading-relaxed text-muted">
          Guides, setup instructions, and product references for ArtCraft and the Craft Apps.
        </p>
      </header>

      <div className="grid lg:grid-cols-[16rem_minmax(0,1fr)]">
        <DocsNavigation docs={docs} />
        <section className="min-w-0">
          {html && <ArticleBody html={html} />}
          {pages.length ? (
            <div className="mx-auto max-w-4xl px-6 py-10 md:px-10">
              <p className="hud-label mb-5 text-faint">GUIDES · {pages.length}</p>
              <ul className="divide-y divide-line border-y border-line">
                {pages.map((doc) => (
                  <li key={doc.slug}>
                    <Link
                      href={docsSlugHref(doc.slug)}
                      className="group flex items-center justify-between gap-6 py-5"
                    >
                      <span>
                        <span className="block font-display text-xl text-ink-strong group-hover:text-accent-ink">
                          {doc.title}
                        </span>
                        {doc.description && (
                          <span className="mt-1 block text-sm leading-relaxed text-muted">
                            {doc.description}
                          </span>
                        )}
                      </span>
                      <ArrowRightIcon aria-hidden className="h-4 w-4 shrink-0 text-faint transition-transform group-hover:translate-x-1" />
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          ) : (
            <div className="mx-auto max-w-3xl px-6 py-12 md:px-10 md:py-16">
              <p className="text-muted">
                Documentation is on its way. The guides are maintained in the community repository.
              </p>
              <a
                href="https://github.com/storytold/craft-documentation"
                target="_blank"
                rel="noreferrer"
                className="hud-label mt-6 inline-flex border-b border-line-strong pb-1 text-ink hover:text-accent-ink"
              >
                Browse or contribute on GitHub ↗
              </a>
            </div>
          )}
        </section>
      </div>
    </main>
  );
}
