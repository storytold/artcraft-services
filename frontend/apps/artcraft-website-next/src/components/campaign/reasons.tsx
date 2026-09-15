import { CameraIcon, FilmIcon, PaintbrushIcon } from "lucide-react";
import { GitHubIcon } from "@/components/icons";
import { Accent } from "@/components/page/page-header";
import { MODEL_BADGES, REASON_IMAGES } from "@/lib/campaign-data";
import { SOCIAL_LINKS } from "@/lib/links";
import { CampaignSection } from "./sections";

// "Five reasons" bento from the Seedance pages, rebuilt as a hairline cell
// grid: two wide cells, a 1/3 + 2/3 row, and a full-width closer.
export default function Reasons({ index }: { index: string }) {
  return (
    <CampaignSection
      id="reasons"
      index={index}
      label="Why ArtCraft"
      annotation="Five reasons"
      title={
        <>
          Five reasons it&rsquo;s the <Accent>best tool</Accent>.
        </>
      }
    >
      <div data-reveal-group className="grid gap-px border-t border-line bg-line md:grid-cols-6">
        {/* 1 — Control beyond text prompting */}
        <article data-reveal className="flex flex-col bg-bg md:col-span-3">
          <CellRow label="Control" index="01" />
          <div className="relative aspect-[21/9] w-full overflow-hidden bg-bg-sunken">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={REASON_IMAGES.canvas}
              alt="2D canvas and 3D scene side by side"
              loading="lazy"
              className="absolute inset-0 h-full w-full object-cover"
            />
          </div>
          <div className="p-6 md:p-8">
            <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
              Control beyond text prompting
            </h3>
            <p className="mt-2 leading-relaxed text-muted">
              <span className="text-ink">
                Create images and videos with our easy-to-use AI tool.
              </span>{" "}
              Draw on a canvas or work in a 3D space as if you&rsquo;re playing
              a video game.
            </p>
          </div>
        </article>

        {/* 2 — Desktop app */}
        <article data-reveal className="flex flex-col bg-bg md:col-span-3">
          <CellRow label="Desktop" index="02" />
          <div className="flex flex-1 flex-col p-6 md:p-8">
            <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
              Desktop app
            </h3>
            <p className="mt-2 leading-relaxed text-muted">
              <span className="text-ink">No more hunting for the hundredth tab.</span>{" "}
              Works on Windows, Mac, and soon Linux and tablets. First class
              experience for real artists.
            </p>
            <div className="mt-8 flex flex-1 items-end justify-center gap-8 border-t border-line pt-8">
              {[
                [REASON_IMAGES.windows, "Windows"],
                [REASON_IMAGES.apple, "macOS"],
                [REASON_IMAGES.linux, "Linux"],
              ].map(([src, alt]) => (
                // eslint-disable-next-line @next/next/no-img-element
                <img
                  key={alt}
                  src={src}
                  alt={alt}
                  loading="lazy"
                  draggable={false}
                  className="h-16 w-auto opacity-80 grayscale md:h-20"
                />
              ))}
            </div>
          </div>
        </article>

        {/* 3 — Open source */}
        <article data-reveal className="flex flex-col bg-bg md:col-span-2">
          <CellRow label="Open source" index="03" />
          <div className="flex flex-1 flex-col p-6 md:p-8">
            <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
              It&rsquo;s open source
            </h3>
            <p className="mt-2 leading-relaxed text-muted">
              Our desktop app&rsquo;s code and infrastructure are all{" "}
              <a
                href={SOCIAL_LINKS.GITHUB}
                target="_blank"
                rel="noopener noreferrer"
                className="text-ink underline underline-offset-4 decoration-line-strong hover:decoration-current"
              >
                open source on GitHub
              </a>
              . Join us and contribute!
            </p>
            <div className="mt-8 flex flex-1 items-center justify-center border-t border-line pt-8">
              <GitHubIcon className="h-20 w-20 text-ink md:h-24 md:w-24" />
            </div>
          </div>
        </article>

        {/* 4 — Use every model */}
        <article data-reveal className="flex flex-col overflow-hidden bg-bg md:col-span-4">
          <CellRow label="Models" index="04" />
          <div className="p-6 md:p-8">
            <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
              Use every model
            </h3>
            <p className="mt-2 leading-relaxed text-muted">
              You&rsquo;ll be able to use{" "}
              <span className="text-ink">every image and video model</span> all
              in one place. Log in with your existing subscriptions.
            </p>
          </div>
          <ModelMarquee />
        </article>

        {/* 5 — Created by artists */}
        <article data-reveal className="flex flex-col bg-bg md:col-span-6 md:flex-row md:items-center">
          <div className="flex-1">
            <CellRow label="Makers" index="05" />
            <div className="p-6 md:p-8">
              <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
                Created by artists and filmmakers
              </h3>
              <p className="mt-2 max-w-2xl leading-relaxed text-muted">
                <span className="text-ink">
                  The other leading platforms were created by the Google ad
                  team, crypto bros, and other non-artists.
                </span>{" "}
                Not us. We&rsquo;re one of you.
              </p>
            </div>
          </div>
          <div className="flex items-center gap-px border-t border-line bg-line md:border-t-0 md:border-l">
            {[FilmIcon, PaintbrushIcon, CameraIcon].map((Icon, i) => (
              <span
                key={i}
                className="flex h-20 flex-1 items-center justify-center bg-bg text-ink md:h-full md:w-24 md:flex-none"
              >
                <Icon aria-hidden className="h-6 w-6" />
              </span>
            ))}
          </div>
        </article>
      </div>
    </CampaignSection>
  );
}

function CellRow({ label, index }: { label: string; index: string }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-8">
      <p className="hud-label text-muted">{label}</p>
      <p className="hud-label text-faint">{index}</p>
    </div>
  );
}

// Three mono marquee rows of model badges, round-robin so brands mix.
// Reuses the landing ticker's .marquee keyframes.
function ModelMarquee() {
  const rows: (typeof MODEL_BADGES)[] = [[], [], []];
  MODEL_BADGES.forEach((badge, i) => rows[i % 3].push(badge));
  return (
    <div className="flex flex-col gap-px border-t border-line bg-line">
      {rows.map((row, r) => (
        <div
          key={r}
          className="marquee bg-bg"
          style={{ animationDirection: r % 2 ? "reverse" : "normal" }}
        >
          {[0, 1].map((copy) => (
            <ul
              key={copy}
              aria-hidden={copy === 1 || undefined}
              className="marquee-track items-center"
              style={{
                animationDuration: `${[56, 68, 62][r]}s`,
                animationDirection: r % 2 ? "reverse" : "normal",
              }}
            >
              {row.map((badge) => (
                <li
                  key={badge.name}
                  className="hud-label flex items-center gap-2 whitespace-nowrap px-5 py-3 text-muted"
                >
                  {/* eslint-disable-next-line @next/next/no-img-element */}
                  <img
                    src={badge.icon}
                    alt=""
                    loading="lazy"
                    className="themed-logo h-3.5 w-3.5"
                  />
                  {badge.name}
                </li>
              ))}
            </ul>
          ))}
        </div>
      ))}
    </div>
  );
}
