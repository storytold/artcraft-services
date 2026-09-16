// Minimal markdown renderer, ported from @storyteller/markdown-content. It
// covers exactly the subset our content files use (ATX headings, paragraphs,
// "- " lists, blockquotes, bold/italic, links, images, and the @youtube /
// @video / @loop_autoplay / @image directives). Output is styled by the
// `.article-content` rules in globals.css — no inline styling here, so the
// same markup renders correctly in both themes.

export type Frontmatter = Record<string, string>;

export type FrontmatterResult = {
  frontmatter: Frontmatter;
  body: string;
};

const VIDEO_EXTENSIONS = [".mp4", ".webm", ".ogg", ".mov", ".m4v", ".avi"];

const YOUTUBE_PATTERNS = [
  /(?:youtube\.com\/watch\?v=|youtu\.be\/|youtube\.com\/embed\/|youtube\.com\/v\/)([a-zA-Z0-9_-]{11})/,
  /youtube\.com\/shorts\/([a-zA-Z0-9_-]{11})/,
];

export function parseFrontmatter(raw: string): FrontmatterResult {
  const text = raw.replace(/\r\n/g, "\n");
  if (!text.startsWith("---\n")) {
    return { frontmatter: {}, body: text };
  }
  const end = text.indexOf("\n---\n", 4);
  if (end === -1) return { frontmatter: {}, body: text };
  const header = text.slice(4, end);
  const body = text.slice(end + 5);
  const frontmatter: Frontmatter = {};
  for (const line of header.split("\n")) {
    const idx = line.indexOf(":");
    if (idx === -1) continue;
    const key = line.slice(0, idx).trim();
    const value = line
      .slice(idx + 1)
      .trim()
      .replace(/^"|^'|"$|'$/g, "");
    if (key) frontmatter[key] = value;
  }
  return { frontmatter, body };
}

// Extracts a YouTube video id from a bare id or any common YouTube URL form.
export function extractYouTubeId(input: string): string | null {
  const trimmed = input.trim();
  if (/^[a-zA-Z0-9_-]{11}$/.test(trimmed)) return trimmed;
  for (const pattern of YOUTUBE_PATTERNS) {
    const match = trimmed.match(pattern);
    if (match) return match[1];
  }
  return null;
}

export function markdownToHtml(markdown: string, basePath = ""): string {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  const html: string[] = [];
  let inList = false;
  let blockquoteLines: string[] = [];
  let paragraphLines: string[] = [];

  const isAbsolute = (url: string) => /^(?:[a-z]+:)?\/\//i.test(url);
  const normalizeLink = (url: string): string => {
    if (!basePath || !url) return url;
    if (url.startsWith("#") || url.startsWith("/") || isAbsolute(url)) {
      return url;
    }
    return basePath + url.replace(/^\.\.?\//, "");
  };

  const isVideoUrl = (url: string) => {
    const lower = url.toLowerCase().split("?")[0];
    return VIDEO_EXTENSIONS.some((ext) => lower.endsWith(ext));
  };

  const videoMimeType = (url: string) => {
    const lower = url.toLowerCase().split("?")[0];
    if (lower.endsWith(".webm")) return "webm";
    if (lower.endsWith(".ogg")) return "ogg";
    return "mp4";
  };

  const escapeAttr = (value: string) => value.replace(/"/g, "&quot;");

  const renderYouTube = (videoId: string) =>
    `<div class="video-embed youtube-embed"><iframe src="https://www.youtube-nocookie.com/embed/${videoId}" loading="lazy" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" allowfullscreen></iframe></div>`;

  const renderVideo = (url: string, loop: boolean) => {
    const src = escapeAttr(normalizeLink(url));
    const attrs = loop
      ? "autoplay loop muted playsinline"
      : 'controls preload="metadata"';
    return `<div class="video-embed"><video ${attrs}><source src="${src}" type="video/${videoMimeType(url)}">Your browser does not support the video tag.</video></div>`;
  };

  // Optional "WIDTHxHEIGHT" / "WIDTH" size suffix (from `![alt](url|300x200)`
  // or `@image(url, 300)`); bare numbers are pixels.
  const renderImage = (url: string, alt = "", sizeStr = "") => {
    const src = escapeAttr(normalizeLink(url.trim()));
    const [width, height] = sizeStr.toLowerCase().split("x");
    const toCss = (v?: string) =>
      v ? (v.endsWith("%") || v.endsWith("px") ? v : `${v}px`) : undefined;
    const style = [
      toCss(width) && `width:${toCss(width)}`,
      toCss(height) && `height:${toCss(height)}`,
      width && width !== "100%" && "margin-left:auto;margin-right:auto",
    ]
      .filter(Boolean)
      .join(";");
    return `<img src="${src}" alt="${escapeAttr(alt)}" loading="lazy"${style ? ` style="${style}"` : ""} />`;
  };

  const renderInline = (text: string): string => {
    let out = text.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
    out = out.replace(/\*([^*]+)\*/g, "<em>$1</em>");
    out = out.replace(/!\[([^\]]*)\]\(([^)]+)\)/g, (_m, alt, urlAndSize) => {
      const [url, size] = String(urlAndSize).split("|");
      const safeUrl = String(url ?? "").trim();
      return isVideoUrl(safeUrl)
        ? renderVideo(safeUrl, false)
        : renderImage(safeUrl, alt, size ?? "");
    });
    out = out.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_m, label, url) => {
      const href = escapeAttr(normalizeLink(String(url ?? "")));
      const external = isAbsolute(href);
      return `<a href="${href}"${external ? ' target="_blank" rel="noopener noreferrer"' : ""}>${escapeAttr(String(label ?? ""))}</a>`;
    });
    return out;
  };

  const closeList = () => {
    if (inList) {
      html.push("</ul>");
      inList = false;
    }
  };
  const flushParagraph = () => {
    if (paragraphLines.length) {
      html.push(`<p>${paragraphLines.map(renderInline).join(" ")}</p>`);
      paragraphLines = [];
    }
  };
  const flushBlockquote = () => {
    if (blockquoteLines.length) {
      html.push(`<blockquote>${renderInline(blockquoteLines.join(" "))}</blockquote>`);
      blockquoteLines = [];
    }
  };
  const startBlock = () => {
    flushParagraph();
    closeList();
  };

  for (const raw of lines) {
    const line = raw.trimEnd();
    const ltrim = line.replace(/^\s+/, "");

    const blockquote = line.match(/^>\s?(.*)$/);
    if (blockquote) {
      startBlock();
      blockquoteLines.push(blockquote[1]);
      continue;
    }
    flushBlockquote();

    const youtube = ltrim.match(/^@youtube\(([^)]+)\)\s*$/);
    if (youtube) {
      startBlock();
      const id = extractYouTubeId(youtube[1]);
      html.push(id ? renderYouTube(id) : `<p>Invalid YouTube video: ${youtube[1]}</p>`);
      continue;
    }

    const video = ltrim.match(/^@video\(([^)]+)\)\s*$/);
    if (video) {
      startBlock();
      html.push(renderVideo(video[1], false));
      continue;
    }

    const loop = ltrim.match(/^@loop_autoplay\(([^)]+)\)\s*$/);
    if (loop) {
      startBlock();
      html.push(renderVideo(loop[1], true));
      continue;
    }

    const image = ltrim.match(/^@(image|gif)\(([^,)]+)(?:,\s*([^)]+))?\)\s*$/);
    if (image) {
      startBlock();
      html.push(renderImage(image[2], "", image[3]?.trim() ?? ""));
      continue;
    }

    const heading = ltrim.match(/^(#{1,6})\s*(.*)$/);
    if (heading) {
      startBlock();
      const level = Math.min(heading[1].length, 6);
      html.push(`<h${level}>${renderInline(heading[2] ?? "")}</h${level}>`);
      continue;
    }

    const standaloneImage = line.match(/^!\[([^\]]*)\]\(([^)]+)\)\s*$/);
    if (standaloneImage) {
      startBlock();
      const [url, size] = standaloneImage[2].split("|");
      const safeUrl = String(url ?? "").trim();
      html.push(
        isVideoUrl(safeUrl)
          ? renderVideo(safeUrl, false)
          : renderImage(safeUrl, standaloneImage[1] ?? "", size ?? ""),
      );
      continue;
    }

    if (line.startsWith("- ")) {
      flushParagraph();
      if (!inList) {
        inList = true;
        html.push("<ul>");
      }
      html.push(`<li>${renderInline(line.slice(2))}</li>`);
      continue;
    }

    if (line.trim() === "") {
      startBlock();
      continue;
    }

    closeList();
    paragraphLines.push(line);
  }

  flushParagraph();
  flushBlockquote();
  closeList();
  return html.join("\n");
}
