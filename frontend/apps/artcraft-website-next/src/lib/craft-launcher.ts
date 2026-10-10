import type { Tip } from "./campaign-data";
import { CRAFT_LAUNCHER_RELEASE } from "./crafting-app-releases";
import {
  CRAFTING_APPS_GITHUB_ORG,
  craftReleaseTag,
  releaseDownloads,
  type CraftDownload,
} from "./crafting-apps";

// ArtCraft Launcher: one small native app that installs, updates and opens
// every Crafting App. The /apps hub leads with it and every /apps/<slug>
// Get-it section recommends it above that app's own installers.
//
// Facts come from the craft-launcher README — keep them in sync. Version and
// files live in crafting-app-releases.ts (CRAFT_LAUNCHER_RELEASE); null there
// hides every launcher download.

export const CRAFT_LAUNCHER_NAME = "ArtCraft Launcher";
export const CRAFT_LAUNCHER_STATUS = "Early alpha";
export const CRAFT_LAUNCHER_REPO = `${CRAFTING_APPS_GITHUB_ORG}/craft-launcher`;

/** The launcher section on the /apps hub, and its per-platform downloads. */
export const CRAFT_LAUNCHER_PATH = "/apps#launcher";
export const CRAFT_LAUNCHER_DOWNLOADS_ID = "launcher-downloads";

export const CRAFT_LAUNCHER_FEATURES: Tip[] = [
  {
    title: "Install in one click",
    body: "Picks the right build for your system and processor, checks it against the release's SHA-256 checksums and installs it.",
  },
  {
    title: "Stay up to date",
    body: "Watches every app's releases and updates the ones you have. A notification tells you when something new is out.",
  },
  {
    title: "Always at hand",
    body: "Open any Crafting App from one window, or from the menu bar and system tray. Native, with no Electron and no web view.",
  },
];

/** Every launcher release file, or none while the release is null. */
export function craftLauncherDownloads(): CraftDownload[] {
  return CRAFT_LAUNCHER_RELEASE
    ? releaseDownloads(CRAFT_LAUNCHER_REPO, "artcraft-launcher", CRAFT_LAUNCHER_RELEASE)
    : [];
}

export function craftLauncherReleasePageUrl(): string {
  return CRAFT_LAUNCHER_RELEASE
    ? `${CRAFT_LAUNCHER_REPO}/releases/tag/${craftReleaseTag(CRAFT_LAUNCHER_RELEASE)}`
    : `${CRAFT_LAUNCHER_REPO}/releases`;
}
