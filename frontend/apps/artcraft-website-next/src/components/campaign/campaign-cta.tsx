"use client";

import { useState } from "react";
import { AppleIcon, GlobeIcon, SparklesIcon } from "lucide-react";
import { WindowsIcon } from "@/components/icons";
import DownloadModal from "@/components/download/download-modal";
import { Button } from "@/components/ui";
import { DOWNLOADS_ENABLED } from "@/lib/download-links";
import { webappUrl } from "@/lib/links";
import { useAccount } from "@/lib/use-account";
import { markDownloadInitiated, useDetectedPlatform } from "@/lib/use-platform";

// Campaign CTA pair, ported from the Vite landing pages: a primary "use on
// web" button that routes subscribers straight into the webapp and everyone
// else to /pricing, plus the OS-detected desktop download (with the
// post-download signup modal for logged-out visitors). Mobile visitors get
// a disabled download and the web button only.
export default function CampaignCta({
  webLabel = "Use on web",
  webIcon = "globe",
  className,
}: {
  webLabel?: string;
  webIcon?: "globe" | "sparkles";
  className?: string;
}) {
  const { user, activePlanSlug } = useAccount();
  const detected = useDetectedPlatform();
  const [modalOpen, setModalOpen] = useState(false);

  const hasSubscription = !!user && !!activePlanSlug;
  const WebIcon = webIcon === "sparkles" ? SparklesIcon : GlobeIcon;
  const OsIcon = detected?.platform === "macos" ? AppleIcon : WindowsIcon;

  const onDownloadClick = () => {
    if (user) return;
    setModalOpen(true);
    markDownloadInitiated();
  };

  return (
    <div className={className}>
      <div className="flex flex-wrap items-center justify-center gap-3">
        <Button
          href={hasSubscription ? webappUrl("/") : "/pricing"}
          size="lg"
          title="Use ArtCraft in your browser"
        >
          <WebIcon aria-hidden className="h-4 w-4" />
          {webLabel}
        </Button>
        {detected?.mobile ? (
          <Button size="lg" variant="secondary" disabled>
            Download on a desktop
          </Button>
        ) : (
          DOWNLOADS_ENABLED && (
            <Button
              href={detected?.downloadUrl ?? "/download"}
              external={!!detected}
              onClick={onDownloadClick}
              size="lg"
              variant="secondary"
            >
              <OsIcon aria-hidden className="h-4 w-4" />
              Download for {detected?.short ?? "desktop"}
            </Button>
          )
        )}
      </div>
      <DownloadModal isOpen={modalOpen} onClose={() => setModalOpen(false)} />
    </div>
  );
}
