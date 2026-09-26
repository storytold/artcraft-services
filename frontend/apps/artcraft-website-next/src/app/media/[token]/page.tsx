import type { Metadata } from "next";
import MediaPage from "@/components/media/media-page";
import { API_HOST } from "@/lib/api";
import { mediaFilePath, mediaKind, mediaThumbnail, safeMediaUrl, type SharedMedia } from "@/lib/media";

const OG_IMAGE_WIDTH = 1200;

type Props = { params: Promise<{ token: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { token } = await params;
  const metadata: Metadata = {
    title: "Shared media",
    description: "View images, video, music, and interactive 3D creations shared with ArtCraft.",
    alternates: { canonical: `/media/${encodeURIComponent(token)}` },
    // A shared link is not a request to index a user's creation.
    robots: { index: false, follow: true },
  };
  const media = await fetchPublicMediaFile(token);
  if (!media) return metadata;
  const image = mediaKind(media) === "image"
    ? safeMediaUrl(media.media_links.cdn_url)
    : mediaThumbnail(media.media_links, OG_IMAGE_WIDTH);
  const creator = media.maybe_creator_user;
  const description = creator
    ? `Made with ArtCraft by ${creator.display_name || creator.username}.`
    : "Made with ArtCraft.";
  return {
    ...metadata,
    description,
    openGraph: {
      title: "Shared Media — ArtCraft",
      description,
      url: `/media/${encodeURIComponent(token)}`,
      ...(image && { images: [{ url: image }] }),
    },
    twitter: { card: image ? "summary_large_image" : "summary" },
  };
}

export default async function Media({ params }: Props) {
  const { token } = await params;
  // Resolve in the browser, using the same session credentials as the Vite
  // viewer. Never cache one visitor's private media response in server HTML.
  return <MediaPage key={token} token={token} />;
}

// Social cards use only the public API response. Never forward a visitor's
// cookies or signed session into server metadata; private media stays client-side.
async function fetchPublicMediaFile(token: string): Promise<SharedMedia | null> {
  try {
    const response = await fetch(`${API_HOST}${mediaFilePath(token)}`, {
      headers: { Accept: "application/json" },
      credentials: "omit",
      cache: "no-store",
      signal: AbortSignal.timeout(5000),
    });
    if (!response.ok) return null;
    const payload = await response.json() as { success?: boolean; media_file?: SharedMedia };
    const media = payload.media_file;
    return payload.success && media?.token && safeMediaUrl(media.media_links?.cdn_url) ? media : null;
  } catch {
    return null;
  }
}
