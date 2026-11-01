import { CRAFT_APP_RELEASES } from "./crafting-app-releases";
import {
  CRAFTING_APPS_GITHUB_ORG,
  craftReleaseTag,
  releaseDownloads,
  type CraftDownload,
} from "./crafting-apps";

export const CRAFT_ANYTHING_NAME = "Craft Anything";
export const CRAFT_ANYTHING_REPO = `${CRAFTING_APPS_GITHUB_ORG}/craft-anything`;

/** Every craft-anything release file, or none while the release is null. */
export function craftAnythingDownloads(): CraftDownload[] {
  const release = CRAFT_APP_RELEASES["craft-anything"];
  return release
    ? releaseDownloads(CRAFT_ANYTHING_REPO, "craft-anything", release)
    : [];
}

export function craftAnythingReleasePageUrl(): string {
  const release = CRAFT_APP_RELEASES["craft-anything"];
  return release
    ? `${CRAFT_ANYTHING_REPO}/releases/tag/${craftReleaseTag(release)}`
    : `${CRAFT_ANYTHING_REPO}/releases`;
}
