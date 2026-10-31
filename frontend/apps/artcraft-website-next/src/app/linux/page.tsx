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
          <div
            data-reveal
            className="mt-8 flex items-center justify-center xl:mt-0 xl:shrink-0"
          >
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={mediaUrl("/images/linux-logo.png")}
              alt="Tux, the Linux mascot"
              width={180}
              height={214}
              className="h-36 w-auto object-contain sm:h-44 md:h-52"
              loading="eager"
              draggable={false}
            />
          </div>
        }
      />

      <SectionShell id="instructions">
        <SectionEyebrow index="02" label="Instructions" annotation="One-line setup" />
        <LinuxRepoGuide />
      </SectionShell>
    </>
  );
}
