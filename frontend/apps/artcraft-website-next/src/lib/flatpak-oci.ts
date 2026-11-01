import { createHash } from "node:crypto";
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
  Digest: string;
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

export const DEFAULT_OCI_REGISTRY = "https://ghcr.io";
export const ARCHITECTURES: Array<"x86_64" | "aarch64"> = ["x86_64", "aarch64"];

const MANIFEST_ACCEPT = [
  "application/vnd.oci.image.manifest.v1+json",
  "application/vnd.oci.image.index.v1+json",
  "application/vnd.docker.distribution.manifest.v2+json",
  "application/vnd.docker.distribution.manifest.list.v2+json",
].join(", ");

const digestCache = new Map<string, { digest: string; expiresAt: number }>();
const digestRequests = new Map<string, Promise<string>>();
const tokenCache = new Map<string, { token: string; expiresAt: number }>();
const tokenRequests = new Map<string, Promise<string>>();
const DIGEST_CACHE_MS = 5 * 60 * 1000;
const TOKEN_CACHE_MS = 5 * 60 * 1000;

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
  const binaryName = appId.replace("ai.storyteller.", "");

  return ARCHITECTURES.map((arch) => {
    const ref = `app/${appId}/${arch}/stable`;
    const versionTag = `${version}-${arch}`;
    const stableTag = `stable-${arch}`;

    return {
      // flatpak-oci.sh publishes a version-architecture tag and stable-architecture tag.
      // These names must match the actual registry tags, and Digest is filled from GHCR below.
      Tags: [versionTag, stableTag],
      Digest: "",
      MediaType: "application/vnd.oci.image.manifest.v1+json",
      Architecture: arch,
      OS: "linux",
      Labels: {
        "org.flatpak.ref": ref,
        "org.flatpak.metadata": `[Application]\nname=${appId}\nruntime=org.freedesktop.Platform/${arch}/23.08\nsdk=org.freedesktop.Sdk/${arch}/23.08\ncommand=${binaryName}\n`,
        "org.opencontainers.image.version": version,
        "org.opencontainers.image.title": appName,
        "org.opencontainers.image.vendor": "Storytold",
      },
    };
  });
}

function registryUrl(): URL {
  const configured = process.env.FLATPAK_OCI_REGISTRY_URL ?? DEFAULT_OCI_REGISTRY;
  const url = new URL(configured);
  url.pathname = "";
  url.search = "";
  url.hash = "";
  return url;
}

/**
 * Resolve a real OCI manifest digest from the target registry. Never synthesize a digest: Flatpak
 * asks the registry for this exact content-addressed manifest before it can install the app.
 *
 * GHCR packages must be public for end-user Flatpak installs. FLATPAK_OCI_READ_TOKEN and the
 * optional FLATPAK_OCI_READ_USERNAME are server-side fallbacks for
 * resolving the index while package visibility is being configured; they do not grant visitors
 * access to private images.
 */
async function fetchManifestDigest(repoName: string, tag: string): Promise<string> {
  const registry = registryUrl();
  const key = `${registry.origin}/${repoName}:${tag}`;
  const cached = digestCache.get(key);
  if (cached && cached.expiresAt > Date.now()) return cached.digest;
  const existingRequest = digestRequests.get(key);
  if (existingRequest) return existingRequest;

  const request = resolveManifestDigest(repoName, tag, registry, key);
  digestRequests.set(key, request);
  try {
    return await request;
  } finally {
    digestRequests.delete(key);
  }
}

async function getRegistryPullToken(registry: URL, repoName: string, tag: string): Promise<string> {
  const key = `${registry.origin}/${repoName}`;
  const cached = tokenCache.get(key);
  if (cached && cached.expiresAt > Date.now()) return cached.token;
  const existingRequest = tokenRequests.get(key);
  if (existingRequest) return existingRequest;

  const request = (async () => {
    const tokenUrl = new URL("/token", registry);
    tokenUrl.searchParams.set("service", registry.host);
    tokenUrl.searchParams.set("scope", `repository:${repoName}:pull`);

    const tokenHeaders = new Headers();
    const readToken = process.env.FLATPAK_OCI_READ_TOKEN;
    if (readToken) {
      const username = process.env.FLATPAK_OCI_READ_USERNAME;
      if (!username) {
        throw new Error("FLATPAK_OCI_READ_USERNAME must be set when FLATPAK_OCI_READ_TOKEN is configured");
      }
      const credentials = `${username}:${readToken}`;
      const encodedCredentials = Buffer.from(credentials).toString("base64");
      tokenHeaders.set("Authorization", `Basic ${encodedCredentials}`);
    }

    let tokenResponse: Response;
    try {
      tokenResponse = await fetch(tokenUrl, { headers: tokenHeaders, cache: "no-store" });
    } catch (error) {
      throw new Error(`Could not request a pull token from ${registry.host}: ${String(error)}`);
    }
    if (!tokenResponse.ok) {
      throw new Error(
        `Could not obtain a pull token for ${repoName}:${tag} from ${registry.host} ` +
          `(HTTP ${tokenResponse.status}). Make the GHCR package public or configure ` +
          "FLATPAK_OCI_READ_TOKEN with packages:read access.",
      );
    }

    const tokenPayload: unknown = await tokenResponse.json();
    if (!tokenPayload || typeof tokenPayload !== "object") {
      throw new Error(`Registry returned an invalid pull token response for ${repoName}:${tag}`);
    }
    let tokenValue: string | null = null;
    if ("token" in tokenPayload && typeof tokenPayload.token === "string") {
      tokenValue = tokenPayload.token;
    } else if ("access_token" in tokenPayload && typeof tokenPayload.access_token === "string") {
      tokenValue = tokenPayload.access_token;
    }
    if (!tokenValue) {
      throw new Error(`Registry pull token response did not include a token for ${repoName}:${tag}`);
    }
    tokenCache.set(key, { token: tokenValue, expiresAt: Date.now() + TOKEN_CACHE_MS });
    return tokenValue;
  })();

  tokenRequests.set(key, request);
  try {
    return await request;
  } finally {
    tokenRequests.delete(key);
  }
}

async function resolveManifestDigest(repoName: string, tag: string, registry: URL, key: string): Promise<string> {
  const token = await getRegistryPullToken(registry, repoName, tag);

  const manifestUrl = new URL(`/v2/${repoName}/manifests/${encodeURIComponent(tag)}`, registry);
  let manifestResponse: Response;
  try {
    manifestResponse = await fetch(manifestUrl, {
      method: "HEAD",
      headers: { Accept: MANIFEST_ACCEPT, Authorization: `Bearer ${token}` },
      cache: "no-store",
    });
  } catch (error) {
    throw new Error(`Could not resolve ${repoName}:${tag} from ${registry.host}: ${String(error)}`);
  }
  if (!manifestResponse.ok) {
    const message = manifestResponse.status === 404
      ? "The OCI image tag is missing. Confirm the release workflow pushed it."
      : "The registry denied access. Make the GHCR package public for end-user Flatpak installs.";
    throw new Error(
      `Could not resolve OCI manifest ${repoName}:${tag} from ${registry.host} ` +
        `(HTTP ${manifestResponse.status}). ${message}`,
    );
  }

  let digest = manifestResponse.headers.get("docker-content-digest");
  if (!digest && manifestResponse.status === 200) {
    // Some registries omit Docker-Content-Digest on HEAD. Fetch the small manifest document and
    // calculate its OCI content digest from the exact response bytes.
    const getResponse = await fetch(manifestUrl, {
      headers: { Accept: MANIFEST_ACCEPT, Authorization: `Bearer ${token}` },
      cache: "no-store",
    });
    if (!getResponse.ok) {
      throw new Error(`Could not read OCI manifest ${repoName}:${tag} (HTTP ${getResponse.status})`);
    }
    const body = await getResponse.arrayBuffer();
    if (body.byteLength > 2 * 1024 * 1024) {
      throw new Error(`OCI manifest ${repoName}:${tag} exceeded the 2 MiB safety limit`);
    }
    digest = `sha256:${createHash("sha256").update(Buffer.from(body)).digest("hex")}`;
  }
  if (!digest || !/^sha256:[a-f0-9]{64}$/.test(digest)) {
    throw new Error(`Registry returned an invalid manifest digest for ${repoName}:${tag}`);
  }

  digestCache.set(key, { digest, expiresAt: Date.now() + DIGEST_CACHE_MS });
  return digest;
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
    // Flatpak asks for the ref's branch tag (usually `stable`), while per-architecture GHCR
    // images are physically published as e.g. `stable-x86_64`.
    const refTag = img.Labels["org.flatpak.ref"]?.split("/").at(-1);
    if (criteria.tag && !img.Tags.includes(criteria.tag) && refTag !== criteria.tag) return false;
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

interface FlatpakCatalogItem {
  repoName: string;
  appId: string;
  displayName: string;
  release: CraftRelease | null;
}

function getFlatpakCatalogItems(): FlatpakCatalogItem[] {
  const items: FlatpakCatalogItem[] = [];

  if (CRAFT_LAUNCHER_RELEASE) {
    items.push({
      // The shared release workflow overrides the script's local default with github.repository.
      repoName: "storytold/craft-launcher",
      appId: "ai.storyteller.artcraft-launcher",
      displayName: CRAFT_LAUNCHER_NAME,
      release: CRAFT_LAUNCHER_RELEASE,
    });
  }


  for (const app of CRAFTING_APPS) {
    if (app.slug === "craft-anything" || app.slug === "texturecraft") continue;
    const release = craftAppRelease(app);
    if (release) {
      items.push({
        repoName: `storytold/${app.slug}`,
        appId: `ai.storyteller.${app.slug}`,
        displayName: craftAppName(app),
        release,
      });
    }
  }

  return items;
}

/** Return the catalogued Flatpak app IDs without contacting the OCI registry. */
export function getFlatpakOciAppIds(): string[] {
  return getFlatpakCatalogItems()
    .filter((item) => item.release !== null)
    .map((item) => item.appId);
}

/**
 * Build the Flatpak OCI index catalog from the release catalog.
 * Supports filtering by architecture, ref, repo, and tag per flatpak-oci-specs.
 */
export async function getFlatpakOciIndex(searchParams?: URLSearchParams): Promise<FlatpakOciIndexResponse> {
  const criteria: OciFilterCriteria = {
    arch: normalizeArch(searchParams?.get("architecture") ?? null),
    ref: searchParams?.get("label:org.flatpak.ref") ?? null,
    refMustExist: searchParams?.get("label:org.flatpak.ref:exists") === "1",
    tag: searchParams?.get("tag") ?? null,
  };
  const reqRepo = searchParams?.get("repository") ?? searchParams?.get("repo");

  const items = getFlatpakCatalogItems();
  const selectedItems = items.filter((item) => !reqRepo || reqRepo === item.repoName);
  const entries = await Promise.all(selectedItems.map(async (item) => {
    if (!item.release) return null;
    const entry = createRepoEntry(
      item.repoName,
      item.appId,
      item.displayName,
      item.release,
      criteria,
    );
    if (!entry) return null;

    await Promise.all(entry.Images.map(async (image) => {
      const versionTag = image.Tags[0];
      image.Digest = await fetchManifestDigest(item.repoName, versionTag);
    }));
    return entry;
  }));

  return {
    Registry: registryUrl().origin,
    Results: entries.filter((entry): entry is FlatpakOciRepository => entry !== null),
  };
}
