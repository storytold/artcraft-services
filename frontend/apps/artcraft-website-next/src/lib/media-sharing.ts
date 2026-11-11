import { mediaFormat, mediaKind, mediaThumbnail, safeMediaUrl, type SharedMedia } from "./media";

export const MEDIA_SHARE_ORIGIN = "https://getartcraft.com";
export const MEDIA_SHELL_TOKEN = "__MEDIA_TOKEN__";
const PREVIEW_WIDTH = 1200;
const VIDEO_TYPES: Record<string, string> = { mp4: "video/mp4", webm: "video/webm" };
const AUDIO_TYPES: Record<string, string> = {
  mp3: "audio/mpeg", wav: "audio/wav", m4a: "audio/mp4", aac: "audio/aac",
  ogg: "audio/ogg", opus: "audio/ogg", flac: "audio/flac",
};

type MediaEmbed = {
  kind: "video" | "audio";
  url: string;
  contentType: string;
  // These are iframe viewport dimensions, not invented source dimensions.
  width: number;
  height: number;
};

/**
 * Returns iframe player embed metadata for supported video and audio files, or null otherwise.
 */
export function mediaEmbed(media: SharedMedia): MediaEmbed | null {
  const kind = mediaKind(media);
  if (kind !== "video" && kind !== "audio") return null;
  const contentType = (kind === "video" ? VIDEO_TYPES : AUDIO_TYPES)[mediaFormat(media)];
  const url = safeMediaUrl(media.media_links.cdn_url);
  if (!contentType || !url?.startsWith("https://")) return null;
  return { kind, contentType, url, width: 640, height: kind === "video" ? 360 : 180 };
}

/**
 * Resolves the primary social preview image URL for a shared media item.
 */
export function mediaShareImage(media: SharedMedia): string | undefined {
  const cover = media.cover_image?.maybe_links;
  return mediaThumbnail(media.media_links, PREVIEW_WIDTH)
    ?? (mediaKind(media) === "image" ? safeMediaUrl(media.media_links.cdn_url) : undefined)
    ?? safeMediaUrl(cover?.thumbnail_template?.replace("{WIDTH}", String(PREVIEW_WIDTH)))
    ?? safeMediaUrl(cover?.cdn_url);
}
