import type { ReactNode } from "react";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";

// Campaign page hero: the interior PageHeader frame, centered, with a CTA
// slot and an optional embedded reel beneath (Vimeo iframe or any figure).
export default function CampaignHero({
  id,
  label,
  annotation,
  title,
  lede,
  children,
  media,
  mediaCaption,
}: {
  id: string;
  label: string;
  annotation?: string;
  title: ReactNode;
  lede: ReactNode;
  children?: ReactNode;
  media?: ReactNode;
  mediaCaption?: string;
}) {
  return (
    <SectionShell id={id}>
      <SectionEyebrow index="01" label={label} annotation={annotation} />
      <div data-reveal-group className="flex flex-col items-center px-6 py-14 text-center md:px-10 md:py-20">
        <h1
          data-reveal
          className="max-w-4xl font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl md:text-6xl lg:text-7xl"
        >
          {title}
        </h1>
        <p data-reveal className="mt-6 max-w-xl text-lg leading-relaxed text-muted">
          {lede}
        </p>
        {children && (
          <div data-reveal className="mt-8 w-full">
            {children}
          </div>
        )}
      </div>
      {media && (
        <figure className="border-t border-line">
          <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2 md:px-10">
            <figcaption className="hud-label text-faint">{mediaCaption}</figcaption>
            <p className="hud-label text-faint">Reel</p>
          </div>
          <div className="relative aspect-video w-full overflow-hidden bg-bg-sunken">
            {media}
            <span aria-hidden className="tick top-2 left-2 opacity-60" />
            <span aria-hidden className="tick top-2 right-2 opacity-60" />
            <span aria-hidden className="tick bottom-2 left-2 opacity-60" />
            <span aria-hidden className="tick bottom-2 right-2 opacity-60" />
          </div>
        </figure>
      )}
    </SectionShell>
  );
}

export function VimeoEmbed({ src, title }: { src: string; title: string }) {
  return (
    <iframe
      src={src}
      className="absolute inset-0 h-full w-full"
      allow="autoplay; fullscreen; picture-in-picture"
      allowFullScreen
      title={title}
    />
  );
}
