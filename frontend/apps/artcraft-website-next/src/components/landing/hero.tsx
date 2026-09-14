import { HERO_VIDEO_URL } from "@/lib/landing-data";
import HeroMasthead, { HeroScrim } from "./hero-wordmark";
import HeroGalaxy from "./hero-galaxy";
import HeroViewport from "./hero-viewport";
import HeroCopy from "./hero-headline";
import { SlateFrame, FlankTelemetry, ProofStrip } from "./hero-slate";

export default function Hero() {
  return (
    <section id="hero" className="relative">
      {/* Single-stage radial poster: the wordmark at the center is the
          emanation point of the galaxy — showcase cards born blurred behind
          the brand mark, swirling out along the arms. Everything that sells
          (headline, CTAs, proof) stacks beneath the mark, all above the
          fold. The galaxy bleeds full-bleed past the content rails. */}
      <div className="relative flex min-h-[calc(100svh-3rem)] flex-col">
        <HeroGalaxy />

        {/* Viewfinder slate: corner framing brackets + timecode, and live
            telemetry at the far flanks — width-adaptive chrome that turns
            a wide stage's empty margins into annotated, measured space. */}
        <SlateFrame />
        <FlankTelemetry />

        {/* z-40 (not z-10): this container is a stacking context, so the
            masthead's own z-40 is capped by it — the hero letters must
            outrank the ruler's fixed z-39 contrast pools at root level.
            No border-x here: the poster viewport stays immersive edge to
            edge — the structural rails begin at the announce strip. */}
        <div className="pointer-events-none relative z-40 mx-auto flex w-full max-w-[1280px] flex-1 flex-col items-center justify-center px-6 py-10 md:px-10">
          <div
            data-reveal-group
            className="relative flex w-full max-w-3xl flex-col items-center text-center"
          >
            {/* Soft pocket in the nebula so the sales layer always reads
                (tunable in the Wordmark tuner group). */}
            <HeroScrim />

            <HeroMasthead />

            {/* Specimen copy stack: fluid poster type + variable-axis life
                (intro settle, cursor field) — see hero-headline.tsx. */}
            <HeroCopy />
          </div>
        </div>

        {/* Full-width ruled proof line at the poster's bottom edge — the
            horizontal anchor that spans any stage width. */}
        <ProofStrip />
      </div>

      <div className="relative mx-auto max-w-[1280px] border-x border-line">
        {/* Announce strip — the fold line under the poster. */}
        <div className="relative flex items-center justify-between gap-4 border-y border-line px-6 py-3 md:px-10">
          <span aria-hidden className="tick -top-[6px] -left-[6px]" />
          <span aria-hidden className="tick -top-[6px] -right-[5px]" />
          <p className="hud-label text-muted">Open-source AI studio</p>
          <p className="hud-label text-accent-ink">
            Now with Seedance 2.5, Nano Banana 2 &amp; more
          </p>
        </div>

        {/* Viewport frame — the product-demo stage: blocking vs. AI render. */}
        <figure className="relative">
          <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2 md:px-10">
            <figcaption className="hud-label text-faint">
              Viewport — scene 01
            </figcaption>
            <p aria-hidden className="hud-label text-faint">
              <span className="mr-1.5 inline-block h-1.5 w-1.5 bg-accent align-middle" />
              Blocking ↔ render · move your cursor
            </p>
          </div>
          <div className="relative aspect-video w-full overflow-hidden bg-bg-sunken">
            <HeroViewport
              videoSrc={HERO_VIDEO_URL}
              videoLabel="ArtCraft product reel: composing 3D scenes and rendering them with AI"
            />
            <span aria-hidden className="tick top-2 left-2 opacity-60" />
            <span aria-hidden className="tick top-2 right-2 opacity-60" />
            <span aria-hidden className="tick bottom-2 left-2 opacity-60" />
            <span aria-hidden className="tick bottom-2 right-2 opacity-60" />
          </div>
        </figure>
      </div>
    </section>
  );
}
