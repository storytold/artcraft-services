"use client";

import { ArrowRightIcon } from "lucide-react";
import { Button, Modal } from "@/components/ui";
import { webappUrl } from "@/lib/links";

// Shown right after a desktop download starts: invites the visitor to
// create an account while the installer arrives. Downloads themselves
// start natively via the anchor that opened this.
export default function DownloadModal({
  isOpen,
  onClose,
}: {
  isOpen: boolean;
  onClose: () => void;
}) {
  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Downloading ArtCraft…">
      <p className="leading-relaxed text-muted">
        Your download has started. While you wait, create an account to store
        your creations.
      </p>
      <Button href={webappUrl("/signup")} className="mt-6 w-full">
        Sign up
        <ArrowRightIcon aria-hidden className="h-3.5 w-3.5" />
      </Button>
      <a
        href={webappUrl("/login")}
        className="hud-label mt-4 block text-center text-faint hover:text-ink"
      >
        I already have an account
      </a>
    </Modal>
  );
}
