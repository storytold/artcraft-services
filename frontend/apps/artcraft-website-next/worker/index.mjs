import {
  FLATPAK_APPS,
  FLATPAK_INDEX,
  PACKAGE_REDIRECTS,
  PUBLIC_KEY_BASE64,
  SIGNING_CONFIGURED,
  TUTORIAL_REDIRECTS,
} from "./generated-catalog.mjs";

const MEDIA_SHARE_ORIGIN = "https://getartcraft.com";
const MEDIA_SHELL_TOKEN = "__MEDIA_TOKEN__";
const DEFAULT_API_HOST = "https://api.storyteller.ai";
const PREVIEW_WIDTH = 1200;
const APT_KEYRING_PATH = "/etc/apt/keyrings/artcraft.gpg";
const OCI_SIGNATURE_LOOKASIDE_PATH = "/oci-signatures";

const VIDEO_TYPES = { mp4: "video/mp4", webm: "video/webm" };
const AUDIO_TYPES = {
  mp3: "audio/mpeg",
  wav: "audio/wav",
  m4a: "audio/mp4",
  aac: "audio/aac",
  ogg: "audio/ogg",
  opus: "audio/ogg",
  flac: "audio/flac",
};

const MEDIA_LABELS = {
  image: "Image",
  video: "Video",
  audio: "Audio",
  mesh: "3D mesh",
  splat: "3D splat",
  unsupported: "Media file",
};

const VALID_RPM_ARCHES = new Set(["x86_64", "aarch64", "riscv64", "noarch"]);
const VALID_PACMAN_ARCHES = new Set(["x86_64", "aarch64"]);

const REPO_CACHE_CONTROL = "public, max-age=300, s-maxage=3600, stale-while-revalidate=86400";

/**
 * Escapes HTML special characters for safe interpolation into attributes and text nodes.
 */
function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

/**
 * Decodes a URL path segment, returning null when percent-encoding is malformed.
 */
function safeDecode(segment) {
  try {
    return decodeURIComponent(segment);
  } catch {
    return null;
  }
}

/**
 * Resolves the trusted base origin for generated Linux repository and Flatpak descriptors.
 */
function getRequestBaseUrl(request) {
  const url = new URL(request.url);
  const fwdHost = request.headers.get("x-forwarded-host");
  if (fwdHost && /^deploy-preview-\d+--artcraft-website-next\.netlify\.app(?::\d+)?$/.test(fwdHost)) {
    return `https://${fwdHost}`;
  }
  if (fwdHost && /^(?:localhost|127\.0\.0\.1)(?::\d+)?$/.test(fwdHost)) {
    const proto =
      request.headers.get("x-forwarded-proto") || (url.protocol ? url.protocol.replace(":", "") : "http");
    return `${proto}://${fwdHost}`;
  }
  if (
    url.protocol === "https:" &&
    (url.hostname === "getartcraft.com" ||
      url.hostname === "getartcraft.dev" ||
      url.hostname.endsWith(".getartcraft.dev") ||
      url.hostname.endsWith(".workers.dev") ||
      /^deploy-preview-\d+--artcraft-website-next\.netlify\.app$/.test(url.hostname))
  ) {
    return url.origin;
  }
  if ((url.hostname === "localhost" || url.hostname === "127.0.0.1") && url.protocol === "http:") {
    return url.origin;
  }
  return "https://getartcraft.com";
}

/**
 * Returns Flatpak INI `GPGKey` and `SignatureLookaside` lines when repository signing is active.
 */
function flatpakSignatureLines(baseUrl) {
  if (!SIGNING_CONFIGURED || !PUBLIC_KEY_BASE64) return [];
  return [
    `GPGKey=${PUBLIC_KEY_BASE64}`,
    `SignatureLookaside=${baseUrl}${OCI_SIGNATURE_LOOKASIDE_PATH}`,
  ];
}

/**
 * Normalizes Flatpak/OCI architecture query values to `amd64` or `arm64`.
 */
function normalizeArch(arch) {
  if (!arch) return null;
  const lower = arch.toLowerCase();
  if (lower === "amd64" || lower === "x86_64" || lower === "x64") return "amd64";
  if (lower === "arm64" || lower === "aarch64") return "arm64";
  return lower;
}

/**
 * Filters the precomputed Flatpak OCI index according to query parameters.
 */
function filterFlatpakIndex(searchParams) {
  const arch = normalizeArch(searchParams.get("architecture"));
  const ref = searchParams.get("label:org.flatpak.ref");
  const refMustExist = searchParams.get("label:org.flatpak.ref:exists") === "1";
  const tag = searchParams.get("tag");
  const reqRepo = searchParams.get("repository") ?? searchParams.get("repo");

  const results = [];
  for (const repo of FLATPAK_INDEX.Results) {
    if (reqRepo && repo.Repository !== reqRepo) continue;
    const images = repo.Images.filter((img) => {
      if (arch && img.Architecture !== arch) return false;
      if (ref && img.Labels["org.flatpak.ref"] !== ref) return false;
      if (refMustExist && !img.Labels["org.flatpak.ref"]) return false;
      const refTag = img.Labels["org.flatpak.ref"]?.split("/").at(-1);
      if (tag && tag !== "latest" && !img.Tags.includes(tag) && refTag !== tag) {
        return false;
      }
      return true;
    });
    if (images.length > 0) {
      results.push({ ...repo, Images: images });
    }
  }
  return {
    Registry: FLATPAK_INDEX.Registry,
    Results: results,
  };
}

/**
 * Redirects a catalogued Linux package or signature filename to its GitHub Release asset URL.
 */
function handlePackageRedirect(filename) {
  if (!filename) return new Response("Not found", { status: 404 });
  const targetUrl = PACKAGE_REDIRECTS[filename];
  if (!targetUrl) return new Response("Package not found", { status: 404 });
  return new Response(null, {
    status: 302,
    headers: {
      Location: targetUrl,
      "Cache-Control": "public, max-age=300, s-maxage=3600",
    },
  });
}

/**
 * Serves a pre-generated static repository asset from the Worker `ASSETS` binding with explicit headers.
 */
async function serveStaticAssetWithHeaders(request, env, pathname, contentType, extraHeaders = {}) {
  const assetUrl = new URL(pathname, request.url);
  const response = await env.ASSETS.fetch(new Request(assetUrl, { method: request.method }));
  if (!response.ok) {
    return new Response("Not found", { status: 404 });
  }
  return new Response(response.body, {
    status: 200,
    headers: {
      "Content-Type": contentType,
      "Cache-Control": REPO_CACHE_CONTROL,
      ...extraHeaders,
    },
  });
}

/**
 * Validates that a candidate URL uses HTTP(S) and returns its normalized `href`.
 */
function safeMediaUrl(value) {
  if (!value) return undefined;
  try {
    const url = new URL(value);
    return ["https:", "http:"].includes(url.protocol) ? url.href : undefined;
  } catch {
    return undefined;
  }
}

/**
 * Extracts the normalized lowercase media file extension or format identifier.
 */
function mediaFormat(media) {
  const type = media.media_type?.toLowerCase();
  if (type && !["unknown", "image", "video", "audio", "dimensional"].includes(type)) return type;
  try {
    return new URL(media.media_links.cdn_url).pathname.match(/\.([a-z0-9]+)$/i)?.[1].toLowerCase() ?? type ?? "";
  } catch {
    return type ?? "";
  }
}

/**
 * Determines the high-level media kind (`image`, `video`, `audio`, `mesh`, `splat`, or `unsupported`).
 */
function mediaKind(media) {
  switch (media.media_class) {
    case "image":
    case "video":
    case "audio":
    case "mesh":
    case "splat":
      return media.media_class;
    case "project":
      return "unsupported";
  }
  if (media.maybe_engine_category === "splat") return "splat";
  const format = mediaFormat(media);
  if (["spz", "splat", "ksplat", "ply"].includes(format)) return "splat";
  if (["glb", "gltf", "fbx", "obj", "stl", "pmx", "pmd"].includes(format)) return "mesh";
  if (["image", "png", "jpg", "jpeg", "webp", "gif", "avif"].includes(format)) return "image";
  if (["video", "mp4", "webm", "mov", "m4v"].includes(format)) return "video";
  if (["audio", "wav", "mp3", "m4a", "aac", "ogg", "opus", "flac"].includes(format)) return "audio";
  return "unsupported";
}

/**
 * Computes a human-readable display title for a shared media item.
 */
function mediaTitle(media) {
  const kind = mediaKind(media);
  const label = kind === "mesh" || kind === "splat" ? MEDIA_LABELS[kind] : MEDIA_LABELS[kind].toLowerCase();
  return media.maybe_title?.trim() || media.maybe_original_filename?.trim() || `Untitled ${label}`;
}

/**
 * Resolves a thumbnail URL at the requested width from media link templates.
 */
function mediaThumbnail(links, width) {
  const previews = links.maybe_video_previews;
  const template = links.maybe_thumbnail_template ?? previews?.still_thumbnail_template;
  return safeMediaUrl(template?.replace("{WIDTH}", String(width))) ?? safeMediaUrl(previews?.still);
}

/**
 * Resolves the best social preview image URL for a shared media item.
 */
function mediaShareImage(media) {
  const cover = media.cover_image?.maybe_links;
  return (
    mediaThumbnail(media.media_links, PREVIEW_WIDTH) ??
    (mediaKind(media) === "image" ? safeMediaUrl(media.media_links.cdn_url) : undefined) ??
    safeMediaUrl(cover?.thumbnail_template?.replace("{WIDTH}", String(PREVIEW_WIDTH))) ??
    safeMediaUrl(cover?.cdn_url)
  );
}

/**
 * Returns embed player metadata for supported video and audio files, or null otherwise.
 */
function mediaEmbed(media) {
  const kind = mediaKind(media);
  if (kind !== "video" && kind !== "audio") return null;
  const contentType = (kind === "video" ? VIDEO_TYPES : AUDIO_TYPES)[mediaFormat(media)];
  const url = safeMediaUrl(media.media_links.cdn_url);
  if (!contentType || !url?.startsWith("https://")) return null;
  return { kind, contentType, url, width: 640, height: kind === "video" ? 360 : 180 };
}

/**
 * Fetches public or unlisted shared media metadata from the upstream API by token.
 */
async function fetchPublicMediaFile(token, env) {
  const apiHost = env?.NEXT_PUBLIC_API_HOST || DEFAULT_API_HOST;
  try {
    const response = await fetch(`${apiHost}/v1/media_files/file/${encodeURIComponent(token)}`, {
      headers: { Accept: "application/json" },
      signal: AbortSignal.timeout(5000),
    });
    if (!response.ok) return null;
    const payload = await response.json();
    const media = payload?.media_file;
    if (!payload?.success || media?.token !== token || !safeMediaUrl(media.media_links?.cdn_url)) {
      return null;
    }
    return media.creator_set_visibility === "public" || media.creator_set_visibility === "hidden"
      ? media
      : null;
  } catch {
    return null;
  }
}

/**
 * Parses a positive safe integer query bound or returns `Infinity`.
 */
function maximum(value) {
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : Infinity;
}

/**
 * Handles `/media/:token/oembed` requests and returns an oEmbed 1.0 JSON response.
 */
async function handleMediaOembed(request, token, env) {
  const headers = { "Cache-Control": "private, no-store", "X-Robots-Tag": "noindex, nofollow" };
  const query = new URL(request.url).searchParams;
  if (query.has("format") && query.get("format") !== "json") {
    return Response.json({ error: "Only JSON is supported" }, { status: 501, headers });
  }
  if (query.get("url") !== `${MEDIA_SHARE_ORIGIN}/media/${encodeURIComponent(token)}`) {
    return Response.json({ error: "Media unavailable" }, { status: 404, headers });
  }
  const media = await fetchPublicMediaFile(token, env);
  const embed = media && mediaEmbed(media);
  if (!media || !embed) {
    return Response.json({ error: "Media unavailable" }, { status: 404, headers });
  }
  const playerUrl = `${MEDIA_SHARE_ORIGIN}/media/${encodeURIComponent(token)}/player`;
  const scale = Math.min(
    1,
    maximum(query.get("maxwidth")) / embed.width,
    maximum(query.get("maxheight")) / embed.height,
  );
  const width = Math.max(1, Math.floor(embed.width * scale));
  const height = Math.max(1, Math.floor(embed.height * scale));
  return Response.json(
    {
      version: "1.0",
      type: embed.kind === "video" ? "video" : "rich",
      provider_name: "ArtCraft",
      provider_url: MEDIA_SHARE_ORIGIN,
      title: mediaTitle(media),
      author_name: media.maybe_creator_user?.display_name || media.maybe_creator_user?.username,
      width,
      height,
      html: `<iframe src="${playerUrl}" width="${width}" height="${height}" title="ArtCraft media player" frameborder="0" allow="fullscreen" allowfullscreen></iframe>`,
    },
    { headers },
  );
}

/**
 * Handles `/media/:token/player` requests and returns a standalone HTML5 player document.
 */
async function handleMediaPlayer(token, env) {
  const headers = {
    "Content-Type": "text/html; charset=utf-8",
    "Cache-Control": "private, no-store",
    "X-Content-Type-Options": "nosniff",
    "X-Robots-Tag": "noindex, nofollow",
    "Referrer-Policy": "no-referrer",
    "Content-Security-Policy":
      "default-src 'none'; style-src 'unsafe-inline'; img-src https:; media-src https:; base-uri 'none'; form-action 'none'",
  };
  const media = await fetchPublicMediaFile(token, env);
  const embed = media && mediaEmbed(media);
  if (!media || !embed) {
    return new Response("Media unavailable", { status: 404, headers });
  }
  const title = escapeHtml(mediaTitle(media));
  const poster = escapeHtml(mediaShareImage(media) ?? `${MEDIA_SHARE_ORIGIN}/images/og-image.png`);
  const source = `<source src="${escapeHtml(embed.url)}" type="${embed.contentType}">`;
  const player =
    embed.kind === "video"
      ? `<video controls playsinline preload="metadata" poster="${poster}" aria-label="${title}">${source}</video>`
      : `<div class="audio"><img src="${poster}" alt=""><div><p>${title}</p><audio controls preload="metadata" aria-label="${title}">${source}</audio></div></div>`;
  return new Response(
    `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${title} — ArtCraft</title><style>
    :root{color-scheme:light dark}*{box-sizing:border-box}html,body{margin:0;width:100%;height:100%;font:14px system-ui;background:light-dark(#f2f1ee,#121316);color:light-dark(#222,#eee)}
    video{display:block;width:100%;height:100%;object-fit:contain;background:#000}.audio{display:flex;align-items:center;gap:16px;padding:16px;height:100%}.audio img{width:80px;height:80px;object-fit:cover}.audio div{min-width:0;flex:1}p{margin:0 0 12px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}audio{width:100%}@media(max-width:360px){.audio img{display:none}}
    </style></head><body>${player}</body></html>`,
    { headers },
  );
}

/**
 * Builds the `<head>` social metadata tags (Open Graph, Twitter Card, canonical, and oEmbed) for `/media/:token`.
 */
function buildMediaHeadTags(token, media) {
  const encodedToken = encodeURIComponent(token);
  const canonicalUrl = `${MEDIA_SHARE_ORIGIN}/media/${encodedToken}`;
  const fallbackImage = `${MEDIA_SHARE_ORIGIN}/images/og-image.png`;
  const title = media ? `${mediaTitle(media)} — ArtCraft` : "Shared media — ArtCraft";
  const description = media
    ? media.maybe_creator_user
      ? `Made with ArtCraft by ${media.maybe_creator_user.display_name || media.maybe_creator_user.username}.`
      : "Made with ArtCraft."
    : "View images, video, music, and interactive 3D creations shared with ArtCraft.";
  const shareImage = (media && mediaShareImage(media)) || fallbackImage;
  const imageAlt = media ? mediaTitle(media) : "ArtCraft";
  const embed = media ? mediaEmbed(media) : null;
  const ogType = embed?.kind === "video" ? "video.other" : "website";
  const twitterCard = embed ? "player" : "summary_large_image";

  const tags = [
    `<title>${escapeHtml(title)}</title>`,
    `<meta name="description" content="${escapeHtml(description)}"/>`,
    `<link rel="canonical" href="${escapeHtml(canonicalUrl)}"/>`,
    embed
      ? `<link rel="alternate" type="application/json+oembed" href="${escapeHtml(`${canonicalUrl}/oembed?url=${encodeURIComponent(canonicalUrl)}&format=json`)}"/>`
      : "",
    `<meta name="robots" content="noindex, follow"/>`,
    `<meta property="og:title" content="${escapeHtml(title)}"/>`,
    `<meta property="og:description" content="${escapeHtml(description)}"/>`,
    `<meta property="og:url" content="${escapeHtml(canonicalUrl)}"/>`,
    `<meta property="og:site_name" content="ArtCraft"/>`,
    `<meta property="og:image" content="${escapeHtml(shareImage)}"/>`,
    !media || shareImage === fallbackImage
      ? `<meta property="og:image:width" content="1200"/><meta property="og:image:height" content="630"/>`
      : "",
    `<meta property="og:image:alt" content="${escapeHtml(imageAlt)}"/>`,
    `<meta property="og:type" content="${ogType}"/>`,
    embed?.kind === "video"
      ? `<meta property="og:video" content="${escapeHtml(embed.url)}"/><meta property="og:video:secure_url" content="${escapeHtml(embed.url)}"/><meta property="og:video:type" content="${escapeHtml(embed.contentType)}"/>`
      : "",
    embed?.kind === "audio"
      ? `<meta property="og:audio" content="${escapeHtml(embed.url)}"/><meta property="og:audio:secure_url" content="${escapeHtml(embed.url)}"/><meta property="og:audio:type" content="${escapeHtml(embed.contentType)}"/>`
      : "",
    `<meta name="twitter:card" content="${twitterCard}"/>`,
    `<meta name="twitter:title" content="${escapeHtml(title)}"/>`,
    `<meta name="twitter:description" content="${escapeHtml(description)}"/>`,
    `<meta name="twitter:image" content="${escapeHtml(shareImage)}"/>`,
    !media || shareImage === fallbackImage
      ? `<meta name="twitter:image:width" content="1200"/><meta name="twitter:image:height" content="630"/>`
      : "",
    `<meta name="twitter:image:alt" content="${escapeHtml(imageAlt)}"/>`,
    embed
      ? [
          `<meta name="twitter:player" content="${escapeHtml(`${canonicalUrl}/player`)}"/>`,
          `<meta name="twitter:player:width" content="${embed.width}"/>`,
          `<meta name="twitter:player:height" content="${embed.height}"/>`,
          embed.contentType === "video/mp4"
            ? `<meta name="twitter:player:stream" content="${escapeHtml(embed.url)}"/><meta name="twitter:player:stream:content_type" content="${escapeHtml(embed.contentType)}"/>`
            : "",
        ].join("")
      : "",
  ];

  return tags.filter(Boolean).join("");
}

/**
 * Serves the static `/media/__MEDIA_TOKEN__` shell with resolved `<head>` social metadata for `token`.
 */
async function handleMediaPage(request, token, env) {
  const shellUrl = new URL(`/media/${MEDIA_SHELL_TOKEN}`, request.url);
  const [shellResponse, media] = await Promise.all([
    env.ASSETS.fetch(new Request(shellUrl)),
    fetchPublicMediaFile(token, env),
  ]);
  if (!shellResponse.ok) {
    return new Response("Not found", { status: 404 });
  }
  let html = await shellResponse.text();
  // Strip placeholder page metadata inside <head> and inject the resolved social tags.
  html = html.replace(/<head([^>]*)>([\s\S]*?)<\/head>/i, (_match, attrs, headContent) => {
    const cleaned = headContent
      .replace(/<title>[\s\S]*?<\/title>/gi, "")
      .replace(/<meta\s+(?:name|property)="(?:description|robots|og:[^"]+|twitter:[^"]+)"[^>]*>/gi, "")
      .replace(/<link\s+rel="(?:canonical|alternate)"[^>]*>/gi, "");
    return `<head${attrs}>${buildMediaHeadTags(token, media)}${cleaned}</head>`;
  });
  html = html.replaceAll(MEDIA_SHELL_TOKEN, encodeURIComponent(token));

  return new Response(html, {
    status: 200,
    headers: {
      "Content-Type": "text/html; charset=utf-8",
      "Cache-Control": "private, no-store",
    },
  });
}

export default {
  /**
   * Main Cloudflare Worker fetch entrypoint handling redirects, Linux repos, Flatpak OCI, and shared media.
   */
  async fetch(request, env) {
    const url = new URL(request.url);
    const { pathname } = url;

    // 1. Legacy & route redirects
    if (pathname === "/checkout_success") {
      return Response.redirect("https://app.getartcraft.com/checkout/success", 307);
    }
    if (pathname === "/checkout_cancel" || pathname === "/portal_closed") {
      return Response.redirect("https://app.getartcraft.com/checkout/cancel", 307);
    }
    if (pathname === "/apps/loopcraft" || pathname.startsWith("/images/apps/loopcraft/")) {
      return Response.redirect(new URL("/apps", url.origin).href, 308);
    }
    if (pathname === "/apps/drawcraft") {
      return Response.redirect(new URL("/apps/vectorcraft", url.origin).href, 308);
    }
    if (pathname === "/apps/printcraft") {
      return Response.redirect(new URL("/apps/pdfcraft", url.origin).href, 308);
    }
    if (pathname.startsWith("/images/apps/printcraft/")) {
      const file = pathname.slice("/images/apps/printcraft/".length);
      return Response.redirect(new URL(`/images/apps/pdfcraft/${file}`, url.origin).href, 308);
    }
    if (pathname === "/media" && url.searchParams.has("media")) {
      const token = url.searchParams.get("media");
      if (token) {
        return Response.redirect(new URL(`/media/${encodeURIComponent(token)}`, url.origin).href, 308);
      }
    }
    if (pathname.startsWith("/tutorials/")) {
      const slug = pathname.slice("/tutorials/".length).replace(/\/$/, "").toLowerCase();
      const canonical = TUTORIAL_REDIRECTS[slug];
      if (canonical) {
        return Response.redirect(new URL(`/tutorials/${canonical}`, url.origin).href, 308);
      }
    }

    // 2. Host-aware Linux repository & Flatpak descriptors
    if (pathname === "/apt/artcraft.list") {
      const baseUrl = getRequestBaseUrl(request);
      const body = SIGNING_CONFIGURED
        ? `deb [signed-by=${APT_KEYRING_PATH}] ${baseUrl}/apt stable main\n`
        : `deb [trusted=yes] ${baseUrl}/apt stable main\n`;
      return new Response(body, {
        headers: { "Content-Type": "text/plain; charset=utf-8", "Cache-Control": "public, max-age=3600" },
      });
    }
    if (pathname === "/apt/artcraft.sources") {
      const baseUrl = getRequestBaseUrl(request);
      const body = [
        "Types: deb",
        `URIs: ${baseUrl}/apt`,
        "Suites: stable",
        "Components: main",
        "Architectures: amd64 arm64 riscv64",
        SIGNING_CONFIGURED ? `Signed-By: ${APT_KEYRING_PATH}` : "Trusted: yes",
        "",
      ].join("\n");
      return new Response(body, {
        headers: { "Content-Type": "text/plain; charset=utf-8", "Cache-Control": "public, max-age=3600" },
      });
    }
    if (pathname === "/rpm/artcraft.repo") {
      const baseUrl = getRequestBaseUrl(request);
      const body = SIGNING_CONFIGURED
        ? `[artcraft]\n# Signed metadata and signed packages: dnf verifies repomd.xml against its signature, imports the\n# key below, and verifies the OpenPGP header signature inside every package it downloads.\nname=ArtCraft Linux Packages\nbaseurl=${baseUrl}/rpm\nenabled=1\ngpgcheck=1\nrepo_gpgcheck=1\ngpgkey=${baseUrl}/keys/artcraft.asc\n`
        : `[artcraft]\nname=ArtCraft Linux Packages\nbaseurl=${baseUrl}/rpm\nenabled=1\ngpgcheck=0\nrepo_gpgcheck=0\n`;
      return new Response(body, {
        headers: { "Content-Type": "text/plain; charset=utf-8", "Cache-Control": "public, max-age=3600" },
      });
    }
    if (pathname === "/arch/artcraft.conf") {
      const baseUrl = getRequestBaseUrl(request);
      const body = SIGNING_CONFIGURED
        ? `[artcraft]\n# Import the key before syncing, otherwise pacman rejects the repository:\n#   sudo pacman-key --add /tmp/artcraft.asc && sudo pacman-key --lsign-key <fingerprint>\nSigLevel = Required DatabaseRequired\nServer = ${baseUrl}/arch/$arch\n`
        : `[artcraft]\nSigLevel = Optional TrustAll\nServer = ${baseUrl}/arch/$arch\n`;
      return new Response(body, {
        headers: { "Content-Type": "text/plain; charset=utf-8", "Cache-Control": "public, max-age=3600" },
      });
    }
    if (pathname === "/artcraft-oci.flatpakrepo") {
      const baseUrl = getRequestBaseUrl(request);
      const sigLines = flatpakSignatureLines(baseUrl);
      const ini = `[Flatpak Repo]\nTitle=ArtCraft OCI\nUrl=oci+${baseUrl}\nHomepage=https://getartcraft.com/\nComment=Official ArtCraft OCI Flatpak repository\nDescription=Open-source creative apps for Linux distributed via GitHub Container Registry\n${
        sigLines.length > 0 ? `${sigLines.join("\n")}\n` : "GPGVerify=false\n"
      }`;
      return new Response(ini, {
        headers: {
          "Content-Type": "application/x-flatpak-repo; charset=utf-8",
          "Cache-Control": "public, max-age=86400",
          "Content-Disposition": 'attachment; filename="artcraft-oci.flatpakrepo"',
        },
      });
    }

    const flatpakRefMatch = pathname.match(/^\/apps\/([^/]+)\/([^/]+)$/);
    if (flatpakRefMatch) {
      const [, slug, filename] = flatpakRefMatch;
      const app = FLATPAK_APPS[slug];
      if (!app || filename !== `${slug}-oci.flatpakref`) {
        return new Response("Not found", { status: 404 });
      }
      const baseUrl = `${url.protocol}//${url.host}`;
      const sigLines = flatpakSignatureLines(baseUrl);
      const ini = `[Flatpak Ref]\nName=${app.appId}\nBranch=stable\nTitle=${app.title}\nUrl=oci+${baseUrl}\nRuntimeRepo=https://dl.flathub.org/repo/flathub.flatpakrepo\nIsRuntime=false\n${sigLines
        .map((line) => `${line}\n`)
        .join("")}`;
      return new Response(ini, {
        headers: {
          "Content-Type": "application/x-flatpak-ref; charset=utf-8",
          "Cache-Control": "public, max-age=86400",
          "Content-Disposition": `attachment; filename="${slug}-oci.flatpakref"`,
        },
      });
    }

    // 3. Flatpak OCI Index endpoints
    if (pathname === "/index/static" || pathname === "/index/dynamic") {
      const cacheControl =
        pathname === "/index/static" ? "public, max-age=300, s-maxage=300" : "no-cache";
      return Response.json(filterFlatpakIndex(url.searchParams), {
        headers: {
          "Cache-Control": cacheControl,
          "Netlify-Vary": "query",
        },
      });
    }

    // 4. Pre-generated keys, OCI signatures & Linux repository files + binary redirects
    if (pathname === "/keys/artcraft.asc") {
      if (!SIGNING_CONFIGURED) {
        return new Response("Not found", { status: 404 });
      }
      return serveStaticAssetWithHeaders(request, env, pathname, "application/pgp-keys; charset=utf-8", {
        "Content-Disposition": 'inline; filename="artcraft.asc"',
      });
    }
    if (pathname === "/keys/artcraft.gpg") {
      if (!SIGNING_CONFIGURED) {
        return new Response("Not found", { status: 404 });
      }
      return serveStaticAssetWithHeaders(request, env, pathname, "application/pgp-keys");
    }
    if (pathname.startsWith("/oci-signatures/")) {
      return serveStaticAssetWithHeaders(request, env, pathname, "application/pgp-signature");
    }
    if (pathname.startsWith("/apt/pool/")) {
      const filename = pathname.split("/").at(-1);
      if (!filename?.endsWith(".deb")) return new Response("Not found", { status: 404 });
      return handlePackageRedirect(filename);
    }
    if (pathname.startsWith("/apt/dists/")) {
      if (pathname.endsWith("/Packages.gz")) {
        return serveStaticAssetWithHeaders(request, env, pathname, "application/gzip");
      }
      if (pathname.endsWith("/Release.gpg")) {
        return serveStaticAssetWithHeaders(
          request,
          env,
          pathname,
          "application/pgp-signature; charset=utf-8",
        );
      }
      if (pathname.endsWith("/Packages") || pathname.endsWith("/Release") || pathname.endsWith("/InRelease")) {
        return serveStaticAssetWithHeaders(request, env, pathname, "text/plain; charset=utf-8");
      }
      return new Response("Not found", { status: 404 });
    }
    if (pathname.startsWith("/rpm/")) {
      const rpmPkgMatch = pathname.match(/^\/rpm\/(?:([^/]+)\/)?packages\/(.+)$/);
      if (rpmPkgMatch) {
        const [, arch, rest] = rpmPkgMatch;
        if (arch && !VALID_RPM_ARCHES.has(arch)) return new Response("Not found", { status: 404 });
        const filename = rest.split("/").at(-1);
        if (!filename || (!filename.endsWith(".rpm") && !filename.endsWith(".rpm.asc"))) {
          return new Response("Not found", { status: 404 });
        }
        return handlePackageRedirect(filename);
      }
      if (pathname.endsWith("/primary.xml.gz")) {
        return serveStaticAssetWithHeaders(request, env, pathname, "application/gzip");
      }
      if (pathname.endsWith("/repomd.xml")) {
        return serveStaticAssetWithHeaders(request, env, pathname, "application/xml; charset=utf-8");
      }
      if (pathname.endsWith("/repomd.xml.asc")) {
        return serveStaticAssetWithHeaders(
          request,
          env,
          pathname,
          "application/pgp-signature; charset=utf-8",
        );
      }
      if (pathname === "/rpm/repodata/repomd.xml.key") {
        return serveStaticAssetWithHeaders(request, env, pathname, "application/pgp-keys; charset=utf-8");
      }
      return new Response("Not found", { status: 404 });
    }
    if (pathname.startsWith("/arch/")) {
      const archMatch = pathname.match(/^\/arch\/([^/]+)\/([^/]+)$/);
      if (!archMatch) return new Response("Not found", { status: 404 });
      const [, arch, filename] = archMatch;
      if (!VALID_PACMAN_ARCHES.has(arch)) return new Response("Not found", { status: 404 });
      if (filename === "artcraft.db" || filename === "artcraft.db.tar.gz") {
        return serveStaticAssetWithHeaders(request, env, pathname, "application/gzip");
      }
      if (filename === "artcraft.db.sig") {
        return serveStaticAssetWithHeaders(request, env, pathname, "application/pgp-signature");
      }
      if (filename.endsWith(".sig") && !SIGNING_CONFIGURED) {
        return new Response("Not found", { status: 404 });
      }
      if (filename.endsWith(".pkg.tar.zst") || filename.endsWith(".pkg.tar.zst.sig")) {
        return handlePackageRedirect(filename);
      }
      return new Response("Not found", { status: 404 });
    }

    // 5. Shared media routes (/media/:token, /media/:token/oembed, /media/:token/player)
    const mediaSubMatch = pathname.match(/^\/media\/([^/]+)\/(oembed|player)$/);
    if (mediaSubMatch) {
      const token = safeDecode(mediaSubMatch[1]);
      if (token === null) return new Response("Not found", { status: 404 });
      const subroute = mediaSubMatch[2];
      return subroute === "oembed"
        ? handleMediaOembed(request, token, env)
        : handleMediaPlayer(token, env);
    }
    const mediaPageMatch = pathname.match(/^\/media\/([^/]+)\/?$/);
    if (mediaPageMatch) {
      const rawSegment = mediaPageMatch[1];
      if (rawSegment.endsWith(".txt")) {
        const token = safeDecode(rawSegment.slice(0, -4));
        if (token === null) return new Response("Not found", { status: 404 });
        const rscUrl = new URL(`/media/${MEDIA_SHELL_TOKEN}.txt`, request.url);
        const rscResp = await env.ASSETS.fetch(new Request(rscUrl));
        if (!rscResp.ok) return new Response("Not found", { status: 404 });
        const body = (await rscResp.text()).replaceAll(MEDIA_SHELL_TOKEN, encodeURIComponent(token));
        return new Response(body, {
          headers: { "Content-Type": "text/x-component; charset=utf-8", "Cache-Control": "private, no-store" },
        });
      }
      const token = safeDecode(rawSegment);
      if (token === null) return new Response("Not found", { status: 404 });
      return handleMediaPage(request, token, env);
    }

    return env.ASSETS.fetch(request);
  },
};
