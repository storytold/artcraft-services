// Helpers for the shared-media page, ported from the Vite site's
// components/lightbox/shared.ts, lib/download-media.ts, and the
// @storyteller/common media-format / thumbnail utilities.

import type { PromptContextImage } from "./api";

export const SHARE_URL_BASE = "https://getartcraft.com/media/";

const VIDEO_EXTENSIONS = [".mp4", ".webm", ".mov", ".avi", ".mkv", ".m4v"];
const MODEL_3D_EXTENSIONS = [".glb", ".gltf", ".fbx", ".spz"];

const ASPECT_RATIO_LABELS: Record<string, string> = {
  auto: "Auto",
  square: "Square",
  square_hd: "Square (HD)",
  wide: "Wide",
  tall: "Tall",
  wide_three_by_two: "3:2",
  wide_four_by_three: "4:3",
  wide_five_by_four: "5:4",
  wide_sixteen_by_nine: "16:9",
  wide_twenty_one_by_nine: "21:9",
  tall_two_by_three: "2:3",
  tall_three_by_four: "3:4",
  tall_four_by_five: "4:5",
  tall_nine_by_sixteen: "9:16",
  tall_nine_by_twenty_one: "9:21",
  auto_2k: "Auto (2K)",
  auto_3k: "Auto (3K)",
  auto_4k: "Auto (4K)",
};

const RESOLUTION_LABELS: Record<string, string> = {
  half_k: "0.5K",
  one_k: "1K",
  two_k: "2K",
  three_k: "3K",
  four_k: "4K",
  four_eighty_p: "480p",
  seven_twenty_p: "720p",
  ten_eighty_p: "1080p",
};

// Fallback extensions when the CDN URL carries none.
const EXT_BY_MEDIA_CLASS: Record<string, string> = {
  image: "png",
  video: "mp4",
  dimensional: "glb",
  mesh: "glb",
  splat: "spz",
};

export type MediaKind = "image" | "video" | "3d";

export function mediaKindForUrl(url: string): MediaKind {
  const lower = url.toLowerCase();
  if (VIDEO_EXTENSIONS.some((ext) => lower.includes(ext))) return "video";
  if (MODEL_3D_EXTENSIONS.some((ext) => lower.includes(ext))) return "3d";
  return "image";
}

// The media CDN only sends CORS headers when asked.
export function addCorsParam(url: string): string {
  return `${url}?cors=1`;
}

export function thumbnailUrl(
  template: string | null | undefined,
  width: number,
): string | null {
  return template ? template.replace("{WIDTH}", String(width)) : null;
}

// Thumbnail for a prompt reference. Video references carry no image
// thumbnail template, so their still-frame preview is used instead of the
// raw video URL (which an <img> cannot render).
export function contextImageThumbnail(
  contextImage: PromptContextImage,
  width: number,
): string {
  const links = contextImage.media_links;
  if (!links.maybe_thumbnail_template && links.maybe_video_previews) {
    const previews = links.maybe_video_previews;
    return (
      thumbnailUrl(previews.still_thumbnail_template, width) ?? previews.still
    );
  }
  return thumbnailUrl(links.maybe_thumbnail_template, width) ?? links.cdn_url;
}

export const formatAspectRatio = (value: string): string =>
  ASPECT_RATIO_LABELS[value] ?? value;

export const formatResolution = (value: string): string =>
  RESOLUTION_LABELS[value] ?? value;

export const formatDuration = (seconds: number): string => `${seconds}s`;

// The webapp lightbox shows the date and time on separate lines
// ("Sep 17, 2026" / "9:12:05 PM").
const CREATED_DATE_FORMAT = new Intl.DateTimeFormat("en-US", {
  dateStyle: "medium",
});
const CREATED_TIME_FORMAT = new Intl.DateTimeFormat("en-US", {
  timeStyle: "medium",
});

export function formatCreatedAt(
  iso: string,
): { date: string; time: string } | null {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return null;
  return {
    date: CREATED_DATE_FORMAT.format(parsed),
    time: CREATED_TIME_FORMAT.format(parsed),
  };
}

// Fetches the file as a blob so the browser saves it instead of navigating
// to the CDN. Throws on failure so the caller can surface an error.
export async function downloadMediaFile({
  url,
  filename,
  mediaClass,
}: {
  url: string;
  filename: string;
  mediaClass?: string | null;
}): Promise<void> {
  const response = await fetch(`${addCorsParam(url)}&dl=1`, {
    credentials: "omit",
  });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const blobUrl = URL.createObjectURL(await response.blob());
  try {
    const anchor = document.createElement("a");
    anchor.style.display = "none";
    anchor.href = blobUrl;
    anchor.download = `${filename}.${extensionForUrl(url, mediaClass)}`;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
  } finally {
    URL.revokeObjectURL(blobUrl);
  }
}

function extensionForUrl(url: string, mediaClass?: string | null): string {
  try {
    const match = new URL(url, window.location.href).pathname.match(
      /\.([a-z0-9]{2,5})$/i,
    );
    if (match) return match[1].toLowerCase();
  } catch {
    // Unparseable URL; fall through to the class default.
  }
  return (mediaClass && EXT_BY_MEDIA_CLASS[mediaClass]) || "bin";
}
