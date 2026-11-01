import { createHash } from "node:crypto";
import { CRAFTING_APPS, craftAppRelease } from "./crafting-apps";
import { CRAFT_LAUNCHER_RELEASE } from "./crafting-app-releases";

export type FlatpakOciImage = {
  Tags: string[];
  Digest: string;
  MediaType: string;
  Size: number;
  Architecture: "x86_64" | "aarch64";
  OS: "linux";
  Labels: Record<string, string>;
  Annotations?: Record<string, string>;
};
export type FlatpakOciRepository = { Name: string; Repository: string; Images: FlatpakOciImage[] };
export type FlatpakOciIndexResponse = { Registry: string; Results: FlatpakOciRepository[] };
export const DEFAULT_OCI_REGISTRY = "https://ghcr.io/v2";
export const ARCHITECTURES: Array<"x86_64" | "aarch64"> = ["x86_64", "aarch64"];
const MANIFEST_TYPE = "application/vnd.oci.image.manifest.v1+json";
const MAX_DOCUMENT_BYTES = 2 * 1024 * 1024;

function normalizeArch(arch: string | null): string | null {
  if (!arch) return null;
  const value = arch.toLowerCase();
  if (["amd64", "x86_64", "x64"].includes(value)) return "x86_64";
  if (["arm64", "aarch64"].includes(value)) return "aarch64";
  return value;
}

async function boundedBytes(response: Response): Promise<Buffer> {
  const advertised = Number(response.headers.get("content-length"));
  if (advertised > MAX_DOCUMENT_BYTES) throw new Error("OCI document exceeds size limit");
  if (!response.body) throw new Error("OCI document has no body");
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > MAX_DOCUMENT_BYTES) throw new Error("OCI document exceeds size limit");
      chunks.push(value);
    }
  } finally {
    await reader.cancel();
  }
  return Buffer.concat(chunks, size);
}

function digest(bytes: Buffer): string {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

type CatalogItem = { repoName: string; appId: string; version: string };
function catalogItems(): CatalogItem[] {
  const items: CatalogItem[] = [];
  if (CRAFT_LAUNCHER_RELEASE) {
    items.push({ repoName: "storytold/craft-launcher", appId: "ai.storyteller.artcraft-launcher", version: CRAFT_LAUNCHER_RELEASE.version });
  }
  for (const app of CRAFTING_APPS) {
    const release = craftAppRelease(app);
    if (release) items.push({ repoName: `storytold/${app.slug}`, appId: `ai.storyteller.${app.slug}`, version: release.version });
  }
  return items;
}

/** Read published GHCR metadata; absent or private repositories are not advertised. */
async function fetchImage(item: CatalogItem, arch: "x86_64" | "aarch64", fetcher: typeof fetch): Promise<FlatpakOciImage | null> {
  const scope = encodeURIComponent(`repository:${item.repoName}:pull`);
  const tokenResponse = await fetcher(`https://ghcr.io/token?service=ghcr.io&scope=${scope}`, { signal: AbortSignal.timeout(10_000) });
  if ([401, 403, 404].includes(tokenResponse.status)) return null;
  if (!tokenResponse.ok) throw new Error(`GHCR token request failed for ${item.repoName}: ${tokenResponse.status}`);
  const tokenData = JSON.parse((await boundedBytes(tokenResponse)).toString("utf8")) as { token?: unknown };
  if (typeof tokenData.token !== "string" || !tokenData.token) throw new Error(`Invalid GHCR token for ${item.repoName}`);
  const headers = { Authorization: `Bearer ${tokenData.token}`, Accept: MANIFEST_TYPE };
  const tag = `${item.version}-${arch}`;
  const manifestResponse = await fetcher(`${DEFAULT_OCI_REGISTRY}/${item.repoName}/manifests/${tag}`, { headers, signal: AbortSignal.timeout(10_000), redirect: "error" });
  if ([401, 403, 404].includes(manifestResponse.status)) return null;
  if (!manifestResponse.ok) throw new Error(`GHCR manifest request failed for ${item.repoName}:${tag}: ${manifestResponse.status}`);
  const manifestBytes = await boundedBytes(manifestResponse);
  const manifestDigest = digest(manifestBytes);
  const reported = manifestResponse.headers.get("docker-content-digest");
  if (reported && reported !== manifestDigest) throw new Error(`GHCR manifest digest mismatch for ${item.repoName}:${tag}`);
  const manifest = JSON.parse(manifestBytes.toString("utf8")) as { mediaType?: unknown; config?: { digest?: unknown; size?: unknown }; annotations?: Record<string, string> };
  if (manifest.mediaType !== MANIFEST_TYPE || typeof manifest.config?.digest !== "string" || !/^sha256:[0-9a-f]{64}$/.test(manifest.config.digest)) {
    throw new Error(`Invalid OCI manifest for ${item.repoName}:${tag}`);
  }
  const configResponse = await fetcher(`${DEFAULT_OCI_REGISTRY}/${item.repoName}/blobs/${manifest.config.digest}`, { headers, signal: AbortSignal.timeout(10_000), redirect: "follow" });
  if (!configResponse.ok) throw new Error(`GHCR configuration request failed for ${item.repoName}:${tag}: ${configResponse.status}`);
  const configBytes = await boundedBytes(configResponse);
  if (digest(configBytes) !== manifest.config.digest || (typeof manifest.config.size === "number" && configBytes.length !== manifest.config.size)) {
    throw new Error(`GHCR configuration digest mismatch for ${item.repoName}:${tag}`);
  }
  const config = JSON.parse(configBytes.toString("utf8")) as { os?: string; architecture?: string; config?: { Labels?: Record<string, string> } };
  const labels = config.config?.Labels ?? {};
  if (config.os !== "linux" || normalizeArch(config.architecture ?? null) !== arch || labels["org.flatpak.ref"] !== `app/${item.appId}/${arch}/stable`) {
    throw new Error(`Invalid Flatpak image metadata for ${item.repoName}:${tag}`);
  }
  return { Tags: [tag], Digest: manifestDigest, MediaType: MANIFEST_TYPE, Size: manifestBytes.length, Architecture: arch, OS: "linux", Labels: labels, Annotations: manifest.annotations };
}

/** Build the index only from verified registry manifests and image configurations. */
export async function getFlatpakOciIndex(searchParams?: URLSearchParams, fetcher: typeof fetch = fetch): Promise<FlatpakOciIndexResponse> {
  const registry = process.env.FLATPAK_OCI_REGISTRY_URL ?? DEFAULT_OCI_REGISTRY;
  if (registry !== DEFAULT_OCI_REGISTRY) throw new Error("Only GHCR OCI metadata is supported");
  const requestedRepo = searchParams?.get("repository") ?? searchParams?.get("repo");
  const requestedArch = normalizeArch(searchParams?.get("architecture") ?? null);
  const requestedRef = searchParams?.get("label:org.flatpak.ref");
  const requestedTag = searchParams?.get("tag");
  if (searchParams?.get("os") && searchParams.get("os") !== "linux") return { Registry: registry, Results: [] };
  const results = await Promise.all(catalogItems().filter((item) => !requestedRepo || requestedRepo === item.repoName).map(async (item) => {
    const images = await Promise.all(ARCHITECTURES.filter((arch) => !requestedArch || arch === requestedArch).map((arch) => fetchImage(item, arch, fetcher)));
    const matching = images.filter((image): image is FlatpakOciImage => image !== null)
      .filter((image) => (!requestedRef || image.Labels["org.flatpak.ref"] === requestedRef) && (!requestedTag || image.Tags.includes(requestedTag)));
    return matching.length ? { Name: item.repoName, Repository: item.repoName, Images: matching } : null;
  }));
  return { Registry: registry, Results: results.filter((entry): entry is FlatpakOciRepository => entry !== null) };
}
