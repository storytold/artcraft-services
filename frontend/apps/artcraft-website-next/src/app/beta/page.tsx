import type { Metadata } from "next";
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

export default function BetaPage() {
  return (
    <>
      <RevealManager />

      <SectionShell id="beta">
        <SectionEyebrow index="01" label="ArtCraft beta" annotation="Takes about a minute" />
        <div className="flex justify-center bg-bg px-6 py-14 md:px-10 md:py-20">
          <div className="w-full max-w-120">
            <h1
              data-reveal
              className="text-center font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl"
            >
              Join the <Accent>early beta</Accent>.
            </h1>
            <p data-reveal className="mt-4 text-center text-lg leading-relaxed text-muted">
              We&rsquo;re looking to select a few to join the early beta.
            </p>
            <div data-reveal className="mt-10 border border-line bg-bg-raised p-6 md:p-8">
              <BetaForm />
            </div>
          </div>
        </div>
      </SectionShell>
    </>
  );
}
