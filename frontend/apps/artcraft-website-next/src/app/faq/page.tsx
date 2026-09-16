import type { Metadata } from "next";
import Link from "next/link";
import { ArrowRightIcon } from "lucide-react";
import RevealManager from "@/components/reveal-manager";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Accent, PageHeader } from "@/components/page/page-header";
import { getFaqItems } from "@/lib/content";

export const metadata: Metadata = {
  title: "Frequently Asked Questions",
  description:
    "Explore ArtCraft FAQs: guides on AI image generation, editing, and workflows.",
  alternates: { canonical: "/faq" },
};

export default function FaqIndexPage() {
  const items = getFaqItems();

  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: items.map((item) => ({
      "@type": "Question",
      name: item.title,
      acceptedAnswer: { "@type": "Answer", text: item.description },
    })),
  };

  return (
    <>
      <RevealManager />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />

      <PageHeader
        id="faq"
        index="01"
        label="FAQ"
        annotation={`${items.length} guides`}
        title={
          <>
            Frequently asked <Accent>questions</Accent>.
          </>
        }
        lede="Deep-dive guides and answers about ArtCraft."
      />

      <SectionShell id="articles">
        <SectionEyebrow index="02" label="Guides" annotation="Read in order or jump around" />
        <ol data-reveal-group className="grid gap-px bg-line md:grid-cols-2">
          {items.map((item, i) => (
            <li key={item.slug} data-reveal className="bg-bg">
              <Link
                href={`/faq/${item.slug}`}
                className="group flex h-full flex-col p-6 hover:bg-bg-raised md:p-8"
              >
                <p className="hud-label text-faint">
                  {String(i + 1).padStart(2, "0")}
                </p>
                <h2 className="mt-6 flex items-start gap-2 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
                  <span className="flex-1">{item.title}</span>
                  <ArrowRightIcon
                    aria-hidden
                    className="mt-1.5 h-4 w-4 shrink-0 text-faint transition-transform group-hover:translate-x-1 group-hover:text-ink"
                  />
                </h2>
                <p className="mt-2 leading-relaxed text-muted">
                  {item.description}
                </p>
              </Link>
            </li>
          ))}
          {items.length % 2 === 1 && (
            <li aria-hidden className="hidden bg-bg md:block" />
          )}
        </ol>
      </SectionShell>
    </>
  );
}
