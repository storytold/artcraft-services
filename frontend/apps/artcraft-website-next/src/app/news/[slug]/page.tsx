import type { Metadata } from "next";
import { notFound } from "next/navigation";
import RevealManager from "@/components/reveal-manager";
import { SectionShell } from "@/components/landing/section-shell";
import {
  ArticleBody,
  BackLink,
  PageHeader,
} from "@/components/page/page-header";
import { getNewsPostBySlug, getNewsPosts } from "@/lib/content";
import { mediaUrl } from "@/lib/links";
import { markdownToHtml } from "@/lib/markdown";

type Params = Promise<{ slug: string }>;

export const dynamicParams = false;

export function generateStaticParams() {
  return getNewsPosts().map((post) => ({ slug: post.slug }));
}

export async function generateMetadata({
  params,
}: {
  params: Params;
}): Promise<Metadata> {
  const { slug } = await params;
  const post = getNewsPostBySlug(slug);
  if (!post) return {};
  return {
    title: post.title,
    description: post.description,
    alternates: { canonical: `/news/${post.slug}` },
    openGraph: { type: "article", publishedTime: post.date },
  };
}

export default async function NewsPostPage({ params }: { params: Params }) {
  const { slug } = await params;
  const post = getNewsPostBySlug(slug);
  if (!post) notFound();

  // Post bodies reference images as `./images/blog/…`, relative to the
  // site root where they are hosted.
  const html = markdownToHtml(post.body, mediaUrl("/"));

  return (
    <>
      <RevealManager />

      <PageHeader
        id="news"
        index="01"
        label="News"
        annotation={post.date}
        title={post.title}
        lede={post.description || undefined}
      >
        <div className="mt-8">
          <BackLink href="/news">Back to News</BackLink>
        </div>
      </PageHeader>

      <SectionShell id="articles">
        {post.thumbnail && (
          <figure className="border-b border-line bg-bg-sunken">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={mediaUrl(post.thumbnail)}
              alt={post.title}
              className="mx-auto block h-auto w-full max-w-3xl"
            />
          </figure>
        )}
        <ArticleBody html={html} />
      </SectionShell>
    </>
  );
}
