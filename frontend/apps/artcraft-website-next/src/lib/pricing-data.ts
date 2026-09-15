// Subscription plan roster, ported from @storyteller/subscription and the
// pricing table's promo math. Prices and features are the approved
// marketing numbers — keep them in sync with the webapp's plan data.
//
// Checkout itself (Stripe, session detection, plan switching) lives in the
// webapp; every plan CTA here links there.

export type BillingCadence = "monthly" | "yearly";

export type PlanFeature = {
  text: string;
  seedanceOnly?: boolean;
};

// Per-plan color identity (green/purple/orange/blue from the original
// table). Each maps to a `.plan-*` class in globals.css that sets the
// `--plan` / `--plan-ink` tokens for both themes.
export type PlanColor = "basic" | "pro" | "max" | "enterprise";

export type SubscriptionPlan = {
  slug: string;
  name: string;
  tagline: string;
  color: PlanColor;
  monthlyPrice: number;
  yearlyPrice: number;
  features: PlanFeature[];
  /** Callout tab rendered above the card. */
  highlight?: "Most popular" | "Best value";
};

// Every plan is shown at this single discount off a higher "before" price, on
// BOTH cadences. The charged prices are unchanged — this only drives the
// "% off" pills, the crossed-out anchor, and the banner copy.
export const PROMO_PCT = 20;

export const CONTACT_EMAIL = "hello@storyteller.ai";

export const SUBSCRIPTION_PLANS: SubscriptionPlan[] = [
  {
    slug: "artcraft_basic",
    name: "Basic",
    tagline: "For getting started",
    color: "basic",
    monthlyPrice: 10,
    yearlyPrice: 96,
    features: [
      { text: "1,000 credits / month" },
      { text: "~250 Nano Banana images" },
      { text: "~67 Nano Banana Pro images" },
      { text: "~33 Nano Banana Pro 4K images" },
      { text: "~200 GPT-Image-1.5 images" },
      { text: "~63 seconds Seedance 2.0 video", seedanceOnly: true },
      { text: "~45 seconds Kling 3.0 Pro video" },
      { text: "You own ArtCraft forever" },
    ],
  },
  {
    slug: "artcraft_pro",
    name: "Pro",
    tagline: "For serious creators",
    color: "pro",
    monthlyPrice: 35,
    yearlyPrice: 336,
    highlight: "Most popular",
    features: [
      { text: "3,750 credits / month" },
      { text: "~938 Nano Banana images" },
      { text: "~250 Nano Banana Pro images" },
      { text: "~125 Nano Banana Pro 4K images" },
      { text: "~750 GPT-Image-1.5 images" },
      { text: "~234 seconds Seedance 2.0 video", seedanceOnly: true },
      { text: "~167 seconds Kling 3.0 Pro video" },
      { text: "You own ArtCraft forever" },
    ],
  },
  {
    slug: "artcraft_max",
    name: "Max",
    tagline: "For studios and power users",
    color: "max",
    monthlyPrice: 60,
    yearlyPrice: 576,
    highlight: "Best value",
    features: [
      { text: "6,600 credits / month" },
      { text: "~1,650 Nano Banana images" },
      { text: "~440 Nano Banana Pro images" },
      { text: "~220 Nano Banana Pro 4K images" },
      { text: "~1,320 GPT-Image-1.5 images" },
      { text: "~413 seconds Seedance 2.0 video", seedanceOnly: true },
      { text: "~295 seconds Kling 3.0 Pro video" },
      { text: "You own ArtCraft forever" },
    ],
  },
];

export const ENTERPRISE_FEATURES = [
  "Bespoke credit allocation",
  "Secure models",
  "Support SLAs",
  "Custom integrations",
];

// Reassurance strip under the plan grid. Only claims the product backs:
// Stripe checkout/portal, cancel-anytime, Discord credit refunds, and the
// open-source ownership pitch.
export const TRUST_POINTS = [
  "Cancel anytime",
  "Secure checkout via Stripe",
  "Credits refunded if a generation fails",
  "Open source — yours forever",
];

export type PlanPricing = {
  /** Displayed per-month price for the cadence, whole dollars. */
  current: number;
  /** Crossed-out "before" price the discount is applied to, whole dollars. */
  basePrice: number;
  /** Whole-dollar saving per year on the yearly cadence vs paying monthly. */
  yearlySavings: number;
};

// The displayed price is the real charge; the anchor is current ÷ (1 − promo),
// rounded so no price ever shows cents.
export function planPricing(
  plan: SubscriptionPlan,
  cadence: BillingCadence,
): PlanPricing {
  const current =
    cadence === "yearly" ? Math.round(plan.yearlyPrice / 12) : plan.monthlyPrice;
  return {
    current,
    basePrice: Math.round(current / (1 - PROMO_PCT / 100)),
    yearlySavings: plan.monthlyPrice * 12 - plan.yearlyPrice,
  };
}
