import { marked } from "marked";
import sanitizeHtml from "sanitize-html";

const DOCS_RAW_BASE = "https://raw.githubusercontent.com/storytold/craft-documentation/main";

function safeDocHref(href: string, sourcePath: string): string {
  if (/^(?:https?:|mailto:|tel:|#|\/)/i.test(href)) return href;
  const [file, fragment] = href.split("#", 2);
  if (!file) return `#${fragment ?? ""}`;
  const sourceDirectory = sourcePath.split("/").slice(0, -1);
  for (const segment of file.split("/")) {
    if (!segment || segment === ".") continue;
    if (segment === "..") sourceDirectory.pop();
    else sourceDirectory.push(segment);
  }
  const normalized = sourceDirectory.join("/");
  if (/\.mdx?$/i.test(normalized)) {
    const slug = normalized.replace(/\.mdx?$/i, "").replace(/(^|\/)index$/i, "").replace(/\/$/, "");
    return `/docs/${slug.split("/").map(encodeURIComponent).join("/")}${fragment ? `#${fragment}` : ""}`;
  }
  return `${DOCS_RAW_BASE}/${normalized.split("/").map(encodeURIComponent).join("/")}${fragment ? `#${fragment}` : ""}`;
}

export async function renderDocsMarkdown(markdown: string, sourcePath: string): Promise<string> {
  const renderer = new marked.Renderer();
  renderer.link = ({ href, title, text }) => {
    const safeHref = safeDocHref(href, sourcePath);
    const titleAttribute = title ? ` title="${title.replaceAll("&", "&amp;").replaceAll('"', "&quot;")}"` : "";
    const external = /^https?:\/\//i.test(safeHref);
    return `<a href="${safeHref.replaceAll("&", "&amp;").replaceAll('"', "&quot;")}"${titleAttribute}${external ? ' target="_blank" rel="noopener noreferrer"' : ""}>${text}</a>`;
  };
  renderer.image = ({ href, title, text }) => {
    const src = /^(?:https?:|\/)/i.test(href)
      ? href
      : `${DOCS_RAW_BASE}/${sourcePath.split("/").slice(0, -1).concat(href.split("/")).map(encodeURIComponent).join("/")}`;
    const titleAttribute = title ? ` title="${title}"` : "";
    return `<img src="${src}" alt="${text}"${titleAttribute} loading="lazy" />`;
  };
  const html = await marked.parse(markdown, { gfm: true, breaks: false, renderer });
  return sanitizeHtml(html, {
    allowedTags: sanitizeHtml.defaults.allowedTags.concat([
      "img", "h1", "h2", "h3", "h4", "h5", "h6", "del", "input", "pre", "code",
    ]),
    allowedAttributes: {
      ...sanitizeHtml.defaults.allowedAttributes,
      a: ["href", "name", "target", "rel", "title"],
      img: ["src", "alt", "title", "loading"],
      code: ["class"],
      input: ["type", "checked", "disabled"],
    },
    allowedSchemes: ["http", "https", "mailto", "tel"],
    transformTags: {
      a: sanitizeHtml.simpleTransform("a", { rel: "noopener noreferrer" }, true),
    },
  });
}
