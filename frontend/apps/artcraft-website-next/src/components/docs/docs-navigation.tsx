import Link from "next/link";
import { docsSlugHref, type CraftDoc } from "@/lib/craft-docs";

export default function DocsNavigation({
  docs,
  activeSlug,
}: {
  docs: CraftDoc[];
  activeSlug?: string;
}) {
  const pages = docs.filter((doc) => doc.slug !== "");
  return (
    <aside className="border-b border-line px-6 py-8 md:px-8 lg:sticky lg:top-12 lg:h-[calc(100vh-3rem)] lg:overflow-y-auto lg:border-b-0 lg:border-r">
      <Link href="/docs" className="hud-label text-ink-strong hover:text-accent-ink">
        Documentation
      </Link>
      {pages.length ? (
        <nav aria-label="Documentation pages" className="mt-6">
          <ul className="space-y-1">
            {pages.map((doc) => (
              <li key={doc.slug}>
                <Link
                  href={docsSlugHref(doc.slug)}
                  aria-current={activeSlug === doc.slug ? "page" : undefined}
                  className={`block border-l px-3 py-2 text-sm transition-colors ${
                    activeSlug === doc.slug
                      ? "border-accent-ink text-ink-strong"
                      : "border-line text-muted hover:border-line-strong hover:text-ink"
                  }`}
                >
                  {doc.title}
                </Link>
              </li>
            ))}
          </ul>
        </nav>
      ) : (
        <p className="mt-5 text-sm leading-relaxed text-muted">
          The guide library is getting started. Check back soon.
        </p>
      )}
      <a
        href="https://github.com/storytold/craft-documentation"
        target="_blank"
        rel="noreferrer"
        className="hud-label mt-8 inline-flex text-muted underline decoration-line-strong underline-offset-4 hover:text-ink"
      >
        Edit on GitHub ↗
      </a>
    </aside>
  );
}
