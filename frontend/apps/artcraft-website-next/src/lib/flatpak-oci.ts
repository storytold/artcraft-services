import { createHash } from "node:crypto";
import {
  CRAFTING_APPS,
  craftAppRelease,
} from "./crafting-apps";
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
  /** GOARCH value, per the registry index spec. Flatpak matches these against `amd64`/`arm64`. */
  Architecture: "amd64" | "arm64";
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

/** What the index publishes for one registry image: its manifest digest and config labels. */
type ResolvedImage = { digest: string; labels: Record<string, string> };

const imageCache = new Map<string, { image: ResolvedImage; expiresAt: number }>();
const imageRequests = new Map<string, Promise<ResolvedImage>>();
const tokenCache = new Map<string, { token: string; expiresAt: number }>();
const tokenRequests = new Map<string, Promise<string>>();
const IMAGE_CACHE_MS = 5 * 60 * 1000;
const MANIFEST_MAX_BYTES = 2 * 1024 * 1024;
// Image configs carry the AppStream data and base64 icons, so allow more than a manifest.
const CONFIG_MAX_BYTES = 8 * 1024 * 1024;
const TOKEN_CACHE_MS = 5 * 60 * 1000;

/** Flatpak architecture names to the GOARCH names the registry index protocol uses. */
const OCI_ARCHITECTURES = { x86_64: "amd64", aarch64: "arm64" } as const;

/** Normalize architecture names to the GOARCH values used in index responses. */
function normalizeArch(arch: string | null): string | null {
  if (!arch) return null;
  const lower = arch.toLowerCase();
  if (lower === "amd64" || lower === "x86_64" || lower === "x64") return "amd64";
  if (lower === "arm64" || lower === "aarch64") return "arm64";
  return lower;
}

function buildImagesForTarget(appId: string, release: CraftRelease): FlatpakOciImage[] {
  const version = release.version;

  return ARCHITECTURES.map((arch) => {
    const ref = `app/${appId}/${arch}/stable`;
    const versionTag = `${version}-${arch}`;
    const stableTag = `stable-${arch}`;

    return {
      // flatpak-oci.sh publishes a version-architecture tag and stable-architecture tag.
      // These names must match the actual registry tags. Digest and Labels are filled from GHCR
      // below; the ref label here only selects images before the registry is queried.
      Tags: [versionTag, stableTag],
      Digest: "",
      MediaType: "application/vnd.oci.image.manifest.v1+json",
      Architecture: OCI_ARCHITECTURES[arch],
      OS: "linux",
      Labels: { "org.flatpak.ref": ref },
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
 * Resolve an image's manifest digest and config labels from the target registry. Never synthesize
 * either: Flatpak asks the registry for this exact content-addressed manifest, and resolves the
 * runtime, metadata and AppStream catalog from the labels. `flatpak build-bundle --oci` writes
 * those labels into the image config, so the index repeats them as published.
 *
 * GHCR packages must be public for end-user Flatpak installs. FLATPAK_OCI_READ_TOKEN and the
 * optional FLATPAK_OCI_READ_USERNAME are server-side fallbacks for
 * resolving the index while package visibility is being configured; they do not grant visitors
 * access to private images.
 */
async function fetchImage(repoName: string, tag: string, ref: string): Promise<ResolvedImage> {
  const registry = registryUrl();
  const key = `${registry.origin}/${repoName}:${tag}`;
  const cached = imageCache.get(key);
  if (cached && cached.expiresAt > Date.now()) return cached.image;
  const existingRequest = imageRequests.get(key);
  if (existingRequest) return existingRequest;

  const request = resolveImage(repoName, tag, ref, registry, key);
  imageRequests.set(key, request);
  try {
    return await request;
  } finally {
    imageRequests.delete(key);
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

function sha256Digest(body: ArrayBuffer): string {
  return `sha256:${createHash("sha256").update(Buffer.from(body)).digest("hex")}`;
}

async function readLimited(response: Response, limit: number, what: string): Promise<ArrayBuffer> {
  const body = await response.arrayBuffer();
  if (body.byteLength > limit) {
    throw new Error(`${what} exceeded the ${limit / (1024 * 1024)} MiB safety limit`);
  }
  return body;
}

async function resolveImage(
  repoName: string,
  tag: string,
  ref: string,
  registry: URL,
  key: string,
): Promise<ResolvedImage> {
  const token = await getRegistryPullToken(registry, repoName, tag);
  const headers = { Authorization: `Bearer ${token}` };

  const manifestUrl = new URL(`/v2/${repoName}/manifests/${encodeURIComponent(tag)}`, registry);
  let manifestResponse: Response;
  try {
    manifestResponse = await fetch(manifestUrl, {
      headers: { ...headers, Accept: MANIFEST_ACCEPT },
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
  // The digest of the exact response bytes is the manifest's content address.
  const manifestBody = await readLimited(manifestResponse, MANIFEST_MAX_BYTES, `OCI manifest ${repoName}:${tag}`);
  const digest = sha256Digest(manifestBody);
  const headerDigest = manifestResponse.headers.get("docker-content-digest");
  if (headerDigest && headerDigest !== digest) {
    throw new Error(`Registry returned a manifest for ${repoName}:${tag} that does not match its digest`);
  }

  const manifest: unknown = JSON.parse(Buffer.from(manifestBody).toString("utf8"));
  const configDigest = (manifest as { config?: { digest?: unknown } })?.config?.digest;
  if (typeof configDigest !== "string" || !/^sha256:[a-f0-9]{64}$/.test(configDigest)) {
    throw new Error(`OCI manifest ${repoName}:${tag} is not a single-image manifest with a config`);
  }

  // GHCR redirects blobs to signed storage; fetch drops Authorization on that cross-origin hop.
  const configResponse = await fetch(new URL(`/v2/${repoName}/blobs/${configDigest}`, registry), {
    headers,
    cache: "no-store",
  });
  if (!configResponse.ok) {
    throw new Error(`Could not read the OCI image config for ${repoName}:${tag} (HTTP ${configResponse.status})`);
  }
  const configBody = await readLimited(configResponse, CONFIG_MAX_BYTES, `OCI image config ${repoName}:${tag}`);
  if (sha256Digest(configBody) !== configDigest) {
    throw new Error(`The OCI image config for ${repoName}:${tag} does not match its digest`);
  }
  const config: unknown = JSON.parse(Buffer.from(configBody).toString("utf8"));
  const labels = (config as { config?: { Labels?: unknown } })?.config?.Labels;
  if (!labels || typeof labels !== "object" || Object.values(labels).some((value) => typeof value !== "string")) {
    throw new Error(`The OCI image config for ${repoName}:${tag} has no Flatpak labels`);
  }
  const imageLabels = labels as Record<string, string>;
  if (imageLabels["org.flatpak.ref"] !== ref) {
    throw new Error(
      `OCI image ${repoName}:${tag} is labelled ${imageLabels["org.flatpak.ref"] ?? "without a ref"}, not ${ref}`,
    );
  }

  const image = { digest, labels: imageLabels };
  imageCache.set(key, { image, expiresAt: Date.now() + IMAGE_CACHE_MS });
  return image;
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
    // Flatpak asks for `latest` when the remote URL has no `#tag` fragment, which is how
    // artcraft-oci.flatpakrepo is written. Every image here is the current release.
    if (criteria.tag && criteria.tag !== "latest" && !img.Tags.includes(criteria.tag) && refTag !== criteria.tag) {
      return false;
    }
    return true;
  });
}

function createRepoEntry(
  repoName: string,
  appId: string,
  release: CraftRelease,
  criteria: OciFilterCriteria,
): FlatpakOciRepository | null {
  const images = filterImages(buildImagesForTarget(appId, release), criteria);
  if (images.length === 0) return null;
  return { Name: repoName, Repository: repoName, Images: images };
}

interface FlatpakCatalogItem {
  repoName: string;
  appId: string;
  release: CraftRelease | null;
}

function getFlatpakCatalogItems(): FlatpakCatalogItem[] {
  const items: FlatpakCatalogItem[] = [];

  if (CRAFT_LAUNCHER_RELEASE) {
    items.push({
      // The shared release workflow overrides the script's local default with github.repository.
      repoName: "storytold/craft-launcher",
      appId: "ai.storyteller.artcraft-launcher",
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
    const entry = createRepoEntry(item.repoName, item.appId, item.release, criteria);
    if (!entry) return null;

    await Promise.all(entry.Images.map(async (image) => {
      const resolved = await fetchImage(item.repoName, image.Tags[0], image.Labels["org.flatpak.ref"]);
      image.Digest = resolved.digest;
      image.Labels = resolved.labels;
    }));
    return entry;
  }));

  return {
    Registry: registryUrl().origin,
    Results: entries.filter((entry): entry is FlatpakOciRepository => entry !== null),
  };
}
