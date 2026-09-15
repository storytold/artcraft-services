import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { parseFrontmatter } from "./markdown";

// Markdown content loaders, ported from @storyteller/markdown-content.
//
// Platform difference: the Vite site pulled these files in with
// `import.meta.glob(..., { eager: true })`, which bundles every article into
// the client. Next has no glob import; instead these run on the server (build
// time for the static pages, via generateStaticParams) and read the files
// straight from disk. SERVER ONLY — never import this module from a
// "use client" component; pass the resulting plain objects down as props.

const CONTENT_ROOT = join(process.cwd(), "src", "content");

export type NewsPost = {
  slug: string;
  title: string;
  description: string;
  date: string;
  body: string;
  thumbnail?: string;
};

export type FaqItem = {
  slug: string;
  title: string;
  description: string;
  body: string;
  isPublished: boolean;
  thumbnail?: string;
};

export type TutorialItem = {
  slug: string;
  title: string;
  abstract: string;
  category?: string;
  thumbnail?: string;
  videoUrl?: string;
  youtubeId?: string;
  aliases: string[];
  body: string;
  isPublished: boolean;
};

export function getNewsPosts(): NewsPost[] {
  return readCollection("news")
    .map(({ slug, frontmatter, body }) => ({
      slug,
      title: frontmatter.title || slug,
      description: frontmatter.abstract || "",
      date: frontmatter.date || "",
      thumbnail: frontmatter.thumbnail,
      body,
    }))
    .sort((a, b) => {
      if (a.date && b.date) {
        return new Date(b.date).getTime() - new Date(a.date).getTime();
      }
      return 0;
    });
}

export function getNewsPostBySlug(slug: string): NewsPost | null {
  return getNewsPosts().find((post) => post.slug === slug) ?? null;
}

export function getFaqItems(): FaqItem[] {
  return readCollection("faq")
    .map(({ slug, frontmatter, body }) => ({
      slug,
      title: frontmatter.title || slug,
      description: frontmatter.abstract || "",
      body,
      isPublished: frontmatter.isPublished !== "false",
      thumbnail: frontmatter.thumbnail,
    }))
    .filter((item) => item.isPublished);
}

export function getFaqItemBySlug(slug: string): FaqItem | null {
  return getFaqItems().find((item) => item.slug === slug) ?? null;
}

export function getTutorialItems(): TutorialItem[] {
  return readCollection("tutorials")
    .map(({ slug, frontmatter, body }) => ({
      slug: (frontmatter.slug || slug).trim().toLowerCase(),
      title: frontmatter.title || slug,
      abstract: frontmatter.abstract || "",
      category: frontmatter.category,
      thumbnail: frontmatter.thumbnail,
      videoUrl: frontmatter.videoUrl,
      youtubeId: frontmatter.youtubeId,
      aliases: frontmatter.aliases
        ? frontmatter.aliases
            .split(/[\s,]+/)
            .map((s) => s.trim().toLowerCase())
            .filter(Boolean)
        : [],
      body,
      isPublished: frontmatter.isPublished !== "false",
    }))
    .filter((item) => item.isPublished);
}

export function getTutorialItemBySlug(slug: string): TutorialItem | null {
  const wanted = slug.toLowerCase();
  return (
    getTutorialItems().find(
      (item) => item.slug === wanted || item.aliases.includes(wanted),
    ) ?? null
  );
}

// Reads every .md file in a content folder, sorted by filename so output is
// deterministic across platforms (readdir order is not guaranteed).
function readCollection(folder: "news" | "faq" | "tutorials") {
  const dir = join(CONTENT_ROOT, folder);
  return readdirSync(dir)
    .filter((file) => file.endsWith(".md"))
    .sort()
    .map((file) => {
      const { frontmatter, body } = parseFrontmatter(
        readFileSync(join(dir, file), "utf8"),
      );
      return { slug: file.replace(/\.md$/, ""), frontmatter, body };
    });
}
