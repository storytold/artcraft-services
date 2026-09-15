"use client";

import { useEffect, useState, type ReactNode } from "react";
import {
  ArrowDownToLineIcon,
  ArrowRightIcon,
  FilesIcon,
  MemoryStickIcon,
  MonitorIcon,
  AppleIcon,
} from "lucide-react";
import { WindowsIcon } from "@/components/icons";
import { SectionShell, SectionEyebrow } from "@/components/landing/section-shell";
import { Accent, PageHeader } from "@/components/page/page-header";
import { Button, Modal } from "@/components/ui";
import {
  DOWNLOAD_LINKS,
  DOWNLOAD_VERSIONS,
  DOWNLOADS_ENABLED,
} from "@/lib/download-links";
import { mediaUrl, webappUrl } from "@/lib/links";

type Platform = "windows" | "macos";

type System = {
  id: Platform;
  os: string;
  short: string;
  icon: ReactNode;
  link: string;
  version: string;
  requirements: { icon: ReactNode; text: string }[];
};

const SYSTEMS: System[] = [
  {
    id: "windows",
    os: "Windows",
    short: "Windows",
    icon: <WindowsIcon className="h-5 w-5" />,
    link: DOWNLOAD_LINKS.WINDOWS,
    version: DOWNLOAD_VERSIONS.WINDOWS,
    requirements: [
      { icon: <MonitorIcon aria-hidden className="h-4 w-4" />, text: "Windows 10 (64-bit) or newer" },
      { icon: <MemoryStickIcon aria-hidden className="h-4 w-4" />, text: "8 GB RAM recommended" },
      { icon: <FilesIcon aria-hidden className="h-4 w-4" />, text: "2 GB available storage" },
    ],
  },
  {
    id: "macos",
    os: "macOS",
    short: "Mac",
    icon: <AppleIcon aria-hidden className="h-5 w-5" />,
    link: DOWNLOAD_LINKS.MACOS,
    version: DOWNLOAD_VERSIONS.MACOS,
    requirements: [
      { icon: <MonitorIcon aria-hidden className="h-4 w-4" />, text: "macOS 12.0 or newer" },
      { icon: <MemoryStickIcon aria-hidden className="h-4 w-4" />, text: "8 GB RAM recommended" },
      { icon: <FilesIcon aria-hidden className="h-4 w-4" />, text: "2 GB available storage" },
    ],
  },
];

const STEPS = [
  { title: "Download", description: "Grab the installer for your platform." },
  { title: "Install & sign in", description: "Create a free account or log in." },
  { title: "Create", description: "Start generating artwork immediately." },
];

const DOWNLOAD_INITIATED_KEY = "artcraft_download_initiated";

// Platform detection runs after mount (there is no user agent on the
// server), so the first paint is the neutral, complete page and the
// "your system" affordances arrive with hydration. Ported from the Vite
// site's react-device-detect usage.
function useDetectedPlatform() {
  const [state, setState] = useState<{ platform: Platform; mobile: boolean } | null>(null);
  useEffect(() => {
    const ua = navigator.userAgent;
    const mobile = /Android|iPhone|iPad|iPod|Mobile/i.test(ua);
    const mac = /Macintosh|Mac OS X/i.test(ua);
    setState({ platform: mac ? "macos" : "windows", mobile });
  }, []);
  return state;
}

export default function DownloadPage() {
  const detected = useDetectedPlatform();
  const [modalOpen, setModalOpen] = useState(false);

  const detectedSystem =
    SYSTEMS.find((s) => s.id === detected?.platform) ?? SYSTEMS[0];
  const isMobile = detected?.mobile ?? false;

  // Downloads start natively via the anchor; the modal only invites the
  // visitor to create an account while the installer arrives.
  const onDownloadClick = () => {
    setModalOpen(true);
    try {
      localStorage.setItem(DOWNLOAD_INITIATED_KEY, "true");
    } catch {
      // Storage unavailable — nothing depends on the flag.
    }
  };

  return (
    <>
      <PageHeader
        index="01"
        label="Download"
        annotation="macOS · Windows · Web"
        title={
          <>
            Download <Accent>ArtCraft</Accent>.
          </>
        }
        lede="AI-powered artwork creation with canvas editing and 3D scene composition — right on your desktop."
      >
        <div data-reveal className="mt-8 flex flex-wrap items-center gap-3">
          <PrimaryDownloadButton
            system={detectedSystem}
            isMobile={isMobile}
            onClick={onDownloadClick}
          />
          <Button href={webappUrl("/")} variant="secondary" size="lg">
            Use on web
          </Button>
        </div>
        <p data-reveal className="hud-label mt-5 text-faint">
          Free · Open source · v{detectedSystem.version}
        </p>
      </PageHeader>

      <SectionShell>
        <figure>
          <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2 md:px-10">
            <figcaption className="hud-label text-faint">
              Viewport — 3D scene editor
            </figcaption>
            <p className="hud-label text-faint">Desktop app</p>
          </div>
          <div className="relative w-full overflow-hidden bg-bg-sunken">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={mediaUrl("/images/3d-interface-preview.jpg")}
              alt="ArtCraft interface preview"
              className="block h-auto w-full"
            />
            <span aria-hidden className="tick top-2 left-2 opacity-60" />
            <span aria-hidden className="tick top-2 right-2 opacity-60" />
            <span aria-hidden className="tick bottom-2 left-2 opacity-60" />
            <span aria-hidden className="tick bottom-2 right-2 opacity-60" />
          </div>
        </figure>
      </SectionShell>

      <SectionShell id="platforms">
        <SectionEyebrow index="02" label="Platforms" annotation="Universal macOS build · x64 Windows" />
        <div className="grid gap-px bg-line md:grid-cols-2">
          {SYSTEMS.map((system, i) => {
            const isDetected = detected?.platform === system.id && !isMobile;
            return (
              <article key={system.id} className="flex flex-col bg-bg">
                <div className="flex items-center justify-between gap-4 border-b border-line px-6 py-2.5 md:px-8">
                  <p className="hud-label text-muted">
                    {isDetected ? (
                      <span className="text-accent-ink">Your system</span>
                    ) : (
                      "Platform"
                    )}
                  </p>
                  <p className="hud-label text-faint">
                    {String(i + 1).padStart(2, "0")}
                  </p>
                </div>
                <div className="flex flex-1 flex-col p-6 md:p-8">
                  <div className="flex items-center gap-3">
                    <span className="flex h-10 w-10 items-center justify-center border border-line text-ink">
                      {system.icon}
                    </span>
                    <h2 className="font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
                      {system.os}
                    </h2>
                  </div>
                  <ul className="mt-6 flex flex-1 flex-col gap-2.5">
                    {system.requirements.map((req) => (
                      <li key={req.text} className="flex items-center gap-2.5 text-sm text-muted">
                        <span className="text-faint">{req.icon}</span>
                        {req.text}
                      </li>
                    ))}
                  </ul>
                  <div className="mt-8">
                    {isMobile ? (
                      <Button disabled className="w-full">
                        Desktop only
                      </Button>
                    ) : DOWNLOADS_ENABLED ? (
                      <Button
                        href={system.link}
                        onClick={onDownloadClick}
                        variant={isDetected ? "primary" : "secondary"}
                        className="w-full"
                      >
                        <ArrowDownToLineIcon aria-hidden className="h-4 w-4" />
                        Download for {system.short}
                      </Button>
                    ) : (
                      <Button disabled className="w-full">
                        Temporarily unavailable
                      </Button>
                    )}
                  </div>
                </div>
              </article>
            );
          })}
        </div>
      </SectionShell>

      <SectionShell id="quick-start">
        <SectionEyebrow index="03" label="Quick start" annotation="Minutes, not hours" />
        <ol className="grid gap-px bg-line md:grid-cols-3">
          {STEPS.map((step, i) => (
            <li key={step.title} className="bg-bg p-6 md:p-8">
              <p className="hud-label text-faint">Step {i + 1}</p>
              <h3 className="mt-6 font-display text-2xl font-medium tracking-[-0.02em] text-ink-strong">
                {step.title}
              </h3>
              <p className="mt-2 leading-relaxed text-muted">{step.description}</p>
            </li>
          ))}
        </ol>
      </SectionShell>

      <Modal
        isOpen={modalOpen}
        onClose={() => setModalOpen(false)}
        title="Downloading ArtCraft…"
      >
        <p className="leading-relaxed text-muted">
          Your download has started. While you wait, create an account to
          store your creations.
        </p>
        <Button href={webappUrl("/signup")} className="mt-6 w-full">
          Sign up
          <ArrowRightIcon aria-hidden className="h-3.5 w-3.5" />
        </Button>
        <a
          href={webappUrl("/login")}
          className="hud-label mt-4 block text-center text-faint hover:text-ink"
        >
          I already have an account
        </a>
      </Modal>
    </>
  );
}

function PrimaryDownloadButton({
  system,
  isMobile,
  onClick,
}: {
  system: System;
  isMobile: boolean;
  onClick: () => void;
}) {
  if (isMobile) {
    return (
      <Button size="lg" disabled>
        Download on a desktop
      </Button>
    );
  }
  if (!DOWNLOADS_ENABLED) {
    return (
      <Button size="lg" disabled>
        Downloads temporarily unavailable
      </Button>
    );
  }
  return (
    <Button href={system.link} size="lg" onClick={onClick}>
      {system.icon}
      Download for {system.short}
    </Button>
  );
}
