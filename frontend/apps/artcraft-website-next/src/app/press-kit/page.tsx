import type { Metadata } from "next";
import { DiscordIcon } from "@/components/icons";
import RevealManager from "@/components/reveal-manager";
import { SectionShell } from "@/components/landing/section-shell";
import { Accent, PageHeader } from "@/components/page/page-header";
import PressKitGallery from "@/components/press-kit/press-kit-gallery";
import { Button } from "@/components/ui";
import { SOCIAL_LINKS } from "@/lib/links";
import { PRESS_CONTACT_PHONE } from "@/lib/press-kit-data";

export const metadata: Metadata = {
  title: "Press Kit",
  description:
    "Download ArtCraft press assets including logos, promotional videos, screenshots, and branding materials for media coverage.",
  alternates: { canonical: "/press-kit" },
};

export default function PressKitPage() {
  return (
    <>
      <RevealManager />

      <PageHeader
        index="01"
        label="Press kit"
        annotation="Logos · Videos · Screenshots"
        title={
          <>
            Press <Accent>kit</Accent>.
          </>
        }
        lede="Everything you need for press coverage, reviews, and content creation. Download high-quality assets and promotional materials about the world's only open source precision AI tool for artists."
      />

      <PressKitGallery />

      <SectionShell id="contact">
        <div className="flex flex-col items-center px-6 py-16 text-center md:py-24">
          <p className="hud-label text-faint">Need something specific?</p>
          <h2
            data-reveal
            className="mt-6 max-w-2xl font-display text-4xl font-medium leading-[1.02] tracking-[-0.035em] text-ink-strong sm:text-5xl"
          >
            Talk to a <Accent>human</Accent>.
          </h2>
          <p className="mt-5 max-w-lg leading-relaxed text-muted">
            For specific press inquiries, interview requests, or custom assets,
            reach out to us on Discord. Or text Brandon at{" "}
            <a
              href={`tel:${PRESS_CONTACT_PHONE.tel}`}
              className="whitespace-nowrap text-ink underline underline-offset-4 decoration-line-strong hover:decoration-current"
            >
              {PRESS_CONTACT_PHONE.display}
            </a>
            .
          </p>
          <Button
            href={SOCIAL_LINKS.DISCORD}
            target="_blank"
            rel="noopener noreferrer"
            size="lg"
            className="mt-8"
          >
            <DiscordIcon className="h-4 w-4" />
            Contact us on Discord
          </Button>
        </div>
      </SectionShell>
    </>
  );
}
