import { ReactNode, useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { CoinsIcon, TagIcon } from "lucide-react";
import { Button } from "@storyteller/ui-button";
import { BillingApi, UsersApi } from "@storyteller/api";
import { PricingTable, PROMO_PCT } from "@storyteller/ui-pricing-table";
import Seo from "../../components/seo";
import { CreditsModal } from "../../components/credits-modal";

const Pricing = () => {
  const [searchParams] = useSearchParams();
  const isSeedanceRef = searchParams.get("ref") === "sd2fakeyou";
  const [creditsModalOpen, setCreditsModalOpen] = useState(false);
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [hasPlan, setHasPlan] = useState(false);

  useEffect(() => {
    const check = async () => {
      try {
        const api = new UsersApi();
        const res = await api.GetSession();
        const loggedIn =
          res.success && !!res.data?.loggedIn && !!res.data?.user;
        setIsLoggedIn(loggedIn);
        if (!loggedIn) return;
        const subs = await new BillingApi().ListActiveSubscriptions();
        setHasPlan(
          !!subs.success &&
            !!subs.data?.active_subscriptions?.some(
              (sub) => sub.namespace === "artcraft",
            ),
        );
      } catch {
        // not logged in
      }
    };
    check();
  }, []);

  return (
    <div className="relative min-h-full bg-ui-background text-white">
      <Seo
        title="Pricing - ArtCraft"
        description="Simple, transparent pricing for ArtCraft. Start for free and scale as you grow."
      />
      <main className="pb-16">
        {!isSeedanceRef && (
          <div className="border-t border-white/15 bg-primary text-white">
            <div className="mx-auto flex max-w-[1280px] flex-wrap items-center justify-between gap-x-6 gap-y-1 px-6 py-2.5 md:px-10">
              <p className="pricing-hud flex items-center gap-2 font-bold">
                <TagIcon aria-hidden className="h-3.5 w-3.5" />
                Limited-time offer
              </p>
              <p className="text-sm">
                Save {PROMO_PCT}% on every plan, monthly or yearly — lock in the
                lowest price today.
              </p>
            </div>
          </div>
        )}
        <PricingSection>
          <SectionLabel
            index="01"
            label={isSeedanceRef ? "Early access" : "Pricing"}
            annotation={
              isSeedanceRef
                ? "Available today in ArtCraft"
                : "Free & open source · Subscriptions optional†"
            }
          />
          <div className="px-6 py-14 md:px-10 md:py-20">
            <h1 className="max-w-3xl font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] sm:text-5xl md:text-6xl">
              {isSeedanceRef ? (
                <>
                  Seedance 2.0 is{" "}
                  <span className="font-serif-italic">here</span>.
                </>
              ) : (
                <>
                  Invest in your{" "}
                  <span className="font-serif-italic">creativity</span>.
                </>
              )}
            </h1>
            <p className="mt-5 max-w-xl text-lg leading-relaxed text-white/55">
              {isSeedanceRef
                ? "Generate jaw-dropping AI videos with Seedance 2.0 before it's available anywhere else. Every paid plan includes video credits, so you can start creating right now."
                : "Get a ton of generations and invest in a tool you'll always own. Your subscription helps keep ArtCraft free and open for everyone."}
            </p>
            {hasPlan && (
              <Button
                variant="secondary"
                className="mt-6 gap-2 rounded-none"
                onClick={() => setCreditsModalOpen(true)}
              >
                <CoinsIcon aria-hidden className="h-4 w-4" />
                Buy more credits
              </Button>
            )}
          </div>
        </PricingSection>
        {isSeedanceRef && (
          <PricingSection>
            <div className="grid lg:grid-cols-[3fr_2fr]">
              <figure className="min-w-0">
                <figcaption className="pricing-hud flex justify-between border-b border-white/15 px-6 py-2.5 text-white/55 md:px-8">
                  <span>Seedance in ArtCraft</span>
                  <span>Reel</span>
                </figcaption>
                <div className="relative aspect-video overflow-hidden bg-black">
                  <iframe
                    src="https://player.vimeo.com/video/1169289718?autoplay=1&muted=1&loop=1&background=0&byline=0&portrait=0&title=0"
                    className="absolute inset-0 h-full w-full"
                    allow="autoplay; fullscreen; picture-in-picture"
                    allowFullScreen
                    title="Seedance in ArtCraft"
                  />
                </div>
              </figure>
              <div className="grid border-t border-white/15 sm:grid-cols-2 lg:grid-cols-1 lg:border-l lg:border-t-0">
                <div className="p-6 md:p-8">
                  <p className="pricing-hud text-primary">
                    Seedance video credits
                  </p>
                  <p className="mt-3 leading-relaxed text-white/55">
                    Included with every paid ArtCraft plan.
                  </p>
                </div>
                <div className="border-t border-white/15 p-6 sm:border-l sm:border-t-0 md:p-8 lg:border-l-0 lg:border-t">
                  <p className="pricing-hud text-white/55">
                    First in the world
                  </p>
                  <p className="mt-3 leading-relaxed text-white/55">
                    Seedance launches in ArtCraft ahead of anywhere else.
                  </p>
                </div>
              </div>
            </div>
          </PricingSection>
        )}
        <PricingSection id="plans">
          <SectionLabel
            index="02"
            label="Choose your plan"
            annotation="Every paid plan includes video credits"
          />
          <PricingTable
            showHeader={false}
            websiteLayout
            showSeedanceFeatures
            showEnterprise
          />
        </PricingSection>
        <PricingSection id="credits">
          <div className="grid md:grid-cols-2">
            <div className="p-6 md:p-10">
              <p className="pricing-hud text-white/40">
                {isLoggedIn ? "Need more credits?" : "Already have an account?"}
              </p>
              <h2 className="mt-4 font-display text-2xl font-medium tracking-[-0.02em]">
                One-time credit packs
              </h2>
              <p className="mt-2 max-w-md leading-relaxed text-white/55">
                Top up without changing your plan. Credit packs never expire.
              </p>
              {isLoggedIn ? (
                <Button
                  variant="secondary"
                  className="mt-6 gap-2 rounded-none"
                  onClick={() => setCreditsModalOpen(true)}
                >
                  <CoinsIcon aria-hidden className="h-4 w-4 text-primary" />
                  Buy credits
                </Button>
              ) : (
                <Link
                  to="/login?from=%2Fpricing"
                  className="pricing-hud mt-6 inline-flex border border-white/15 px-4 py-3 hover:bg-white/10"
                >
                  Sign in to buy credits
                </Link>
              )}
            </div>
            <div className="border-t border-white/15 p-6 md:border-l md:border-t-0 md:p-10">
              <p className="pricing-hud text-white/40">† Footnote</p>
              <p className="mt-4 max-w-md leading-relaxed text-white/55">
                ArtCraft can be used without paying for a subscription. You can
                bring your own compute and third-party subscriptions. We hope
                you’ll subscribe, though, as that helps accelerate our
                development.
              </p>
            </div>
          </div>
        </PricingSection>
      </main>
      {isLoggedIn && (
        <CreditsModal
          isOpen={creditsModalOpen}
          onClose={() => setCreditsModalOpen(false)}
        />
      )}
    </div>
  );
};

function PricingSection({
  id,
  children,
}: {
  id?: string;
  children: ReactNode;
}) {
  return (
    <section id={id} className="border-t border-white/15">
      <div className="relative mx-auto max-w-[1280px] border-x border-white/15">
        {children}
      </div>
    </section>
  );
}

function SectionLabel({
  index,
  label,
  annotation,
}: {
  index: string;
  label: string;
  annotation: string;
}) {
  return (
    <div className="pricing-hud flex items-center justify-between gap-4 border-b border-white/15 px-6 py-3 text-white/55 md:px-10">
      <p>
        <span className="text-white/40">{index} / </span>
        {label}
      </p>
      <p className="hidden text-white/40 sm:block">{annotation}</p>
    </div>
  );
}

export default Pricing;
