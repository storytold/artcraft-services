import {
  CRAFT_LAUNCHER_ASSETS,
  fetchCraftRelease,
  type CraftRelease,
} from "./crafting-app-releases";
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
// Facts come from the craft-launcher README — keep them in sync. The version
// is the repo's latest GitHub release; its files are CRAFT_LAUNCHER_ASSETS in
// crafting-app-releases.ts, and null there hides every launcher download.

export const CRAFT_LAUNCHER_NAME = "ArtCraft Launcher";
export const CRAFT_LAUNCHER_REPO = `${CRAFTING_APPS_GITHUB_ORG}/craft-launcher`;

/** The launcher's latest GitHub release, or null while its downloads are hidden. */
export async function craftLauncherRelease(): Promise<CraftRelease | null> {
  return CRAFT_LAUNCHER_ASSETS ? fetchCraftRelease("craft-launcher", CRAFT_LAUNCHER_ASSETS) : null;
}

/** Every launcher release file, or none while the release is hidden. */
export async function craftLauncherDownloads(): Promise<CraftDownload[]> {
  const release = await craftLauncherRelease();
  return release ? releaseDownloads(CRAFT_LAUNCHER_REPO, "artcraft-launcher", release) : [];
}

export async function craftLauncherReleasePageUrl(): Promise<string> {
  const release = await craftLauncherRelease();
  return release
    ? `${CRAFT_LAUNCHER_REPO}/releases/tag/${craftReleaseTag(release)}`
    : `${CRAFT_LAUNCHER_REPO}/releases`;
}
