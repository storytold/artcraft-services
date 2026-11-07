import type { Metadata } from "next";
import MediaPage from "@/components/media/media-page";
import { fetchPublicMediaFile } from "@/lib/fetch-public-media";
import { mediaEmbed, mediaShareImage, MEDIA_SHARE_ORIGIN, MEDIA_SHELL_TOKEN } from "@/lib/media-sharing";
import { mediaTitle } from "@/lib/media";

const FALLBACK_IMAGE = { url: "/images/og-image.png", width: 1200, height: 630, alt: "ArtCraft" };

type Props = { params: Promise<{ token: string }> };

/**
 * Generates static placeholder route parameters when exporting the site for Cloudflare.
 */
export function generateStaticParams() {
  return process.env.NEXT_STATIC_EXPORT === "1" ? [{ token: MEDIA_SHELL_TOKEN }] : [];
}

/**
 * Generates Open Graph, Twitter Card, and oEmbed metadata for a shared media route.
 */
export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { token } = await params;
  const url = `/media/${encodeURIComponent(token)}`;
  const fallbackTitle = "Shared media";
  const fallbackDescription = "View images, video, music, and interactive 3D creations shared with ArtCraft.";
  const metadata: Metadata = {
    title: fallbackTitle,
    description: fallbackDescription,
    alternates: { canonical: url },
    // A shared link is not a request to index a user's creation.
    robots: { index: false, follow: true },
    openGraph: {
      type: "website", siteName: "ArtCraft", url,
      title: `${fallbackTitle} — ArtCraft`, description: fallbackDescription,
      images: [FALLBACK_IMAGE],
    },
    twitter: {
      card: "summary_large_image", title: `${fallbackTitle} — ArtCraft`,
      description: fallbackDescription, images: [FALLBACK_IMAGE],
    },
  };
  if (token === MEDIA_SHELL_TOKEN) return metadata;
  const media = await fetchPublicMediaFile(token);
  if (!media) return metadata;
  const title = mediaTitle(media);
  const image = mediaShareImage(media);
  const embed = mediaEmbed(media);
  const images = image ? [{ url: image, alt: title }] : [FALLBACK_IMAGE];
  const creator = media.maybe_creator_user;
  const description = creator
    ? `Made with ArtCraft by ${creator.display_name || creator.username}.`
    : "Made with ArtCraft.";
  return {
    ...metadata,
    title,
    description,
    openGraph: {
      type: embed?.kind === "video" ? "video.other" : "website",
      ...(embed?.kind === "video" && { videos: [{ url: embed.url, secureUrl: embed.url, type: embed.contentType }] }),
      ...(embed?.kind === "audio" && { audio: [{ url: embed.url, secureUrl: embed.url, type: embed.contentType }] }),
      siteName: "ArtCraft",
      title: `${title} — ArtCraft`,
      description,
      url,
      images,
    },
    twitter: {
      ...(embed ? { card: "player" as const, players: [] } : { card: "summary_large_image" as const }),
      title: `${title} — ArtCraft`,
      description,
      images,
    },
    ...(embed && {
      alternates: { canonical: url, types: { "application/json+oembed": `${MEDIA_SHARE_ORIGIN}${url}/oembed?url=${encodeURIComponent(`${MEDIA_SHARE_ORIGIN}${url}`)}&format=json` } },
      // Next's player descriptor requires a stream URL, while X's optional
      // native stream must be MP4. Audio/WebM use the HTML player instead.
      other: {
        "twitter:player": `${MEDIA_SHARE_ORIGIN}${url}/player`,
        "twitter:player:width": embed.width,
        "twitter:player:height": embed.height,
        ...(embed.contentType === "video/mp4" && {
          "twitter:player:stream": embed.url,
          "twitter:player:stream:content_type": embed.contentType,
        }),
      },
    }),
  };
}

/**
 * Renders the shared media viewer page for a given media token.
 */
export default async function Media({ params }: Props) {
  const { token } = await params;
  // Resolve in the browser, using the same session credentials as the Vite
  // viewer. Never cache one visitor's private media response in server HTML.
  return <MediaPage key={token} token={token} />;
}
