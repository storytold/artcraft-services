"use client";

import { useEffect, useState } from "react";
import { DOWNLOAD_LINKS, DOWNLOAD_VERSIONS } from "./download-links";

export type Platform = "windows" | "macos";

export type DetectedPlatform = {
  platform: Platform;
  mobile: boolean;
  /** "Mac" | "Windows" for CTA labels. */
  short: string;
  downloadUrl: string;
  version: string;
};

// Platform detection runs after mount (there is no user agent on the
// server), so the first paint is the neutral, complete page and the
// "your system" affordances arrive with hydration. Ported from the Vite
// site's react-device-detect usage; null until hydrated.
export function useDetectedPlatform(): DetectedPlatform | null {
  const [state, setState] = useState<DetectedPlatform | null>(null);
  useEffect(() => {
    const ua = navigator.userAgent;
    const mobile = /Android|iPhone|iPad|iPod|Mobile/i.test(ua);
    const mac = /Macintosh|Mac OS X/i.test(ua);
    setState(
      mac
        ? {
            platform: "macos",
            mobile,
            short: "Mac",
            downloadUrl: DOWNLOAD_LINKS.MACOS,
            version: DOWNLOAD_VERSIONS.MACOS,
          }
        : {
            platform: "windows",
            mobile,
            short: "Windows",
            downloadUrl: DOWNLOAD_LINKS.WINDOWS,
            version: DOWNLOAD_VERSIONS.WINDOWS,
          },
    );
  }, []);
  return state;
}

const DOWNLOAD_INITIATED_KEY = "artcraft_download_initiated";

// Flag read by the webapp's onboarding so it knows the visitor came in
// via a desktop download.
export function markDownloadInitiated(): void {
  try {
    localStorage.setItem(DOWNLOAD_INITIATED_KEY, "true");
  } catch {
    // Storage unavailable; nothing depends on the flag.
  }
}
