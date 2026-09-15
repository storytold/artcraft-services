"use client";

import { useState } from "react";
import {
  ArrowRightIcon,
  CheckIcon,
  CopyIcon,
  GemIcon,
  MailIcon,
  StarIcon,
} from "lucide-react";
import { twMerge } from "tailwind-merge";
import { Badge, Button, TabSelector } from "@/components/ui";
import {
  getPortalUrl,
  subscriptionCheckout,
  switchPlan,
  userSignupSubscriptionCheckout,
} from "@/lib/api";
import {
  CONTACT_EMAIL,
  ENTERPRISE_FEATURES,
  PROMO_PCT,
  SUBSCRIPTION_PLANS,
  TRUST_POINTS,
  planPricing,
  type BillingCadence,
  type SubscriptionPlan,
} from "@/lib/pricing-data";
import {
  getLandingUrl,
  getReferralCode,
  getReferralUsername,
  getReferrer,
} from "@/lib/referral";
import { useAccount } from "@/lib/use-account";

const BILLING_TABS: { id: BillingCadence; label: string }[] = [
  { id: "yearly", label: "Yearly" },
  { id: "monthly", label: "Monthly" },
];

const HIGHLIGHT_ICONS = {
  "Most popular": StarIcon,
  "Best value": GemIcon,
} as const;

// Solid plan-colored CTA: overrides the primary variant's invert colors.
const PLAN_BUTTON_CLASSES =
  "w-full bg-(--plan) text-white hover:opacity-90";

// Plan grid in the landing's hairline-cell language, color-coded per plan
// (the original table's green / purple / orange / blue via `.plan-*`
// tokens): a solid color index tab, a wash pooling beneath it, plan-colored
// checks and CTA, and a 2px frame on the highlighted tiers.
//
// Checkout is the Vite table's flow, ported: logged out → Stripe signup
// checkout (with referral attribution); logged in without a plan → Stripe
// subscription checkout; logged in with a plan → Stripe portal plan switch.
// The server HTML renders the logged-out state; hydration upgrades it.
export default function PricingTable({
  showSeedanceFeatures = false,
}: {
  showSeedanceFeatures?: boolean;
}) {
  const [cadence, setCadence] = useState<BillingCadence>("yearly");
  const { user, activePlanSlug, loading } = useAccount();
  const [processingPlan, setProcessingPlan] = useState<string | null>(null);
  const [managing, setManaging] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const hasActivePlan = !!activePlanSlug && activePlanSlug !== "free";

  const redirect = (result: { success: true; data: { checkoutUrl: string } | { portalUrl: string } } | { success: false; errorMessage: string }) => {
    if (!result.success) {
      setError(result.errorMessage);
      return false;
    }
    window.location.href =
      "checkoutUrl" in result.data ? result.data.checkoutUrl : result.data.portalUrl;
    return true;
  };

  const choosePlan = async (plan: SubscriptionPlan) => {
    if (plan.slug === activePlanSlug) return;
    setProcessingPlan(plan.slug);
    setError(null);
    const body = { plan: plan.slug, cadence };
    const redirected = redirect(
      !user
        ? await userSignupSubscriptionCheckout({
            ...body,
            maybeReferralUrl: getReferrer(),
            maybeLandingUrl: getLandingUrl(),
            maybeReferralUsername: getReferralUsername(),
            maybeReferralCode: getReferralCode(),
          })
        : hasActivePlan
          ? await switchPlan(body)
          : await subscriptionCheckout(body),
    );
    if (!redirected) setProcessingPlan(null);
  };

  const managePlan = async () => {
    setManaging(true);
    setError(null);
    if (!redirect(await getPortalUrl())) setManaging(false);
  };

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
            label={`${PROMO_PCT}% off · ends soon`}
            className="border-transparent bg-accent text-white"
          />
          <p className="hud-label hidden text-accent-ink sm:block">
            {cadence === "yearly" ? "2 months free" : "Switch to yearly, save 20%"}
          </p>
        </div>
        <p className="hud-label hidden text-faint sm:block">
          {user ? `Signed in as ${user.display_name}` : "Prices in USD"}
        </p>
      </div>

      {error && (
        <p
          role="alert"
          className="border-b border-line px-6 py-3 font-mono text-[10px] uppercase tracking-[0.1em] text-danger md:px-10"
        >
          {error}
        </p>
      )}

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
            isCurrent={plan.slug === activePlanSlug}
            ctaLabel={
              plan.slug === activePlanSlug
                ? "Current plan"
                : hasActivePlan
                  ? `Switch to ${plan.name}`
                  : `Get ${plan.name}`
            }
            disabled={loading || processingPlan !== null}
            processing={processingPlan === plan.slug}
            onChoose={() => choosePlan(plan)}
          />
        ))}
        <EnterpriseCell index={String(SUBSCRIPTION_PLANS.length + 1).padStart(2, "0")} />
      </div>

      <div className="flex flex-wrap items-center justify-between gap-x-8 gap-y-2 border-t border-line px-6 py-4 md:px-10">
        <ul className="flex flex-wrap items-center gap-x-8 gap-y-2">
          {TRUST_POINTS.map((point) => (
            <li key={point} className="hud-label flex items-center gap-2 text-muted">
              <CheckIcon aria-hidden className="h-3.5 w-3.5 text-accent-ink" />
              {point}
            </li>
          ))}
        </ul>
        {hasActivePlan && (
          <button
            type="button"
            onClick={managePlan}
            disabled={managing}
            className="hud-label text-muted underline underline-offset-4 hover:text-ink disabled:opacity-50"
          >
            {managing ? "Opening portal…" : "Manage plan"}
          </button>
        )}
      </div>
    </>
  );
}

function PlanCell({
  plan,
  index,
  cadence,
  showSeedanceFeatures,
  isCurrent,
  ctaLabel,
  disabled,
  processing,
  onChoose,
}: {
  plan: SubscriptionPlan;
  index: string;
  cadence: BillingCadence;
  showSeedanceFeatures: boolean;
  isCurrent: boolean;
  ctaLabel: string;
  disabled: boolean;
  processing: boolean;
  onChoose: () => void;
}) {
  const { current, basePrice, yearlySavings } = planPricing(plan, cadence);
  const HighlightIcon = plan.highlight ? HIGHLIGHT_ICONS[plan.highlight] : null;
  const features = plan.features.filter(
    (f) => !f.seedanceOnly || showSeedanceFeatures,
  );

  return (
    <article
      data-reveal
      data-highlight={plan.highlight || isCurrent ? "" : undefined}
      className={`plan-card plan-${plan.color} flex flex-col`}
    >
      <CellIndexRow index={index}>
        {isCurrent ? (
          <>
            <CheckIcon aria-hidden className="h-3 w-3" />
            Your plan
          </>
        ) : (
          <>
            {HighlightIcon && <HighlightIcon aria-hidden className="h-3 w-3" />}
            {plan.highlight ?? "Plan"}
          </>
        )}
      </CellIndexRow>

      <div className="flex flex-1 flex-col p-6 md:p-8">
        <div className="flex items-center justify-between gap-3">
          <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
            {plan.name}
          </h3>
          <span className="hud-label bg-(--plan) px-2 py-1 font-bold text-white">
            {PROMO_PCT}% off
          </span>
        </div>
        <p className="mt-1 text-sm text-muted">{plan.tagline}</p>

        <div className="mt-5 flex items-baseline gap-2">
          <span className="font-mono text-base text-danger/80 line-through">
            ${basePrice}
          </span>
          <span className="font-display text-5xl font-medium tracking-[-0.03em] text-ink-strong">
            ${current}
          </span>
          <span className="text-muted">/mo</span>
        </div>
        <p className="hud-label mt-2 text-faint">
          {cadence === "yearly" ? (
            <>
              ${plan.yearlyPrice} billed yearly ·{" "}
              <span className="text-(--plan-ink)">Save ${yearlySavings}</span>
            </>
          ) : (
            "Billed monthly · cancel anytime"
          )}
        </p>

        <Button
          type="button"
          size="lg"
          onClick={onChoose}
          disabled={disabled || isCurrent}
          loading={processing}
          className={twMerge(PLAN_BUTTON_CLASSES, "mt-6", isCurrent && "opacity-60")}
        >
          {ctaLabel}
          {!isCurrent && <ArrowRightIcon aria-hidden className="h-3.5 w-3.5" />}
        </Button>

        <p className="hud-label mt-8 text-faint">What you get</p>
        <FeatureList items={features.map((f) => f.text)} className="mt-3" />
      </div>
    </article>
  );
}

function EnterpriseCell({ index }: { index: string }) {
  return (
    <article
      id="enterprise"
      data-reveal
      className="plan-card plan-enterprise flex flex-col"
    >
      <CellIndexRow index={index}>For teams</CellIndexRow>

      <div className="flex flex-1 flex-col p-6 md:p-8">
        <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
          Enterprise
        </h3>
        <div className="mt-5 flex items-baseline gap-2">
          <span className="font-display text-5xl font-medium tracking-[-0.03em] text-ink-strong">
            Custom
          </span>
        </div>
        <p className="hud-label mt-2 text-faint">Bespoke volume &amp; terms</p>

        <ContactButtons className="mt-6" />

        <p className="hud-label mt-8 text-faint">Everything in Max, plus</p>
        <FeatureList items={ENTERPRISE_FEATURES} className="mt-3" />
      </div>
    </article>
  );
}

// Solid plan-colored index strip at the top of every cell.
function CellIndexRow({
  index,
  children,
}: {
  index: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 bg-(--plan) px-6 py-2.5 text-white md:px-8">
      <p className="hud-label flex items-center gap-1.5 font-bold">{children}</p>
      <p className="hud-label opacity-70">{index}</p>
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
        <li key={text} className="flex items-start gap-2.5 text-sm text-ink">
          <span className="plan-check mt-px flex h-4 w-4 shrink-0 items-center justify-center">
            <CheckIcon aria-hidden className="h-3 w-3" />
          </span>
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
    <div className={twMerge("flex gap-px bg-white/30", className)}>
      <Button
        href={`mailto:${CONTACT_EMAIL}`}
        size="lg"
        className={twMerge(PLAN_BUTTON_CLASSES, "flex-1")}
      >
        <MailIcon aria-hidden className="h-3.5 w-3.5" />
        Contact us
      </Button>
      <Button
        type="button"
        size="lg"
        onClick={copy}
        aria-label={copied ? "Email copied" : "Copy email address"}
        className={twMerge(PLAN_BUTTON_CLASSES, "w-12 px-0")}
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
