"use client";

import { useState } from "react";
import { CoinsIcon } from "lucide-react";
import { Button } from "@/components/ui";
import { webappUrl } from "@/lib/links";
import { useAccount } from "@/lib/use-account";
import CreditsModal from "./credits-modal";

// Credit-pack cell on the pricing page. Logged-in visitors buy directly via
// Stripe (the modal); everyone else is sent to the app to sign in first.
export default function CreditsCta() {
  const { user, loading } = useAccount();
  const [open, setOpen] = useState(false);

  return (
    <>
      <p className="hud-label text-faint">
        {user ? `Signed in as ${user.display_name}` : "Already have an account?"}
      </p>
      <h2 className="mt-4 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
        One-time credit packs
      </h2>
      <p className="mt-2 max-w-md leading-relaxed text-muted">
        Top up without changing your plan. Credit packs never expire.
      </p>
      {user ? (
        <Button
          type="button"
          variant="secondary"
          onClick={() => setOpen(true)}
          className="mt-6"
        >
          <CoinsIcon aria-hidden className="h-3.5 w-3.5 text-accent-ink" />
          Buy credits
        </Button>
      ) : (
        <Button
          href={webappUrl("/pricing")}
          variant="secondary"
          loading={loading}
          className="mt-6"
        >
          Sign in to buy credits
        </Button>
      )}
      {user && <CreditsModal isOpen={open} onClose={() => setOpen(false)} />}
    </>
  );
}
