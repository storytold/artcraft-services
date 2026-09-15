// Press kit assets, ported from the Vite site. To add one, copy an existing
// entry:
// - type: "video" | "image" | "embed" | "link"
// - thumbnail: site-relative image path (served via mediaUrl) or absolute URL
// - embedUrl: YouTube embed URL for type "embed"
// - videoUrl: direct video URL for type "video" (defaults to downloadUrl)
// - downloadUrl: the downloadable file (e.g. Cloudflare R2 URL)
// - containThumbnail: letterbox the thumbnail with padding (good for logos)

export type PressKitAssetType = "video" | "image" | "embed" | "link";

export type PressKitAsset = {
  type: PressKitAssetType;
  title: string;
  description?: string;
  thumbnail?: string;
  embedUrl?: string;
  videoUrl?: string;
  downloadUrl: string;
  downloadLabel?: string;
  fileSize?: string;
  containThumbnail?: boolean;
};

export type PressKitCategory = {
  name: string;
  description?: string;
  assets: PressKitAsset[];
};

export const PRESS_CONTACT_PHONE = { display: "(678) 744-6080", tel: "678-744-6080" };

const R2 = "https://pub-f7441936e5804042a1ea2bdc92e4dc71.r2.dev";

export const PRESS_KIT_CATEGORIES: PressKitCategory[] = [
  {
    name: "Promotional videos",
    description: "High-quality promotional videos for press coverage",
    assets: [
      {
        type: "video",
        title: "ArtCraft Commercial",
        description: "Official ArtCraft commercial showcasing the app",
        thumbnail: "/images/video-thumbnails/artcraft-commercial.png",
        downloadUrl: `${R2}/artcraft_website_v2.mp4`,
        fileSize: "125 MB",
      },
      {
        type: "video",
        title: "ArtCraft 3D Sci-Fi Animation",
        description: "3D pre-viz workflow demonstrates precision control",
        thumbnail: "/images/video-thumbnails/artcraft-sci-fi.png",
        downloadUrl: `${R2}/ArtCraft_SciFi_Horror.mp4`,
        fileSize: "14.5 MB",
      },
      {
        type: "video",
        title: "ArtCraft x WorldLabs Pirate Demo",
        description: "ArtCraft with Gaussian splats can be used to quickly create sets",
        thumbnail: "/images/video-thumbnails/artcraft-pirate.png",
        downloadUrl: `${R2}/ArtCraft_World_Pirate.mp4`,
        fileSize: "88.5 MB",
      },
      {
        type: "video",
        title: "ArtCraft Posing and Blocking",
        description: "More examples of detailed posing and blocking in 3D",
        thumbnail: "/images/video-thumbnails/artcraft-knight-pose.png",
        downloadUrl: `${R2}/ArtCraft_Pose_Ad.mp4`,
        fileSize: "60.6 MB",
      },
      {
        type: "video",
        title: "ArtCraft x WorldLabs Ad",
        description: "Several shots crafted using WorldLabs' advanced Marble model",
        thumbnail: "/images/video-thumbnails/artcraft-worldlabs-ad.png",
        downloadUrl: `${R2}/ArtCraft_WorldLabs_Ad.mp4`,
        fileSize: "23.7 MB",
      },
      {
        type: "embed",
        title: "Grinch: The Anime",
        description: "Made using ArtCraft",
        thumbnail: "https://img.youtube.com/vi/oqoCWdOwr2U/maxresdefault.jpg",
        embedUrl: "https://www.youtube-nocookie.com/embed/oqoCWdOwr2U",
        downloadUrl: `${R2}/ArtCraft_Grinch_Anime.mp4`,
        fileSize: "56.3 MB",
      },
    ],
  },
  {
    name: "Logos & branding",
    description: "Official ArtCraft logos and branding assets",
    assets: [
      {
        type: "image",
        title: "ArtCraft Logo (PNG)",
        thumbnail: "/images/artcraft-logo.png",
        downloadUrl: "/images/artcraft-logo.png",
        containThumbnail: true,
      },
    ],
  },
  {
    name: "Screenshots & media",
    description: "High-resolution screenshots and promotional images",
    assets: [],
  },
];
