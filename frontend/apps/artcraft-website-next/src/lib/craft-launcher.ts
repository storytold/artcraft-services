import { CRAFT_LAUNCHER_RELEASE } from "./crafting-app-releases";
import {
  CRAFTING_APPS_GITHUB_ORG,
  craftReleaseTag,
  releaseDownloads,
  type CraftDownload,
} from "./crafting-apps";

// ArtCraft Launcher: one small native app that installs, updates and opens
// every Crafting App. The /apps hub's main button downloads it and every
// /apps/<slug> Get-it section recommends it above that app's own installers.
//
// Facts come from the craft-launcher README — keep them in sync. Version and
// files live in crafting-app-releases.ts (CRAFT_LAUNCHER_RELEASE); null there
// hides every launcher download.

export const CRAFT_LAUNCHER_NAME = "ArtCraft Launcher";
export const CRAFT_LAUNCHER_REPO = `${CRAFTING_APPS_GITHUB_ORG}/craft-launcher`;

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
