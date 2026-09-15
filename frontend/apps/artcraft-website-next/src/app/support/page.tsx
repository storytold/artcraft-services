import type { Metadata } from "next";
import type { ReactNode } from "react";
import Link from "next/link";
import { ArrowRightIcon, CircleHelpIcon, MailIcon } from "lucide-react";
import { DiscordIcon } from "@/components/icons";
import RevealManager from "@/components/reveal-manager";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Accent, PageHeader } from "@/components/page/page-header";
import { SOCIAL_LINKS, SUPPORT_EMAIL } from "@/lib/links";

export const metadata: Metadata = {
  title: "Support",
  description:
    "Get help with ArtCraft: browse the FAQ, join our Discord community, or email us directly.",
  alternates: { canonical: "/support" },
};

type Channel = {
  index: string;
  icon: ReactNode;
  title: string;
  description: string;
  href: string;
  cta: string;
  external?: boolean;
};

const CHANNELS: Channel[] = [
  {
    index: "A",
    icon: <DiscordIcon className="h-5 w-5" />,
    title: "Join our Discord",
    description:
      "The fastest way to reach us. Quick replies from the team and credit refunds if something goes wrong.",
    href: SOCIAL_LINKS.DISCORD,
    cta: "Open Discord",
    external: true,
  },
  {
    index: "B",
    icon: <MailIcon aria-hidden className="h-5 w-5" />,
    title: "Email us",
    description: SUPPORT_EMAIL,
    href: `mailto:${SUPPORT_EMAIL}`,
    cta: "Write to us",
    external: true,
  },
  {
    index: "C",
    icon: <CircleHelpIcon aria-hidden className="h-5 w-5" />,
    title: "Browse the FAQ",
    description: "Answers to common questions about ArtCraft.",
    href: "/faq",
    cta: "Read the FAQ",
  },
];

export default function SupportPage() {
  return (
    <>
      <RevealManager />

      <PageHeader
        id="support"
        index="01"
        label="Support"
        annotation="We're happy to help"
        title={
          <>
            Get <Accent>help</Accent>.
          </>
        }
        lede="Find answers, join the community, or reach us directly."
      />

      <SectionShell id="contact">
        <SectionEyebrow index="02" label="Channels" annotation="Pick one" />
        <ul data-reveal-group className="grid gap-px bg-line md:grid-cols-3">
          {CHANNELS.map((channel) => (
            <li key={channel.index} data-reveal className="bg-bg">
              <ChannelLink channel={channel}>
                <div className="flex items-center justify-between gap-4">
                  <span className="flex h-10 w-10 items-center justify-center border border-line text-ink transition-colors group-hover:bg-invert-bg group-hover:text-invert-fg">
                    {channel.icon}
                  </span>
                  <p className="hud-label text-faint">{channel.index}</p>
                </div>
                <h2 className="mt-6 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
                  {channel.title}
                </h2>
                <p className="mt-2 leading-relaxed text-muted">
                  {channel.description}
                </p>
                <span className="hud-label mt-6 flex items-center gap-1.5 text-muted group-hover:text-ink">
                  {channel.cta}
                  <ArrowRightIcon
                    aria-hidden
                    className="h-3.5 w-3.5 transition-transform group-hover:translate-x-1"
                  />
                </span>
              </ChannelLink>
            </li>
          ))}
        </ul>
      </SectionShell>
    </>
  );
}

function ChannelLink({
  channel,
  children,
}: {
  channel: Channel;
  children: ReactNode;
}) {
  const className = "group flex h-full flex-col p-6 hover:bg-bg-raised md:p-8";
  if (!channel.external) {
    return (
      <Link href={channel.href} className={className}>
        {children}
      </Link>
    );
  }
  const isMailto = channel.href.startsWith("mailto:");
  return (
    <a
      href={channel.href}
      target={isMailto ? undefined : "_blank"}
      rel={isMailto ? undefined : "noopener noreferrer"}
      className={className}
    >
      {children}
    </a>
  );
}
