import { ArrowUpRightIcon } from "lucide-react";
import { FEATURES, HERO_VIDEO_URL } from "@/lib/landing-data";
import { WEBAPP_URL } from "@/lib/links";
import LazyVideo from "@/components/lazy-video";
import HeroViewport from "./hero-viewport";
import { SectionShell, SectionEyebrow } from "./section-shell";

// Feature grid: hairline-separated cells (gap-px over the line color).
// The studio demo leads full-width — the announce strip as its header, the
// blocking↔render comparator as its body — then the seven features and the
// launch-CTA cell split an even two-column grid. Every feature cell is
// index + label + real product footage + copy — no illustration, only the
// actual tool.
export default function Features() {
  return (
    <SectionShell id="features">
      <SectionEyebrow
        index="01"
        label="Crafting features"
        annotation="Real footage — captured in ArtCraft"
      />

      <div className="px-6 py-14 md:px-10 md:py-20">
        <h2
          data-reveal
          className="max-w-3xl font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl md:text-6xl"
        >
          The control that mere{" "}
          <span className="font-serif italic font-normal">words</span> cannot
          buy.
        </h2>
        <p data-reveal className="mt-5 max-w-xl text-lg leading-relaxed text-muted">
          Text prompting is neat, but artists crave control. Compose the shot
          yourself — then let the model render it.
        </p>
      </div>

      <div
        data-reveal-group
        className="grid gap-px border-t border-line bg-line md:grid-cols-2"
      >
        {/* Lead demo: the studio itself — compose the blocking, mouse across
            to the AI render. */}
        <figure data-reveal className="bg-bg md:col-span-2">
          <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-8">
            <p className="hud-label text-muted">Open-source AI studio</p>
            <p className="hud-label hidden text-accent-ink sm:block">
              Now with Seedance 2.5, Nano Banana 2 &amp; more
            </p>
          </div>
          <div className="relative aspect-video w-full overflow-hidden bg-bg-sunken md:aspect-[21/9]">
            <HeroViewport
              videoSrc={HERO_VIDEO_URL}
              videoLabel="ArtCraft product reel: composing 3D scenes and rendering them with AI"
            />
            {/* Cursor hint — meaningful only where the comparator mounts
                (fine pointers, md+). */}
            <p
              aria-hidden
              className="hud-label absolute right-3 top-3 hidden items-center gap-1.5 bg-invert-bg px-3 py-1.5 font-bold text-invert-fg md:flex"
            >
              <span className="inline-block h-1.5 w-1.5 bg-accent" />
              Blocking ↔ render · move your cursor
            </p>
            <span aria-hidden className="tick top-2 left-2 opacity-60" />
            <span aria-hidden className="tick bottom-2 left-2 opacity-60" />
            <span aria-hidden className="tick bottom-2 right-2 opacity-60" />
          </div>
        </figure>

        {FEATURES.map((feature) => (
          <FeatureCell key={feature.index} feature={feature} />
        ))}

        {/* Eighth cell: the launch CTA, full-sized like its siblings — the
            grid stays even and the section ends on a door into the app. */}
        <a data-reveal href={WEBAPP_URL} className="group flex flex-col bg-bg">
          <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-8">
            <p className="hud-label text-muted">More in the app</p>
            <p className="hud-label text-faint">08</p>
          </div>
          <div className="relative flex aspect-video w-full flex-col items-center justify-center gap-4 overflow-hidden bg-bg-sunken">
            <span className="flex h-14 w-14 items-center justify-center border border-line-strong text-ink transition-colors group-hover:bg-invert-bg group-hover:text-invert-fg">
              <ArrowUpRightIcon className="h-6 w-6" />
            </span>
            <p className="hud-label text-faint">app.getartcraft.com</p>
            <span aria-hidden className="tick top-2 left-2 opacity-60" />
            <span aria-hidden className="tick bottom-2 right-2 opacity-60" />
          </div>
          <div className="px-6 py-6 md:px-8">
            <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
              Launch the studio
            </h3>
            <p className="mt-2 max-w-xl leading-relaxed text-muted">
              Generate, edit, and compose with every model — right in your
              browser, no install needed.
            </p>
          </div>
        </a>
      </div>
    </SectionShell>
  );
}

function FeatureCell({ feature }: { feature: (typeof FEATURES)[number] }) {
  return (
    <article data-reveal className="bg-bg">
      <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-8">
        <p className="hud-label text-muted">{feature.label}</p>
        <p className="hud-label text-faint">{feature.index}</p>
      </div>
      <div className="relative aspect-video w-full overflow-hidden bg-bg-sunken">
        <LazyVideo
          src={feature.video}
          label={`${feature.title} demo`}
          className="absolute inset-0 h-full w-full object-cover"
        />
      </div>
      <div className="px-6 py-6 md:px-8">
        <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
          {feature.title}
        </h3>
        <p className="mt-2 max-w-xl leading-relaxed text-muted">
          {feature.description}
        </p>
      </div>
    </article>
  );
}
