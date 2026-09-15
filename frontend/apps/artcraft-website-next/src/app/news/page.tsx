import type { Metadata } from "next";
import Link from "next/link";
import { ArrowRightIcon } from "lucide-react";
import RevealManager from "@/components/reveal-manager";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Accent, PageHeader } from "@/components/page/page-header";
import { getNewsPosts } from "@/lib/content";

export const metadata: Metadata = {
  title: "News & Updates",
  description:
    "Latest updates, features, and announcements from the ArtCraft team.",
  alternates: { canonical: "/news" },
};

export default function NewsIndexPage() {
  const posts = getNewsPosts();

  return (
    <>
      <RevealManager />

      <PageHeader
        id="news"
        index="01"
        label="News"
        annotation={`${posts.length} posts`}
        title={
          <>
            News &amp; <Accent>updates</Accent>.
          </>
        }
        lede="Latest updates, features, and announcements from the ArtCraft team."
      />

      <SectionShell id="articles">
        <SectionEyebrow index="02" label="Posts" annotation="Newest first" />
        <ol data-reveal-group className="flex flex-col gap-px bg-line">
          {posts.map((post) => (
            <li key={post.slug} data-reveal className="bg-bg">
              <Link
                href={`/news/${post.slug}`}
                className="group grid gap-4 p-6 hover:bg-bg-raised md:grid-cols-[10rem_1fr] md:p-8"
              >
                <p className="hud-label text-faint">{post.date}</p>
                <div>
                  <h2 className="flex items-start gap-2 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
                    <span className="flex-1">{post.title}</span>
                    <ArrowRightIcon
                      aria-hidden
                      className="mt-1.5 h-4 w-4 shrink-0 text-faint transition-transform group-hover:translate-x-1 group-hover:text-ink"
                    />
                  </h2>
                  <p className="mt-2 max-w-2xl leading-relaxed text-muted">
                    {post.description}
                  </p>
                </div>
              </Link>
            </li>
          ))}
          {posts.length === 0 && (
            <li className="bg-bg p-6 text-muted md:p-8">
              No updates yet. Check back soon.
            </li>
          )}
        </ol>
      </SectionShell>
    </>
  );
}
