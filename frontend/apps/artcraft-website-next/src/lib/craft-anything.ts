import { CRAFT_ANYTHING_RELEASE } from "./crafting-app-releases";
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
  return CRAFT_ANYTHING_RELEASE
    ? releaseDownloads(CRAFT_ANYTHING_REPO, "craft-anything", CRAFT_ANYTHING_RELEASE)
    : [];
}

export function craftAnythingReleasePageUrl(): string {
  return CRAFT_ANYTHING_RELEASE
    ? `${CRAFT_ANYTHING_REPO}/releases/tag/${craftReleaseTag(CRAFT_ANYTHING_RELEASE)}`
    : `${CRAFT_ANYTHING_REPO}/releases`;
}
