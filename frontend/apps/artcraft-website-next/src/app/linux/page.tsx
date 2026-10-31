import type { Metadata } from "next";
import RevealManager from "@/components/reveal-manager";
import { Accent, PageHeader } from "@/components/page/page-header";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import LinuxRepoGuide from "@/components/linux/linux-repo-guide";
import { mediaUrl } from "@/lib/links";

export const metadata: Metadata = {
  title: "Linux Package Repositories - APT, DNF, Zypper & Flatpak",
  description:
    "Official Linux package repository setup instructions for Debian, Ubuntu, Fedora, RHEL, openSUSE, and Flatpak. Install and continuously update Crafting apps natively.",
  alternates: { canonical: "/linux" },
};

export default function LinuxReposPage() {
  return (
    <>
      <RevealManager />

      <PageHeader
        id="linux"
        index="01"
        label="Linux Repositories"
        annotation="APT · DNF · Zypper · Flatpak OCI"
        title={
          <>
            Install on <Accent>Linux</Accent>.
          </>
        }
        lede="Native package repositories for Debian, Ubuntu, Fedora, RHEL, openSUSE, and Flatpak. Keep ArtCraft apps and the ArtCraft Launcher up to date directly through your distribution's package manager."
        aside={
          <aside
            data-reveal
            aria-label="Linux Mascot"
            className="mt-8 flex flex-col items-center justify-center border border-line bg-bg-raised p-6 sm:p-8 xl:mt-0 xl:w-72 xl:shrink-0"
          >
            <div className="relative flex items-center justify-center p-2">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={mediaUrl("/images/linux-logo.png")}
                alt="Tux, the Linux mascot"
                width={160}
                height={190}
                className="h-32 w-auto object-contain transition-transform duration-300 hover:scale-105 sm:h-40"
                loading="eager"
                draggable={false}
              />
              <span aria-hidden className="tick -top-1 -left-1 opacity-60" />
              <span aria-hidden className="tick -top-1 -right-1 opacity-60" />
              <span aria-hidden className="tick -bottom-1 -left-1 opacity-60" />
              <span aria-hidden className="tick -bottom-1 -right-1 opacity-60" />
            </div>
            <p className="hud-label mt-4 text-center text-faint">
              Native Package Repositories
            </p>
          </aside>
        }
      />

      <SectionShell id="instructions">
        <SectionEyebrow index="02" label="Instructions" annotation="One-line setup" />
        <LinuxRepoGuide />
      </SectionShell>
    </>
  );
}
