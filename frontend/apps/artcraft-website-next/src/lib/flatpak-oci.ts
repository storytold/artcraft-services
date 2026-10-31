import {
  CRAFTING_APPS,
  craftAppName,
  craftAppRelease,
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
  Name: string;
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

type OciFilterCriteria = {
  arch: string | null;
  ref: string | null;
  refMustExist: boolean;
  tag: string | null;
};

function filterImages(images: FlatpakOciImage[], criteria: OciFilterCriteria): FlatpakOciImage[] {
  return images.filter((img) => {
    if (criteria.arch && img.Architecture !== criteria.arch) return false;
    if (criteria.ref && img.Labels["org.flatpak.ref"] !== criteria.ref) return false;
    if (criteria.refMustExist && !img.Labels["org.flatpak.ref"]) return false;
    if (criteria.tag && !img.Tags.includes(criteria.tag)) return false;
    return true;
  });
}

function createRepoEntry(
  repoName: string,
  appId: string,
  appName: string,
  release: CraftRelease,
  criteria: OciFilterCriteria,
): FlatpakOciRepository | null {
  const images = filterImages(buildImagesForTarget(appId, appName, release), criteria);
  if (images.length === 0) return null;
  return { Name: repoName, Repository: repoName, Images: images };
}

/**
 * Build the Flatpak OCI index catalog from the release catalog.
 * Supports filtering by architecture, ref, repo, and tag per flatpak-oci-specs.
 */
export function getFlatpakOciIndex(searchParams?: URLSearchParams): FlatpakOciIndexResponse {
  const criteria: OciFilterCriteria = {
    arch: normalizeArch(searchParams?.get("architecture") ?? null),
    ref: searchParams?.get("label:org.flatpak.ref") ?? null,
    refMustExist: searchParams?.get("label:org.flatpak.ref:exists") === "1",
    tag: searchParams?.get("tag") ?? null,
  };
  const reqRepo = searchParams?.get("repository") ?? searchParams?.get("repo");

  const results: FlatpakOciRepository[] = [];

  // Craft Launcher
  if (CRAFT_LAUNCHER_RELEASE) {
    const launcherRepo = "storytold/artcraft-launcher";
    if (!reqRepo || reqRepo === launcherRepo) {
      const entry = createRepoEntry(
        launcherRepo,
        "ai.storyteller.artcraft-launcher",
        CRAFT_LAUNCHER_NAME,
        CRAFT_LAUNCHER_RELEASE,
        criteria,
      );
      if (entry) results.push(entry);
    }
  }

  // Craft Apps
  for (const app of CRAFTING_APPS) {
    const repoName = `storytold/${app.slug}`;
    if (reqRepo && reqRepo !== repoName) continue;

    const release = craftAppRelease(app);
    if (!release) continue;

    const entry = createRepoEntry(
      repoName,
      `ai.storyteller.${app.slug}`,
      craftAppName(app),
      release,
      criteria,
    );
    if (entry) results.push(entry);
  }

  return {
    Registry: process.env.FLATPAK_OCI_REGISTRY_URL ?? DEFAULT_OCI_REGISTRY,
    Results: results,
  };
}
