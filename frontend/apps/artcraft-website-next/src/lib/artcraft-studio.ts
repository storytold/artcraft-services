import { CRAFT_RELEASE_ASSETS, type CraftRelease } from "./crafting-app-releases";
import { releaseDownloads } from "./crafting-apps";

// Merge this release selection only after its signed Linux assets are published.
export const ARTCRAFT_STUDIO_RELEASE: CraftRelease = {
  version: "0.42.0",
  tag: "artcraft-v0.42.0",
  assets: CRAFT_RELEASE_ASSETS.filter((asset) => asset.group === "Linux" || asset.file === "SHA256SUMS.txt"),
};

export function artcraftStudioDownloads() {
  return releaseDownloads("https://github.com/storytold/artcraft", "artcraft", ARTCRAFT_STUDIO_RELEASE);
}
