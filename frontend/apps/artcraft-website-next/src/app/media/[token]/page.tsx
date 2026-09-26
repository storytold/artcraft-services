import type { Metadata } from "next";
import MediaViewer from "@/components/media/media-viewer";
import { API_HOST, mediaFilePath, type MediaFile } from "@/lib/api";
import { mediaKindForUrl, thumbnailUrl } from "@/lib/media";

type Params = Promise<{ token: string }>;

const OG_IMAGE_WIDTH = 1200;

// Share links get unfurled by chat apps and social cards, so the preview
// image is resolved on the server (public media only: this request carries
// no session). Private or missing media falls back to the site defaults.
export async function generateMetadata({
  params,
}: {
  params: Params;
}): Promise<Metadata> {
  const { token } = await params;
  const metadata: Metadata = {
    title: "Shared Media",
    description: "View shared media from ArtCraft.",
    alternates: { canonical: `/media/${token}` },
    robots: { index: false },
  };

  const media = await fetchPublicMediaFile(token);
  if (!media) return metadata;

  const url = media.media_links.cdn_url;
  const image =
    mediaKindForUrl(url) === "image"
      ? url
      : thumbnailUrl(media.media_links.thumbnail_template, OG_IMAGE_WIDTH);
  const description = media.maybe_creator_user
    ? `Made with ArtCraft by ${media.maybe_creator_user.display_name}.`
    : "Made with ArtCraft.";

  return {
    ...metadata,
    description,
    openGraph: {
      title: "Shared Media — ArtCraft",
      description,
      url: `/media/${token}`,
      ...(image && { images: [{ url: image }] }),
    },
    twitter: { card: image ? "summary_large_image" : "summary" },
  };
}

export default async function MediaPage({ params }: { params: Params }) {
  const { token } = await params;
  return <MediaViewer token={token} />;
}

async function fetchPublicMediaFile(token: string): Promise<MediaFile | null> {
  try {
    const response = await fetch(`${API_HOST}${mediaFilePath(token)}`, {
      headers: { Accept: "application/json" },
      next: { revalidate: 300 },
    });
    if (!response.ok) return null;
    const payload = (await response.json()) as {
      success?: boolean;
      media_file?: MediaFile;
    };
    return payload.success && payload.media_file ? payload.media_file : null;
  } catch {
    return null;
  }
}
