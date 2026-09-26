import type { Metadata } from "next";
import MediaPage from "@/components/media/media-page";

type Props = { params: Promise<{ token: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { token } = await params;
  return {
    title: "Shared media",
    description: "View images, video, music, and interactive 3D creations shared with ArtCraft.",
    alternates: { canonical: `/media/${encodeURIComponent(token)}` },
    // A shared link is not a request to index a user's creation.
    robots: { index: false, follow: true },
  };
}

export default async function Media({ params }: Props) {
  const { token } = await params;
  // Resolve in the browser, using the same session credentials as the Vite
  // viewer. Never cache one visitor's private media response in server HTML.
  return <MediaPage key={token} token={token} />;
}
