"use client";

import { useState } from "react";
import { CoinsIcon } from "lucide-react";
import { twMerge } from "tailwind-merge";
import { Button, Modal } from "@/components/ui";
import { creditsPackCheckout } from "@/lib/api";

type CreditPack = {
  id: string;
  total: number;
  priceUsd: number;
  badge?: string;
};

// One-time packs, ported from the Vite site's credits modal. Ids are the
// Stripe product slugs the API expects.
const CREDIT_PACKS: CreditPack[] = [
  { id: "artcraft_1000", total: 1000, priceUsd: 10 },
  { id: "artcraft_2500", total: 2500, priceUsd: 25 },
  { id: "artcraft_5000", total: 5000, priceUsd: 50, badge: "Popular" },
  { id: "artcraft_10000", total: 10000, priceUsd: 100 },
];

export default function CreditsModal({
  isOpen,
  onClose,
}: {
  isOpen: boolean;
  onClose: () => void;
}) {
  const [purchasingId, setPurchasingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const purchase = async (pack: CreditPack) => {
    setPurchasingId(pack.id);
    setError(null);
    const result = await creditsPackCheckout(pack.id);
    if (result.success) {
      window.location.href = result.data.checkoutUrl;
      return;
    }
    setError(result.errorMessage);
    setPurchasingId(null);
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      childPadding={false}
      className="max-w-2xl"
      accessibleTitle="Buy credits"
    >
      <div className="border-b border-line px-6 py-5 md:px-8">
        <p className="hud-label text-faint">Credit packs</p>
        <h2 className="mt-3 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
          Buy credits
        </h2>
        <p className="mt-1 text-sm text-muted">
          One-time packs. No subscription required, never expire.
        </p>
      </div>

      <ul className="grid gap-px bg-line sm:grid-cols-2">
        {CREDIT_PACKS.map((pack) => {
          const busy = purchasingId !== null;
          return (
            <li key={pack.id} className="bg-bg-raised">
              <button
                type="button"
                onClick={() => purchase(pack)}
                disabled={busy}
                className={twMerge(
                  "group flex h-full w-full flex-col p-6 text-left transition-colors hover:bg-bg disabled:cursor-not-allowed disabled:opacity-60 md:p-8",
                )}
              >
                <div className="flex items-center justify-between gap-3">
                  <span className="flex items-center gap-2 font-display text-3xl font-medium tracking-[-0.02em] text-ink-strong">
                    <CoinsIcon aria-hidden className="h-5 w-5 text-accent-ink" />
                    {pack.total.toLocaleString()}
                  </span>
                  {pack.badge && (
                    <span className="hud-label bg-accent px-2 py-1 font-bold text-white">
                      {pack.badge}
                    </span>
                  )}
                </div>
                <p className="hud-label mt-2 text-faint">credits</p>
                <div className="mt-6 flex items-center justify-between gap-3">
                  <span className="font-display text-xl font-medium text-ink">
                    ${pack.priceUsd}
                  </span>
                  <Button
                    variant="secondary"
                    size="sm"
                    loading={purchasingId === pack.id}
                    className="pointer-events-none"
                    tabIndex={-1}
                  >
                    Purchase
                  </Button>
                </div>
              </button>
            </li>
          );
        })}
      </ul>

      {error && (
        <p className="border-t border-line px-6 py-3 font-mono text-[10px] uppercase tracking-[0.1em] text-danger md:px-8">
          {error}
        </p>
      )}
    </Modal>
  );
}
