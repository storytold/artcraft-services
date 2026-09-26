export type MediaKind = "image" | "video" | "audio" | "mesh" | "splat" | "unsupported";

// Fields from storyteller-web's GET /v1/media_files/file/{token} response.
export type SharedMedia = {
  token: string;
  media_class?: string | null;
  media_type?: string | null;
  maybe_engine_category?: string | null;
  maybe_title?: string | null;
  maybe_original_filename?: string | null;
  maybe_prompt_token?: string | null;
  maybe_text_transcript?: string | null;
  maybe_duration_millis?: number | null;
  maybe_creator_user?: {
    username: string;
    display_name?: string;
  } | null;
  maybe_model_weight_info?: { title: string } | null;
  created_at?: string;
  media_links: {
    cdn_url: string;
    maybe_thumbnail_template?: string | null;
    maybe_video_previews?: { still: string } | null;
  };
};

export type MediaPrompt = {
  maybe_positive_prompt?: string | null;
  maybe_negative_prompt?: string | null;
  maybe_model_type?: string | null;
  maybe_generation_provider?: string | null;
  maybe_aspect_ratio?: string | null;
  maybe_resolution?: string | null;
  maybe_duration_seconds?: number | null;
  maybe_generate_audio?: boolean | null;
  maybe_context_images?: {
    media_token: string;
    semantic?: string;
    media_links: { cdn_url: string; maybe_thumbnail_template?: string | null };
  }[] | null;
};

export const MEDIA_LABELS: Record<MediaKind, string> = {
  image: "Image", video: "Video", audio: "Audio", mesh: "3D mesh",
  splat: "3D splat", unsupported: "Media file",
};

export function mediaKind(media: SharedMedia): MediaKind {
  // Class is authoritative: PLY can be either a mesh or a Gaussian splat.
  switch (media.media_class) {
    case "image": case "video": case "audio": case "mesh": case "splat":
      return media.media_class;
    case "project": return "unsupported";
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

export function mediaFormat(media: SharedMedia): string {
  const type = media.media_type?.toLowerCase();
  if (type && !["unknown", "image", "video", "audio", "dimensional"].includes(type)) return type;
  try {
    return new URL(media.media_links.cdn_url).pathname.match(/\.([a-z0-9]+)$/i)?.[1].toLowerCase() ?? type ?? "";
  } catch {
    return type ?? "";
  }
}

export function safeMediaUrl(value?: string | null): string | undefined {
  if (!value) return undefined;
  try {
    const url = new URL(value);
    return ["https:", "http:"].includes(url.protocol) ? url.href : undefined;
  } catch {
    return undefined;
  }
}

export function corsMediaUrl(value: string): string {
  const url = new URL(value);
  // Match the old CDN CORS opt-in without corrupting existing query parameters.
  url.searchParams.set("cors", "1");
  return url.href;
}

export function mediaPoster(media: SharedMedia): string | undefined {
  return safeMediaUrl(media.media_links.maybe_video_previews?.still)
    ?? safeMediaUrl(media.media_links.maybe_thumbnail_template?.replace("{WIDTH}", "1280"));
}

export function mediaTitle(media: SharedMedia): string {
  const kind = mediaKind(media);
  const label = kind === "mesh" || kind === "splat" ? MEDIA_LABELS[kind] : MEDIA_LABELS[kind].toLowerCase();
  return media.maybe_title?.trim() || media.maybe_original_filename?.trim() || `Untitled ${label}`;
}

export function mediaDetailLabel(value?: string | null): string | undefined {
  if (!value) return undefined;
  const labels: Record<string, string> = {
    artcraft: "ArtCraft", square: "1:1", square_hd: "1:1 (HD)",
    wide_three_by_two: "3:2", wide_four_by_three: "4:3", wide_five_by_four: "5:4",
    wide_sixteen_by_nine: "16:9", wide_twenty_one_by_nine: "21:9",
    tall_two_by_three: "2:3", tall_three_by_four: "3:4", tall_four_by_five: "4:5",
    tall_nine_by_sixteen: "9:16", tall_nine_by_twenty_one: "9:21",
    half_k: "0.5K", one_k: "1K", two_k: "2K", three_k: "3K", four_k: "4K",
    four_eighty_p: "480p", seven_twenty_p: "720p", ten_eighty_p: "1080p",
  };
  return labels[value] ?? value.replace(/(\d)p(\d)/g, "$1.$2").replaceAll("_", " ").replace(/\b[a-z]/g, (letter) => letter.toUpperCase()).replace(/\b3d\b/gi, "3D");
}
