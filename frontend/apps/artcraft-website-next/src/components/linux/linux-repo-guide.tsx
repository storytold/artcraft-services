"use client";

import { useState } from "react";
import Link from "next/link";
import {
  CpuIcon,
  ExternalLinkIcon,
  FolderDownIcon,
  InfoIcon,
  PackageIcon,
  ShieldCheckIcon,
  TerminalIcon,
} from "lucide-react";
import { CopyButton } from "@/components/ui/copy-button";
import { TabSelector, type TabItem } from "@/components/ui/tab-selector";
import { LINUX_REPO_APPS } from "@/lib/linux-repos";

const REPO_TABS: TabItem[] = [
  { id: "flatpak", label: "Flatpak (OCI)" },
  { id: "apt", label: "Ubuntu / Debian" },
  { id: "dnf", label: "Fedora / RHEL" },
  { id: "zypper", label: "openSUSE" },
];

type DistroConfig = {
  id: string;
  name: string;
  distros: string;
  architectures: string[];
  setupCommand: string;
  setupDescription: string;
  altSetupCommand?: string;
  altSetupDescription?: string;
  rawFiles: { name: string; href: string; description: string }[];
};

const DISTRO_CONFIGS: Record<string, DistroConfig> = {
  flatpak: {
    id: "flatpak",
    name: "Flatpak (OCI Registry Remote)",
    distros: "Any Linux distribution with flatpak installed",
    architectures: ["x86_64", "aarch64"],
    setupDescription: "Add the ArtCraft OCI Flatpak remote:",
    setupCommand: `# Add ArtCraft OCI Flatpak repository
flatpak remote-add --user --if-not-exists artcraft-oci https://getartcraft.com/artcraft-oci.flatpakrepo

# Search and install Craft apps
flatpak remote-ls artcraft-oci
flatpak install --user artcraft-oci com.artcraft.Launcher com.artcraft.pdfcraft`,
    altSetupDescription: "Single-app installation via .flatpakref file:",
    altSetupCommand: `# Install pdfcraft directly via its flatpakref descriptor
flatpak install --user https://getartcraft.com/apps/pdfcraft/pdfcraft-oci.flatpakref`,
    rawFiles: [
      { name: "artcraft-oci.flatpakrepo", href: "/artcraft-oci.flatpakrepo", description: "Flatpak OCI remote descriptor" },
      { name: "OCI Index", href: "/index/static", description: "Flatpak OCI registry summary index" },
    ],
  },
  apt: {
    id: "apt",
    name: "APT (Debian, Ubuntu & Derivatives)",
    distros: "Ubuntu 22.04+, Debian 12+, Linux Mint, Pop!_OS, Elementary OS",
    architectures: ["amd64 (x86_64)", "arm64 (aarch64)", "riscv64 (64-bit RISC-V)"],
    setupDescription: "Modern DEB822 format (recommended for Ubuntu 22.04+ and Debian 12+):",
    setupCommand: `# Add the ArtCraft repository source
sudo curl -fsSL https://getartcraft.com/apt/artcraft.sources -o /etc/apt/sources.list.d/artcraft.sources

# Update package cache and install
sudo apt update
sudo apt install artcraft-launcher pdfcraft`,
    altSetupDescription: "Legacy one-line sources.list format:",
    altSetupCommand: `# Add the ArtCraft repository list
sudo curl -fsSL https://getartcraft.com/apt/artcraft.list -o /etc/apt/sources.list.d/artcraft.list

# Update package cache and install
sudo apt update
sudo apt install artcraft-launcher pdfcraft`,
    rawFiles: [
      { name: "artcraft.sources", href: "/apt/artcraft.sources", description: "DEB822 multi-arch source declaration" },
      { name: "artcraft.list", href: "/apt/artcraft.list", description: "Legacy single-line APT source configuration" },
      { name: "Release", href: "/apt/dists/stable/Release", description: "Repository index with SHA-256 integrity digests" },
    ],
  },
  dnf: {
    id: "dnf",
    name: "DNF & RPM (Fedora, RHEL & Derivatives)",
    distros: "Fedora 40+, RHEL 9+, CentOS Stream, Rocky Linux, AlmaLinux",
    architectures: ["x86_64", "aarch64", "riscv64"],
    setupDescription: "Add repository using dnf config-manager:",
    setupCommand: `# Add the ArtCraft RPM repository
sudo dnf config-manager addrepo --from-repofile=https://getartcraft.com/rpm/artcraft.repo

# Install apps
sudo dnf install artcraft-launcher pdfcraft`,
    altSetupDescription: "Direct repository file download:",
    altSetupCommand: `# Drop the repository configuration into /etc/yum.repos.d/
sudo curl -fsSL https://getartcraft.com/rpm/artcraft.repo -o /etc/yum.repos.d/artcraft.repo

# Install apps (works with baseurl=/rpm and baseurl=/rpm/$basearch)
sudo dnf install artcraft-launcher pdfcraft`,
    rawFiles: [
      { name: "artcraft.repo", href: "/rpm/artcraft.repo", description: "YUM/DNF repository configuration" },
      { name: "repomd.xml", href: "/rpm/repodata/repomd.xml", description: "RPM repository master metadata index" },
    ],
  },
  zypper: {
    id: "zypper",
    name: "Zypper & RPM (openSUSE)",
    distros: "openSUSE Tumbleweed, openSUSE Leap 15.6+",
    architectures: ["x86_64", "aarch64", "riscv64"],
    setupDescription: "Add repository using zypper:",
    setupCommand: `# Add the ArtCraft RPM repository
sudo zypper addrepo https://getartcraft.com/rpm artcraft

# Refresh repositories and install
sudo zypper refresh
sudo zypper install artcraft-launcher pdfcraft`,
    rawFiles: [
      { name: "artcraft.repo", href: "/rpm/artcraft.repo", description: "RPM repository configuration" },
    ],
  },
};

export function LinuxRepoGuide() {
  const [activeTab, setActiveTab] = useState<string>("flatpak");
  const config = DISTRO_CONFIGS[activeTab] ?? DISTRO_CONFIGS.flatpak;

  return (
    <div className="flex flex-col gap-12">
      {/* Distro selector */}
      <div className="flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <p className="hud-label text-muted">Select distribution / package manager</p>
          <span className="hud-label hidden text-faint sm:inline-block">Continuous updates via native packages</span>
        </div>
        <TabSelector
          tabs={REPO_TABS}
          activeTab={activeTab}
          onTabChange={setActiveTab}
          className="w-full sm:w-auto"
        />
      </div>

      {/* Main setup card */}
      <article className="border border-line bg-bg">
        <div className="flex flex-wrap items-center justify-between gap-4 border-b border-line px-6 py-4 md:px-8">
          <div>
            <h2 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
              {config.name}
            </h2>
            <p className="mt-1 text-sm text-muted">{config.distros}</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <span className="hud-label flex items-center gap-1.5 text-faint">
              <CpuIcon aria-hidden className="h-3.5 w-3.5 text-muted" />
              Supported CPU:
            </span>
            {config.architectures.map((arch) => (
              <span
                key={arch}
                className="border border-line bg-bg-raised px-2 py-0.5 font-mono text-[11px] font-medium text-ink"
              >
                {arch}
              </span>
            ))}
          </div>
        </div>

        <div className="flex flex-col gap-8 p-6 md:p-8">
          {/* Primary setup instructions */}
          <div>
            <p className="text-sm font-medium text-ink">{config.setupDescription}</p>
            <TerminalBox command={config.setupCommand} />
          </div>

          {/* Alternative setup instructions if present */}
          {config.altSetupCommand && (
            <div>
              <p className="text-sm font-medium text-ink">{config.altSetupDescription}</p>
              <TerminalBox command={config.altSetupCommand} />
            </div>
          )}

          {/* Raw repository descriptors & inspection links */}
          <div className="border-t border-line pt-6">
            <p className="hud-label text-muted">Repository descriptors & endpoints</p>
            <div className="mt-3 grid gap-3 sm:grid-cols-2 md:grid-cols-3">
              {config.rawFiles.map((file) => (
                <Link
                  key={file.name}
                  href={file.href}
                  target="_blank"
                  className="group flex flex-col justify-between border border-line bg-bg-sunken p-3.5 transition-colors hover:bg-bg-raised"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-xs font-semibold text-ink group-hover:text-accent-ink">
                      {file.name}
                    </span>
                    <ExternalLinkIcon aria-hidden className="h-3.5 w-3.5 text-faint group-hover:text-ink" />
                  </div>
                  <p className="mt-2 text-xs text-muted">{file.description}</p>
                </Link>
              ))}
            </div>
          </div>
        </div>
      </article>

      {/* Security & Zero-Bandwidth Architecture */}
      <section className="grid gap-px bg-line md:grid-cols-2">
        <div className="flex flex-col bg-bg p-6 md:p-8">
          <div className="flex items-center gap-2.5">
            <span className="flex h-8 w-8 items-center justify-center border border-line text-ink">
              <ShieldCheckIcon aria-hidden className="h-4 w-4 text-accent-ink" />
            </span>
            <h3 className="font-display text-xl font-medium tracking-[-0.02em] text-ink-strong">
              Cryptographic Integrity
            </h3>
          </div>
          <p className="mt-4 text-sm leading-relaxed text-muted">
            Package managers verify all downloaded `.deb` and `.rpm` files against exact SHA-256
            checksums published in the repository index. Downloads are streamed directly from official
            GitHub Releases via HTTP 302 redirects, ensuring full end-to-end binary verification.
          </p>
          <div className="mt-6 flex items-start gap-2.5 border border-line bg-bg-sunken p-3.5 text-xs text-muted">
            <InfoIcon aria-hidden className="mt-0.5 h-4 w-4 shrink-0 text-faint" />
            <span>
              During public repository preview, repositories use un-signed transport flags (
              <code className="font-mono text-ink">[trusted=yes]</code> for APT and{" "}
              <code className="font-mono text-ink">gpgcheck=0</code> for DNF). Official GPG signing key
              distribution will be integrated alongside release signing.
            </span>
          </div>
        </div>

        <div className="flex flex-col bg-bg p-6 md:p-8">
          <div className="flex items-center gap-2.5">
            <span className="flex h-8 w-8 items-center justify-center border border-line text-ink">
              <FolderDownIcon aria-hidden className="h-4 w-4 text-accent-ink" />
            </span>
            <h3 className="font-display text-xl font-medium tracking-[-0.02em] text-ink-strong">
              Standalone Binaries & Tarballs
            </h3>
          </div>
          <p className="mt-4 text-sm leading-relaxed text-muted">
            Prefer not to configure system package repositories? Every Crafting App also publishes standalone
            portable packages for all distributions and architectures without root privileges:
          </p>
          <ul className="mt-4 space-y-2 text-xs text-muted">
            <li className="flex items-center gap-2">
              <span className="h-1.5 w-1.5 rounded-full bg-accent-ink" />
              <span>
                <strong className="text-ink">AppImage:</strong> Universal executable packages for x86_64 and ARM64.
              </span>
            </li>
            <li className="flex items-center gap-2">
              <span className="h-1.5 w-1.5 rounded-full bg-accent-ink" />
              <span>
                <strong className="text-ink">Tarball (.tar.gz):</strong> Plain portable binaries for x86_64, aarch64, and riscv64.
              </span>
            </li>
          </ul>
          <div className="mt-auto pt-6">
            <Link
              href="/apps"
              className="hud-label inline-flex items-center gap-1.5 text-muted hover:text-ink"
            >
              Browse all standalone downloads on /apps →
            </Link>
          </div>
        </div>
      </section>

      {/* Available Packages Table */}
      <section className="flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
              Available Packages
            </h3>
            <p className="mt-1 text-sm text-muted">
              Packages currently served through our native APT and RPM repositories.
            </p>
          </div>
          <PackageIcon aria-hidden className="hidden h-6 w-6 text-faint sm:block" />
        </div>

        <div className="border border-line bg-bg">
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-left text-sm">
              <thead>
                <tr className="border-b border-line bg-bg-sunken text-xs font-medium text-faint">
                  <th className="px-6 py-3 font-mono uppercase tracking-wider">Package</th>
                  <th className="px-6 py-3 font-mono uppercase tracking-wider">Version</th>
                  <th className="px-6 py-3 font-mono uppercase tracking-wider">Architectures</th>
                  <th className="px-6 py-3 font-mono uppercase tracking-wider">Description</th>
                  <th className="px-6 py-3 text-right font-mono uppercase tracking-wider">Install Command</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-line">
                {LINUX_REPO_APPS.map((app) => (
                  <tr key={app.slug} className="hover:bg-bg-raised/50">
                    <td className="whitespace-nowrap px-6 py-4 font-mono font-semibold text-ink">
                      {app.name}
                    </td>
                    <td className="whitespace-nowrap px-6 py-4 font-mono text-xs text-muted">
                      v{app.version}
                    </td>
                    <td className="whitespace-nowrap px-6 py-4">
                      <div className="flex gap-1.5">
                        <span className="border border-line bg-bg-raised px-1.5 py-0.5 font-mono text-[10px] text-ink">
                          x86_64
                        </span>
                        <span className="border border-line bg-bg-raised px-1.5 py-0.5 font-mono text-[10px] text-ink">
                          aarch64
                        </span>
                        <span className="border border-line bg-bg-raised px-1.5 py-0.5 font-mono text-[10px] text-ink">
                          riscv64
                        </span>
                      </div>
                    </td>
                    <td className="max-w-md px-6 py-4 text-xs text-muted">
                      {app.description}
                    </td>
                    <td className="whitespace-nowrap px-6 py-4 text-right">
                      {(() => {
                        const flatpakId = app.name === "pdfcraft" ? "com.artcraft.pdfcraft" : "com.artcraft.Launcher";
                        const displayCmd =
                          activeTab === "flatpak"
                            ? `flatpak install artcraft-oci ${flatpakId}`
                            : activeTab === "zypper"
                              ? `zypper install ${app.name}`
                              : activeTab === "apt"
                                ? `apt install ${app.name}`
                                : `dnf install ${app.name}`;
                        const copyCmd =
                          activeTab === "flatpak"
                            ? `flatpak install --user artcraft-oci ${flatpakId}`
                            : activeTab === "zypper"
                              ? `sudo zypper install ${app.name}`
                              : activeTab === "apt"
                                ? `sudo apt install ${app.name}`
                                : `sudo dnf install ${app.name}`;
                        return (
                          <div className="inline-flex items-center gap-2">
                            <code className="border border-line bg-bg-sunken px-2.5 py-1 font-mono text-xs text-ink">
                              {displayCmd}
                            </code>
                            <CopyButton
                              value={copyCmd}
                              variant="ghost"
                              className="h-7 w-7 p-0"
                            />
                          </div>
                        );
                      })()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </section>
    </div>
  );
}

function TerminalBox({ command }: { command: string }) {
  return (
    <figure className="mt-3 border border-line bg-bg-sunken">
      <div className="flex items-center justify-between border-b border-line pl-4">
        <figcaption className="hud-label flex items-center gap-2 text-faint">
          <TerminalIcon aria-hidden className="h-3.5 w-3.5" />
          Terminal
        </figcaption>
        <CopyButton
          value={command}
          variant="ghost"
          className="h-9 border-l border-line px-3"
        />
      </div>
      <pre className="overflow-x-auto p-4 font-mono text-[13px] leading-relaxed text-ink">
        <code>{command}</code>
      </pre>
    </figure>
  );
}

export default LinuxRepoGuide;
