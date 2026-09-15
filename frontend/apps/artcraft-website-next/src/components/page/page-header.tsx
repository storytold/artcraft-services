import type { ReactNode } from "react";
import Link from "next/link";
import { ChevronLeftIcon } from "lucide-react";
import { twMerge } from "tailwind-merge";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";

// Header block shared by every interior marketing page: the landing's
// section frame (rails, ticks, mono eyebrow) around a display headline with
// one italic serif contrast word, plus an optional lede. Pages compose their
// body sections beneath it with further SectionShells.
export function PageHeader({
  index,
  label,
  annotation,
  title,
  lede,
  children,
  align = "left",
  className,
}: {
  index: string;
  label: string;
  annotation?: string;
  title: ReactNode;
  lede?: ReactNode;
  /** Extra content under the lede (CTAs, back links, …). */
  children?: ReactNode;
  align?: "left" | "center";
  className?: string;
}) {
  const centered = align === "center";
  return (
    <SectionShell className={className}>
      <SectionEyebrow index={index} label={label} annotation={annotation} />
      <div
        className={twMerge(
          "px-6 py-14 md:px-10 md:py-20",
          centered && "flex flex-col items-center text-center",
        )}
      >
        <h1
          data-reveal
          className="max-w-3xl font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl md:text-6xl"
        >
          {title}
        </h1>
        {lede && (
          <p
            data-reveal
            className="mt-5 max-w-xl text-lg leading-relaxed text-muted"
          >
            {lede}
          </p>
        )}
        {children}
      </div>
    </SectionShell>
  );
}

// The headline's single serif-italic contrast word.
export function Accent({ children }: { children: ReactNode }) {
  return <span className="font-serif italic font-normal">{children}</span>;
}

// Mono "back to index" link used at the top of article pages.
export function BackLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <Link
      href={href}
      className="hud-label inline-flex items-center gap-1.5 text-muted hover:text-ink"
    >
      <ChevronLeftIcon aria-hidden className="h-3.5 w-3.5" />
      {children}
    </Link>
  );
}

// Rendered markdown body. Styled by `.article-content` in globals.css.
export function ArticleBody({ html }: { html: string }) {
  return (
    <article
      className="article-content mx-auto max-w-3xl px-6 py-12 md:px-10 md:py-16"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}

// Shared "not found" body for missing slugs.
export function NotFoundBody({
  what,
  backHref,
  backLabel,
}: {
  what: string;
  backHref: string;
  backLabel: string;
}) {
  return (
    <PageHeader
      index="00"
      label="Not found"
      title={
        <>
          Nothing <Accent>here</Accent>.
        </>
      }
      lede={`We couldn't find this ${what}.`}
    >
      <div className="mt-8">
        <BackLink href={backHref}>{backLabel}</BackLink>
      </div>
    </PageHeader>
  );
}
