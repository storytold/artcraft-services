import Link from "next/link";
import { AppleIcon } from "lucide-react";
import { LinuxIcon, WindowsIcon } from "@/components/icons";
import { Button } from "@/components/ui";
import { SOCIAL_LINKS, WEBAPP_URL } from "@/lib/links";
import { HERO_LINK_CLASSES, HeroPoster } from "./hero";

// ArtCraft Studio's own landing: the showcase clips ride the galaxy.
export default function StudioHero() {
  return (
    <HeroPoster
      variant="clips"
      title={
        <>
          Capable tools{" "}
          <span className="font-serif italic font-normal text-muted">
            for artists.
          </span>
        </>
      }
      lede="ArtCraft Studio is our generative app: compose scenes yourself, then render them with the latest image and video models."
      actions={
        <div className="flex flex-wrap items-center justify-center gap-3">
          <Button href="/download" size="lg">
            <AppleIcon aria-hidden className="h-4 w-4" />
            <WindowsIcon className="h-4 w-4" />
            <LinuxIcon className="h-4 w-4" />
            Download the studio
          </Button>
          <Button href={WEBAPP_URL} variant="secondary" size="lg">
            Use on web
          </Button>
        </div>
      }
      note={
        <>
          Looking for native desktop apps?{" "}
          <Link href="/apps" className={HERO_LINK_CLASSES}>
            See the Crafting Apps
          </Link>
          .
        </>
      }
      githubHref={SOCIAL_LINKS.GITHUB}
      facts={["macOS · Windows · Linux · Web", "No subscription required"]}
    />
  );
}
