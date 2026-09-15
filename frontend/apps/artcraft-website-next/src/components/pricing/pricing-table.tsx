"use client";

import { useState } from "react";
import { CheckIcon, CopyIcon, MailIcon } from "lucide-react";
import { twMerge } from "tailwind-merge";
import { Badge, Button, TabSelector } from "@/components/ui";
import { webappUrl } from "@/lib/links";
import {
  CONTACT_EMAIL,
  ENTERPRISE_FEATURES,
  PROMO_PCT,
  SUBSCRIPTION_PLANS,
  planPricing,
  type BillingCadence,
  type SubscriptionPlan,
} from "@/lib/pricing-data";

const BILLING_TABS: { id: BillingCadence; label: string }[] = [
  { id: "yearly", label: "Yearly" },
  { id: "monthly", label: "Monthly" },
];

// Plan grid in the landing's hairline-cell language: every plan is a cell
// (gap-px over the line color) with an index row, price block, CTA, and
// feature list. Highlighted plans invert their index row instead of
// breaking out of the grid. Checkout happens in the webapp — the CTAs hand
// off there, carrying the referral query through.
export default function PricingTable({
  showSeedanceFeatures = false,
  checkoutQuery = "",
}: {
  showSeedanceFeatures?: boolean;
  /** Query string (without "?") forwarded to the webapp's pricing page. */
  checkoutQuery?: string;
}) {
  const [cadence, setCadence] = useState<BillingCadence>("yearly");
  const checkoutHref = webappUrl(
    `/pricing${checkoutQuery ? `?${checkoutQuery}` : ""}`,
  );

  return (
    <>
      <div className="flex flex-wrap items-center justify-between gap-4 border-b border-line px-6 py-4 md:px-10">
        <div className="flex items-center gap-3">
          <TabSelector
            tabs={BILLING_TABS}
            activeTab={cadence}
            onTabChange={(id) => setCadence(id as BillingCadence)}
            tabClassName="w-24"
          />
          <Badge
            label={`${PROMO_PCT}% off`}
            className="border-transparent bg-accent text-white"
          />
        </div>
        <p className="hud-label hidden text-faint sm:block">
          {cadence === "yearly" ? "Billed yearly · USD" : "Billed monthly · USD"}
        </p>
      </div>

      <div
        data-reveal-group
        className="grid gap-px bg-line md:grid-cols-2 xl:grid-cols-4"
      >
        {SUBSCRIPTION_PLANS.map((plan, i) => (
          <PlanCell
            key={plan.slug}
            plan={plan}
            index={String(i + 1).padStart(2, "0")}
            cadence={cadence}
            showSeedanceFeatures={showSeedanceFeatures}
            href={checkoutHref}
          />
        ))}
        <EnterpriseCell index={String(SUBSCRIPTION_PLANS.length + 1).padStart(2, "0")} />
      </div>
    </>
  );
}

function PlanCell({
  plan,
  index,
  cadence,
  showSeedanceFeatures,
  href,
}: {
  plan: SubscriptionPlan;
  index: string;
  cadence: BillingCadence;
  showSeedanceFeatures: boolean;
  href: string;
}) {
  const { current, basePrice } = planPricing(plan, cadence);
  const highlighted = !!plan.highlight;
  const features = plan.features.filter(
    (f) => !f.seedanceOnly || showSeedanceFeatures,
  );

  return (
    <article data-reveal className="flex flex-col bg-bg">
      <CellIndexRow
        index={index}
        label={plan.highlight ?? "Plan"}
        inverted={highlighted}
      />

      <div className="flex flex-1 flex-col p-6 md:p-8">
        <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
          {plan.name}
        </h3>

        <div className="mt-5 flex items-baseline gap-2">
          <span className="font-mono text-base text-faint line-through decoration-danger/70">
            ${basePrice}
          </span>
          <span className="font-display text-4xl font-medium tracking-[-0.03em] text-ink-strong">
            ${current}
          </span>
          <span className="text-muted">/month</span>
        </div>
        <p className="hud-label mt-2 text-faint">
          {cadence === "yearly"
            ? `Billed yearly · $${plan.yearlyPrice}/yr`
            : "Billed monthly"}
        </p>

        <Button
          href={href}
          variant={highlighted ? "primary" : "secondary"}
          className="mt-6 w-full"
        >
          Get {plan.name}
        </Button>

        <FeatureList items={features.map((f) => f.text)} className="mt-8" />
      </div>
    </article>
  );
}

function EnterpriseCell({ index }: { index: string }) {
  return (
    <article id="enterprise" data-reveal className="flex flex-col bg-bg">
      <CellIndexRow index={index} label="Enterprise" />

      <div className="flex flex-1 flex-col p-6 md:p-8">
        <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
          Enterprise
        </h3>
        <div className="mt-5 flex items-baseline gap-2">
          <span className="font-display text-4xl font-medium tracking-[-0.03em] text-ink-strong">
            Custom
          </span>
        </div>
        <p className="hud-label mt-2 text-faint">For bespoke solutions</p>

        <ContactButtons className="mt-6" />

        <p className="hud-label mt-8 text-faint">Everything in Max, plus</p>
        <FeatureList items={ENTERPRISE_FEATURES} className="mt-3" />
      </div>
    </article>
  );
}

// Mono index strip at the top of every cell, inverted for highlighted plans.
function CellIndexRow({
  index,
  label,
  inverted = false,
}: {
  index: string;
  label: string;
  inverted?: boolean;
}) {
  return (
    <div
      className={twMerge(
        "flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-8",
        inverted && "bg-invert-bg text-invert-fg",
      )}
    >
      <p className={twMerge("hud-label", inverted ? "font-bold" : "text-muted")}>
        {label}
      </p>
      <p className={twMerge("hud-label", inverted ? "opacity-70" : "text-faint")}>
        {index}
      </p>
    </div>
  );
}

function FeatureList({
  items,
  className,
}: {
  items: string[];
  className?: string;
}) {
  return (
    <ul className={twMerge("flex flex-col gap-2.5", className)}>
      {items.map((text) => (
        <li key={text} className="flex items-start gap-2.5 text-sm text-muted">
          <CheckIcon
            aria-hidden
            className="mt-0.5 h-4 w-4 shrink-0 text-accent-ink"
          />
          {text}
        </li>
      ))}
    </ul>
  );
}

// Mailto plus copy-to-clipboard, so visitors without a configured mail
// client can still grab the address.
function ContactButtons({ className }: { className?: string }) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(CONTACT_EMAIL);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard unavailable (insecure context) — the mailto still works.
    }
  };

  return (
    <div className={twMerge("flex gap-px bg-line", className)}>
      <Button
        href={`mailto:${CONTACT_EMAIL}`}
        variant="secondary"
        className="flex-1 border-0"
      >
        <MailIcon aria-hidden className="h-3.5 w-3.5" />
        Contact us
      </Button>
      <Button
        type="button"
        variant="secondary"
        onClick={copy}
        aria-label={copied ? "Email copied" : "Copy email address"}
        className="w-11 border-0 px-0"
      >
        {copied ? (
          <CheckIcon aria-hidden className="h-4 w-4" />
        ) : (
          <CopyIcon aria-hidden className="h-4 w-4" />
        )}
      </Button>
    </div>
  );
}
