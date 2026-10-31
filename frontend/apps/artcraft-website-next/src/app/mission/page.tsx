import type { Metadata } from "next";
import DiscordButton from "@/components/discord-button";
import RevealManager from "@/components/reveal-manager";
import Ownership from "@/components/landing/ownership";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Accent, PageHeader } from "@/components/page/page-header";
import { Button } from "@/components/ui";

export const metadata: Metadata = {
  title: "Our Mission",
  description:
    "ArtCraft exists to put artists in control of AI image and video. An open-source studio you own, built in the open with the creators who use it.",
  alternates: { canonical: "/mission" },
  openGraph: { title: "The ArtCraft Mission" },
};

const BELIEFS = [
  {
    title: "Control over chance",
    body: "Typing a prompt and hoping isn't a creative process. Pose the characters, frame the camera, compose the shot, and let the model render what you meant.",
  },
  {
    title: "Every model, one studio",
    body: "The best models change every few months. Your workflow shouldn't. Bring each new one into the same canvas instead of rebuilding your process around it.",
  },
  {
    title: "Built with artists",
    body: "We ship early, test with our community, and let working creators decide what comes next. The people using ArtCraft shape it.",
  },
];

export default function MissionPage() {
  return (
    <>
      <RevealManager />

      <PageHeader
        id="mission"
        index="01"
        label="Mission"
        annotation="Made by artists, for artists"
        title={
          <>
            Put artists back in <Accent>control</Accent>.
          </>
        }
        lede="AI can now make an image or a shot in seconds. We're building the studio that lets artists decide exactly what ends up on screen, and own the tools they make it with."
      />

      <SectionShell id="principles">
        <SectionEyebrow index="02" label="What we believe" annotation="Three principles" />
        <ol data-reveal-group className="grid gap-px bg-line md:grid-cols-3">
          {BELIEFS.map((belief, i) => (
            <li key={belief.title} data-reveal className="bg-bg p-6 md:p-8">
              <p className="hud-label text-faint">{String(i + 1).padStart(2, "0")}</p>
              <h2 className="mt-6 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
                {belief.title}
              </h2>
              <p className="mt-2 leading-relaxed text-muted">{belief.body}</p>
            </li>
          ))}
        </ol>
      </SectionShell>

      <Ownership index="03" />

      <SectionShell id="community">
        <div className="flex flex-col items-center px-6 py-16 text-center md:py-24">
          <p className="hud-label text-faint">Join us</p>
          <h2
            data-reveal
            className="mt-6 max-w-2xl font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl"
          >
            Help us <Accent>build</Accent> it.
          </h2>
          <p className="mt-5 max-w-lg leading-relaxed text-muted">
            Beta testers get new tools before anyone else and a direct line to
            the team building them.
          </p>
          <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
            <Button href="/beta" size="lg">
              Join the beta
            </Button>
            <DiscordButton variant="secondary" size="lg" />
          </div>
        </div>
      </SectionShell>
    </>
  );
}
