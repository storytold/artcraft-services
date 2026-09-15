import type { ReactNode } from "react";
import { ChevronDownIcon, InfoIcon } from "lucide-react";
import { GitHubIcon, DiscordIcon } from "@/components/icons";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Accent } from "@/components/page/page-header";
import { Button } from "@/components/ui";
import type { QA, Tip } from "@/lib/campaign-data";
import { SOCIAL_LINKS } from "@/lib/links";

// Building blocks shared by the campaign landing pages, all in the
// landing's section language (rails, eyebrow, display heading with one
// serif word, hairline cell grids). Server components: no state.

// Section heading block: eyebrow row + big heading + optional lede.
export function CampaignSection({
  id,
  index,
  label,
  annotation,
  title,
  lede,
  children,
}: {
  id?: string;
  index: string;
  label: string;
  annotation?: string;
  title: ReactNode;
  lede?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <SectionShell id={id}>
      <SectionEyebrow index={index} label={label} annotation={annotation} />
      <div className="px-6 py-14 md:px-10 md:py-20">
        <h2
          data-reveal
          className="max-w-3xl font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl md:text-6xl"
        >
          {title}
        </h2>
        {lede && (
          <p data-reveal className="mt-5 max-w-xl text-lg leading-relaxed text-muted">
            {lede}
          </p>
        )}
      </div>
      {children}
    </SectionShell>
  );
}

// Long-form paragraphs (overview copy) with an optional flagged note.
export function Prose({
  paragraphs,
  note,
}: {
  paragraphs: string[];
  note?: string;
}) {
  return (
    <div className="border-t border-line">
      <div className="mx-auto max-w-3xl px-6 py-12 md:px-10 md:py-16">
        <div className="flex flex-col gap-5 text-lg leading-relaxed text-muted">
          {paragraphs.map((p) => (
            <p key={p.slice(0, 40)}>{p}</p>
          ))}
        </div>
        {note && (
          <p className="mt-10 flex items-start gap-3 border border-line bg-bg-raised px-5 py-4 text-sm leading-relaxed text-muted">
            <InfoIcon aria-hidden className="mt-0.5 h-4 w-4 shrink-0 text-faint" />
            {note}
          </p>
        )}
      </div>
    </div>
  );
}

// Numbered tip / highlight cells.
export function TipGrid({
  items,
  columns = 2,
  icons,
}: {
  items: Tip[];
  columns?: 2 | 3;
  icons?: ReactNode[];
}) {
  return (
    <ol
      data-reveal-group
      className={`grid gap-px border-t border-line bg-line ${
        columns === 3 ? "md:grid-cols-3" : "md:grid-cols-2"
      }`}
    >
      {items.map((tip, i) => (
        <li key={tip.title} data-reveal className="bg-bg p-6 md:p-8">
          <div className="flex items-center justify-between gap-4">
            {icons?.[i] ? (
              <span className="flex h-10 w-10 items-center justify-center border border-line text-accent-ink">
                {icons[i]}
              </span>
            ) : (
              <span />
            )}
            <p className="hud-label text-faint">{String(i + 1).padStart(2, "0")}</p>
          </div>
          <h3 className="mt-6 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
            {tip.title}
          </h3>
          <p className="mt-2 leading-relaxed text-muted">{tip.body}</p>
        </li>
      ))}
      {items.length % columns !== 0 && (
        <li aria-hidden className="hidden bg-bg md:block" />
      )}
    </ol>
  );
}

// Native <details> accordion: complete without JS, hard edges, mono index.
export function FaqAccordion({ items }: { items: QA[] }) {
  return (
    <div className="flex flex-col gap-px border-t border-line bg-line">
      {items.map((item, i) => (
        <details key={item.question} className="group bg-bg">
          <summary className="flex cursor-pointer list-none items-center gap-4 px-6 py-5 md:px-10 [&::-webkit-details-marker]:hidden">
            <span className="hud-label w-8 shrink-0 text-faint">
              {String(i + 1).padStart(2, "0")}
            </span>
            <span className="flex-1 font-display text-xl font-medium tracking-[-0.02em] text-ink-strong">
              {item.question}
            </span>
            <ChevronDownIcon
              aria-hidden
              className="h-4 w-4 shrink-0 text-faint transition-transform group-open:rotate-180"
            />
          </summary>
          <p className="max-w-3xl px-6 pb-6 pl-[4.5rem] leading-relaxed text-muted md:px-10 md:pl-[5.5rem]">
            {item.answer}
          </p>
        </details>
      ))}
    </div>
  );
}

// Discord + GitHub strip.
export function CommunityCta({ id = "community" }: { id?: string }) {
  return (
    <SectionShell id={id}>
      <div className="flex flex-col gap-6 px-6 py-10 md:flex-row md:items-center md:justify-between md:px-10">
        <div>
          <p className="hud-label text-faint">Community</p>
          <h2 className="mt-3 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong sm:text-3xl">
            Join our <Accent>community</Accent>.
          </h2>
          <p className="mt-2 max-w-md leading-relaxed text-muted">
            ArtCraft is open source and community-driven. Come build with us.
          </p>
        </div>
        <div className="flex flex-wrap gap-3">
          <Button href={SOCIAL_LINKS.DISCORD} target="_blank" rel="noopener noreferrer">
            <DiscordIcon className="h-4 w-4" />
            Join Discord
          </Button>
          <Button
            href={SOCIAL_LINKS.GITHUB}
            variant="secondary"
            target="_blank"
            rel="noopener noreferrer"
          >
            <GitHubIcon className="h-4 w-4" />
            Star on GitHub
          </Button>
        </div>
      </div>
    </SectionShell>
  );
}

// Centered closing pitch with the page's CTA slot underneath.
export function ClosingCta({
  id = "start",
  eyebrow,
  title,
  lede,
  footnote,
  children,
}: {
  id?: string;
  eyebrow: string;
  title: ReactNode;
  lede: ReactNode;
  footnote?: string;
  children: ReactNode;
}) {
  return (
    <SectionShell id={id}>
      <div className="flex flex-col items-center px-6 py-20 text-center md:py-28">
        <p className="hud-label text-faint">{eyebrow}</p>
        <h2
          data-reveal
          className="mt-6 max-w-4xl font-display text-5xl font-medium leading-[0.98] tracking-[-0.04em] text-ink-strong sm:text-6xl md:text-7xl"
        >
          {title}
        </h2>
        <p className="mt-6 max-w-xl text-lg leading-relaxed text-muted">{lede}</p>
        <div className="mt-10">{children}</div>
        {footnote && <p className="hud-label mt-8 text-faint">{footnote}</p>}
      </div>
    </SectionShell>
  );
}
