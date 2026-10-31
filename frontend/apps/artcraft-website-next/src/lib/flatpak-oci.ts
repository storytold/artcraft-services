import {
  CRAFTING_APPS,
  craftAppName,
  craftAppRelease,
  type CraftApp,
} from "./crafting-apps";
import { CRAFT_LAUNCHER_NAME } from "./craft-launcher";
import {
  CRAFT_LAUNCHER_RELEASE,
  type CraftRelease,
} from "./crafting-app-releases";

/**
 * Flatpak Registry Index Protocol response types (flatpak-oci-specs).
 * @see https://github.com/flatpak/flatpak-oci-specs
 */
export type FlatpakOciImage = {
  Tags: string[];
  Digest?: string;
  MediaType: string;
  Size?: number;
  Architecture: "x86_64" | "aarch64";
  OS: "linux";
  Labels: Record<string, string>;
  Annotations?: Record<string, string>;
};

export type FlatpakOciRepository = {
  Repository: string;
  Images: FlatpakOciImage[];
};

export type FlatpakOciIndexResponse = {
  Registry: string;
  Results: FlatpakOciRepository[];
};

export const DEFAULT_OCI_REGISTRY = "https://ghcr.io/v2";
export const ARCHITECTURES: Array<"x86_64" | "aarch64"> = ["x86_64", "aarch64"];

/** Normalize container architecture names to Flatpak conventions. */
function normalizeArch(arch: string | null): string | null {
  if (!arch) return null;
  const lower = arch.toLowerCase();
  if (lower === "amd64" || lower === "x86_64" || lower === "x64") return "x86_64";
  if (lower === "arm64" || lower === "aarch64") return "aarch64";
  return lower;
}

function buildImagesForTarget(
  appId: string,
  appName: string,
  release: CraftRelease,
): FlatpakOciImage[] {
  const version = release.version;
  const tags = ["stable", "latest", version];

  return ARCHITECTURES.map((arch) => ({
    Tags: tags,
    MediaType: "application/vnd.oci.image.manifest.v1+json",
    Architecture: arch,
    OS: "linux",
    Labels: {
      "org.flatpak.ref": `app/${appId}/${arch}/stable`,
      "org.opencontainers.image.version": version,
      "org.opencontainers.image.title": appName,
      "org.opencontainers.image.vendor": "Storytold",
    },
  }));
}

/**
 * Build the Flatpak OCI index catalog from the release catalog.
 * Supports filtering by architecture, ref, repo, and tag per flatpak-oci-specs.
 */
export function getFlatpakOciIndex(searchParams?: URLSearchParams): FlatpakOciIndexResponse {
  const reqArch = normalizeArch(searchParams?.get("architecture") ?? null);
  const reqRef = searchParams?.get("label:org.flatpak.ref");
  const refMustExist = searchParams?.get("label:org.flatpak.ref:exists") === "1";
  const reqRepo = searchParams?.get("repo");
  const reqTag = searchParams?.get("tag");

  const results: FlatpakOciRepository[] = [];

  // Craft Launcher
  if (CRAFT_LAUNCHER_RELEASE) {
    const launcherRepo = "storytold/artcraft-launcher";
    if (!reqRepo || reqRepo === launcherRepo) {
      let images = buildImagesForTarget(
        "ai.storyteller.artcraft-launcher",
        CRAFT_LAUNCHER_NAME,
        CRAFT_LAUNCHER_RELEASE,
      );

      if (reqArch) images = images.filter((img) => img.Architecture === reqArch);
      if (reqRef) images = images.filter((img) => img.Labels["org.flatpak.ref"] === reqRef);
      if (refMustExist) images = images.filter((img) => Boolean(img.Labels["org.flatpak.ref"]));
      if (reqTag) images = images.filter((img) => img.Tags.includes(reqTag));

      if (images.length > 0) {
        results.push({
          Repository: launcherRepo,
          Images: images,
        });
      }
    }
  }

  // Craft Apps
  for (const app of CRAFTING_APPS) {
    const release = craftAppRelease(app);
    if (!release) continue;

    const repoName = `storytold/${app.slug}`;
    if (reqRepo && reqRepo !== repoName) continue;

    const appId = `ai.storyteller.${app.slug}`;
    let images = buildImagesForTarget(appId, craftAppName(app), release);

    if (reqArch) images = images.filter((img) => img.Architecture === reqArch);
    if (reqRef) images = images.filter((img) => img.Labels["org.flatpak.ref"] === reqRef);
    if (refMustExist) images = images.filter((img) => Boolean(img.Labels["org.flatpak.ref"]));
    if (reqTag) images = images.filter((img) => img.Tags.includes(reqTag));

    if (images.length > 0) {
      results.push({
        Repository: repoName,
        Images: images,
      });
    }
  }

  return {
    Registry: process.env.FLATPAK_OCI_REGISTRY_URL ?? DEFAULT_OCI_REGISTRY,
    Results: results,
  };
}
