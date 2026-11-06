import { parseFrontmatter } from "./markdown";

const DOCS_REPOSITORY = "storytold/craft-documentation";
const DOCS_BRANCH = "main";
const DOCS_TREE_URL = `https://api.github.com/repos/${DOCS_REPOSITORY}/git/trees/${DOCS_BRANCH}?recursive=1`;
const DOCS_RAW_BASE = `https://raw.githubusercontent.com/${DOCS_REPOSITORY}/${DOCS_BRANCH}`;

export type CraftDoc = {
  slug: string;
  title: string;
  description: string;
  body: string;
  sourcePath: string;
};

type GitTreeResponse = {
  tree?: Array<{ path: string; type: string }>;
};

const isMarkdown = (path: string) => /\.mdx?$/i.test(path);
const slugForPath = (path: string) =>
  path.replace(/\.mdx?$/i, "").replace(/(^|\/)index$/i, "").replace(/\/$/, "");

function titleFromMarkdown(path: string, body: string, frontmatterTitle?: string) {
  if (frontmatterTitle) return frontmatterTitle;
  const heading = body.match(/^#\s+(.+)\s*$/m)?.[1];
  if (heading) return heading.replace(/[`*_]/g, "");
  const filename = path.split("/").at(-1)?.replace(/\.mdx?$/i, "") ?? path;
  return filename.replace(/[-_]/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}

async function fetchDocsTree(): Promise<string[]> {
  try {
    const response = await fetch(DOCS_TREE_URL, {
      headers: { Accept: "application/vnd.github+json" },
      next: { revalidate: 300 },
    });
    if (!response.ok) return [];
    const result = (await response.json()) as GitTreeResponse;
    return (result.tree ?? [])
      .filter((entry) => entry.type === "blob" && isMarkdown(entry.path))
      .map((entry) => entry.path)
      .sort((a, b) => a.localeCompare(b));
  } catch {
    return [];
  }
}

async function fetchDoc(path: string): Promise<CraftDoc | null> {
  try {
    const response = await fetch(`${DOCS_RAW_BASE}/${path.split("/").map(encodeURIComponent).join("/")}`, {
      next: { revalidate: 300 },
    });
    if (!response.ok) return null;
    const parsed = parseFrontmatter(await response.text());
    const slug = slugForPath(path);
    return {
      slug,
      title: titleFromMarkdown(path, parsed.body, parsed.frontmatter.title),
      description: parsed.frontmatter.description ?? parsed.frontmatter.abstract ?? "",
      body: parsed.body,
      sourcePath: path,
    };
  } catch {
    return null;
  }
}

export async function getCraftDocs(): Promise<CraftDoc[]> {
  const paths = await fetchDocsTree();
  const docs = await Promise.all(paths.map(fetchDoc));
  return docs
    .filter((doc): doc is CraftDoc => !!doc)
}

export async function getCraftDoc(slug: string[]): Promise<CraftDoc | null> {
  const wantedSlug = slug.join("/");
  const paths = await fetchDocsTree();
  const path = paths.find((candidate) => slugForPath(candidate) === wantedSlug);
  return path ? fetchDoc(path) : null;
}

export function docsSourceUrl(path: string) {
  return `https://github.com/${DOCS_REPOSITORY}/blob/${DOCS_BRANCH}/${path.split("/").map(encodeURIComponent).join("/")}`;
}

export function docsSlugHref(slug: string) {
  return slug ? `/docs/${slug.split("/").map(encodeURIComponent).join("/")}` : "/docs";
}
