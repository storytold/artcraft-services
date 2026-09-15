import type { Metadata } from "next";
import { CheckIcon } from "lucide-react";
import BetaForm from "@/components/beta/beta-form";
import RevealManager from "@/components/reveal-manager";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Accent } from "@/components/page/page-header";

export const metadata: Metadata = {
  title: "Beta Signup",
  description:
    "Apply for early access to new ArtCraft features. Test unreleased models and tools before anyone else, and help shape the fastest open desktop app for AI video and images.",
  alternates: { canonical: "/beta" },
  openGraph: { title: "Join the ArtCraft Beta" },
};

const PERKS = [
  {
    title: "First access to new tools",
    description: "Try unreleased models and features before they ship.",
  },
  {
    title: "A direct line to the team",
    description: "Report issues and requests straight to the people building ArtCraft.",
  },
  {
    title: "Shape the roadmap",
    description: "Your feedback decides what we build next.",
  },
];

export default function BetaPage() {
  return (
    <>
      <RevealManager />

      <SectionShell id="apply">
        <SectionEyebrow index="01" label="ArtCraft beta" annotation="Takes about thirty seconds" />
        <div className="grid gap-px bg-line lg:grid-cols-[1fr_minmax(0,480px)]">
          <div className="bg-bg px-6 py-14 md:px-10 md:py-20">
            <h1
              data-reveal
              className="max-w-3xl font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl md:text-6xl"
            >
              Get new features <Accent>before anyone else</Accent>.
            </h1>
            <p data-reveal className="mt-5 max-w-xl text-lg leading-relaxed text-muted">
              We invite a small group of creators to test what we&rsquo;re
              building next. Tell us a bit about yourself and we&rsquo;ll reach
              out when a spot opens up.
            </p>

            <ul data-reveal-group className="mt-12 flex flex-col gap-px bg-line">
              {PERKS.map((perk, i) => (
                <li key={perk.title} data-reveal className="flex gap-4 bg-bg py-5">
                  <span className="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center border border-line text-accent-ink">
                    <CheckIcon aria-hidden className="h-3.5 w-3.5" />
                  </span>
                  <div className="flex-1">
                    <p className="font-medium text-ink-strong">{perk.title}</p>
                    <p className="mt-1 text-sm leading-relaxed text-muted">
                      {perk.description}
                    </p>
                  </div>
                  <p className="hud-label text-faint">
                    {String(i + 1).padStart(2, "0")}
                  </p>
                </li>
              ))}
            </ul>
          </div>

          <div className="relative bg-bg-raised p-6 md:p-10">
            <p className="hud-label text-faint">Apply for the beta</p>
            <p className="mt-2 mb-8 text-sm text-muted">
              No spam, ever.
            </p>
            <BetaForm />
          </div>
        </div>
      </SectionShell>
    </>
  );
}
