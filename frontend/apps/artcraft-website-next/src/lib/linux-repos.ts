import { createHash } from "node:crypto";
import { gzipSync } from "node:zlib";
import { unstable_cache } from "next/cache";
import {
  CRAFTING_APPS,
  craftAppName,
  craftAppRelease,
  craftReleaseDownloads,
} from "./crafting-apps";
import {
  craftLauncherDownloads,
} from "./craft-launcher";
import {
  CRAFT_LAUNCHER_RELEASE,
} from "./crafting-app-releases";
import { isRepoSigningConfigured } from "./repo-signing";

export const RPM_REPO_TIMESTAMP = 1730000000;
export const PACMAN_REPO_TIMESTAMP = 1730000000;
export const APT_REPO_DATE = new Date("2026-10-31T17:30:00Z");

export const VALID_APT_DISTRIBUTIONS = new Set(["stable"]);
export const VALID_APT_COMPONENTS = new Set(["main"]);
export const VALID_APT_ARCHES = new Set(["amd64", "arm64", "riscv64"]);
export const VALID_RPM_ARCHES = new Set(["x86_64", "aarch64", "riscv64", "noarch"]);
export const VALID_PACMAN_ARCHES = new Set(["x86_64", "aarch64"]);

export type RpmRequire = {
  name: string;
  flags?: "GE" | "EQ" | "LE";
  epoch?: number;
  ver?: string;
  rel?: string;
};

export type DebPackageArch = "amd64" | "arm64" | "riscv64";
export type RpmPackageArch = "x86_64" | "aarch64" | "riscv64";
export type ArchPackageArch = "x86_64" | "aarch64";

export type LinuxPackageAssetMeta = {
  size: number;
  sha256: string;
  installedSize: number;
  depends?: string[];
  provides?: string[];
  optdepends?: string[];
};

export type LinuxRepoApp = {
  slug: string;
  name: string;
  version: string;
  description: string;
  section: string;
  debAssets: {
    arch: DebPackageArch;
    filename: string;
    size: number;
    sha256: string;
    depends: string;
    recommends?: string;
    installedSize: number;
  }[];
  rpmAssets: {
    arch: RpmPackageArch;
    filename: string;
    size: number;
    sha256: string;
    requires: RpmRequire[];
    installedSize: number;
    /** The release publishes a signature for this package, so dnf can verify it. */
    signed: boolean;
  }[];
  archAssets: {
    arch: ArchPackageArch;
    filename: string;
    size: number;
    sha256: string;
    installedSize: number;
    depends: string[];
    /** `provides` lines exactly as the package's .PKGINFO declares them. */
    provides: string[];
    /** `optdepend` lines exactly as the package's .PKGINFO declares them. */
    optdepends: string[];
    /** The release publishes a .sig for this package, so pacman can verify it. */
    signed: boolean;
  }[];
};

export type RepoFormatFilter = {
  deb?: boolean;
  rpm?: boolean;
  arch?: boolean;
};

export function escapeXml(unsafe: string): string {
  return unsafe
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

/**
 * Use only known deployment origins for generated repository configuration.
 */
export function getRequestBaseUrl(request: Request): string {
  const url = new URL(request.url);
  // A local reverse proxy (scripts/test-distro-repos.sh) forwards to the Next server, and Flatpak
  // has to be given the proxy's URL: it speaks HTTP/2 cleartext, which the Next server drops.
  const fwdHost = request.headers.get("x-forwarded-host");
  if (fwdHost && /^deploy-preview-\d+--artcraft-website-next\.netlify\.app(?::\d+)?$/.test(fwdHost)) {
    return `https://${fwdHost}`;
  }
  if (fwdHost && /^(?:localhost|127\.0\.0\.1)(?::\d+)?$/.test(fwdHost)) {
    const proto = request.headers.get("x-forwarded-proto") || (url.protocol ? url.protocol.replace(":", "") : "http");
    return `${proto}://${fwdHost}`;
  }

  // Check the request URL hostname (direct requests or Netlify edge URL)
  if (url.protocol === "https:" && /^deploy-preview-\d+--artcraft-website-next\.netlify\.app$/.test(url.hostname)) {
    return url.origin;
  }
  if ((url.hostname === "localhost" || url.hostname === "127.0.0.1") && url.protocol === "http:") {
    return url.origin;
  }

  return "https://getartcraft.com";
}

/**
 * Resolve package file download request strictly to catalogued GitHub Release binary asset.
 * Uncatalogued filenames return null to preserve clean 404 behavior and block unauthorized redirects.
 */
export function resolvePackageDownloadUrl(filename: string): string | null {
  if (CRAFT_LAUNCHER_RELEASE) {
    const launcherDownloads = craftLauncherDownloads();
    const match = launcherDownloads.find((d) => d.fileName === filename);
    if (match) return match.href;
  }

  for (const app of CRAFTING_APPS) {
    const release = craftAppRelease(app);
    if (!release) continue;
    const downloads = craftReleaseDownloads(app, release);
    const match = downloads.find((d) => d.fileName === filename);
    if (match) return match.href;
  }

  // Signature assets sit next to the package they cover and are not part of the download manifest,
  // so derive them from the package they sign instead of listing every signature separately.
  const signatureSuffix = PACKAGE_SIGNATURE_SUFFIXES.find((suffix) => filename.endsWith(suffix));
  if (signatureSuffix) {
    const signedUrl = resolvePackageDownloadUrl(filename.slice(0, -signatureSuffix.length));
    if (signedUrl) return `${signedUrl}${signatureSuffix}`;
  }

  return null;
}

/** Suffixes of the detached signatures release workflows publish beside their packages. */
const PACKAGE_SIGNATURE_SUFFIXES = [".sig", ".asc"] as const;

/**
 * Whether the release actually publishes `filename` as a detached signature. The redirect would
 * otherwise point clients at a release asset that does not exist, which pacman reports as a failed
 * signature download.
 */
export async function isPackageSignaturePublished(filename: string): Promise<boolean> {
  if (!PACKAGE_SIGNATURE_SUFFIXES.some((suffix) => filename.endsWith(suffix))) {
    return false;
  }

  const releases: Array<{ repo: string; version: string }> = [];
  if (CRAFT_LAUNCHER_RELEASE) {
    releases.push({ repo: "craft-launcher", version: CRAFT_LAUNCHER_RELEASE.version });
  }
  for (const app of CRAFTING_APPS) {
    const release = craftAppRelease(app);
    if (release) {
      releases.push({ repo: app.slug, version: release.version });
    }
  }

  for (const release of releases) {
    try {
      const sums = await getShaSums(release.repo, release.version);
      if (sums.has(filename)) {
        return true;
      }
    } catch {
      // A release whose checksums cannot be read publishes nothing verifiable.
    }
  }

  return false;
}

export function handlePackageDownloadRedirect(pathSegments: string[] | undefined): Response {
  const filename = pathSegments?.at(-1);
  if (!filename) {
    return new Response("Not found", { status: 404 });
  }

  const targetUrl = resolvePackageDownloadUrl(filename);
  if (!targetUrl) {
    return new Response(`Package asset "${filename}" not found in release catalog`, {
      status: 404,
      headers: { "Content-Type": "text/plain; charset=utf-8" },
    });
  }

  return Response.redirect(targetUrl, 302);
}

// In-memory cache for release checksums (repo:version -> sha256 Map)
const shaSumsCache = new Map<string, Promise<Map<string, string>>>();

// In-memory cache for package metadata (filename -> LinuxPackageAssetMeta)
const packageMetaCache = new Map<string, Promise<LinuxPackageAssetMeta>>();

async function getShaSums(repo: string, version: string): Promise<Map<string, string>> {
  const key = `${repo}:${version}`;
  let promise = shaSumsCache.get(key);
  if (!promise) {
    promise = (async () => {
      const url = `https://github.com/storytold/${repo}/releases/download/v${version}/SHA256SUMS.txt`;
      const res = await fetch(url, { signal: AbortSignal.timeout(10_000) });
      if (!res.ok) throw new Error(`Unable to fetch checksums for ${repo} ${version}: ${res.status}`);
      const text = await res.text();
      const map = new Map<string, string>();
      for (const line of text.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        const [sha, fname] = trimmed.split(/\s+/);
        if (sha && fname) map.set(fname.replace(/^\*/, ""), sha);
      }
      return map;
    })();
    shaSumsCache.set(key, promise);
    promise.catch(() => shaSumsCache.delete(key));
  }
  return promise;
}

/**
 * Resolves repository package section from app metadata.
 */
function getAppSection(schemaCategory: string | undefined, slug: string): string {
  if (schemaCategory === "BusinessApplication") {
    return "utils";
  }
  if (slug === "craft-anything") {
    return "devel";
  }
  return "graphics";
}

/**
 * Reads HTTP HEAD content-length for a package download URL.
 */
async function fetchHeadPackageSize(downloadUrl: string, filename: string): Promise<number> {
  const headRes = await fetch(downloadUrl, {
    method: "HEAD",
    redirect: "follow",
    signal: AbortSignal.timeout(10_000),
  });
  const size = Number(headRes.headers.get("content-length"));
  if (!headRes.ok || !Number.isSafeInteger(size) || size <= 0) {
    throw new Error(`Unable to determine package size for ${filename}`);
  }
  return size;
}

type ArchPkgInfo = {
  size: number;
  installedSize: number;
  depends: string[];
  provides: string[];
  optdepends: string[];
  /** True when .PKGINFO could not be read and these values are only estimates. */
  estimated?: boolean;
};

/**
 * Only used when a package ships no .PKGINFO at all; a guessed dependency pacman does not find in
 * the package is enough to make it refuse the install, so keep this to what the packages declare.
 * libxkbcommon-x11 is deliberately absent: every published package already declares it through
 * craft-common-actions' Arch builder, and adding a dependency pacman cannot match would make it
 * refuse the install.
 */
const DEFAULT_PACMAN_DEPENDS = [
  "glibc",
  "gcc-libs",
  "hicolor-icon-theme",
  "libxkbcommon",
  "vulkan-icd-loader",
];

/** Only used when a package's .PKGINFO could not be read; mirrors craft-common-actions' PKGBUILD. */
const DEFAULT_PACMAN_OPTDEPENDS = [
  "alsa-lib: sound support",
  "pipewire-alsa: audio playback and recording via PipeWire",
  "pulseaudio: audio playback and recording via PulseAudio",
  "wayland: native Wayland display support",
  "libx11: X11 and XWayland display support",
  "libxcursor: X11 cursor handling",
  "libxi: X11 input handling",
  "libxrandr: X11 display resolution handling",
  "libglvnd: GPU rendering through EGL",
  "mesa: GPU rendering through EGL",
  "vulkan-driver: GPU rendering through Vulkan",
  "xdg-desktop-portal: native desktop integration and file dialogs",
  "cups: printing support",
];

/**
 * Fast 128KB Range request to extract .PKGINFO fields from an Arch .pkg.tar.zst package.
 * The prefix is fed through the tar-aware scanner, so a .PKGINFO that straddles the 128KB mark
 * (or sits behind the binaries) yields nothing rather than half its fields.
 */
async function extractArchPkgInfo(downloadUrl: string): Promise<ArchPkgInfo> {
  const empty: ArchPkgInfo = { size: 0, installedSize: 0, depends: [], provides: [], optdepends: [] };
  try {
    const { Decompress } = await import("fzstd");
    const rangeRes = await fetch(downloadUrl, {
      headers: { Range: "bytes=0-131071" },
      signal: AbortSignal.timeout(10_000),
    });
    const contentRange = rangeRes.headers.get("content-range");
    const sizeMatch = contentRange?.match(/\/(\d+)$/);
    const size = sizeMatch ? Number.parseInt(sizeMatch[1], 10) : 0;

    const buf = new Uint8Array(await rangeRes.arrayBuffer());
    const scanner = createPkgInfoScanner();
    let fields: ArchPkgInfoFields | null = null;
    const d = new Decompress((chunk) => {
      if (!fields) {
        fields = scanner.push(Buffer.from(chunk));
      }
    });
    try {
      d.push(buf, false);
    } catch {
      // Partial stream EOF expected
    }

    const captured = fields as ArchPkgInfoFields | null;
    return captured ? { size, ...captured } : { ...empty, size };
  } catch {
    return empty;
  }
}

type ArchPkgInfoFields = {
  installedSize: number;
  depends: string[];
  provides: string[];
  optdepends: string[];
};

function parsePkgInfoText(text: string): ArchPkgInfoFields | null {
  const isizeMatch = text.match(/^size = (\d+)$/m);
  const installedSize = isizeMatch ? Number.parseInt(isizeMatch[1], 10) : 0;
  if (!installedSize) {
    return null;
  }
  const values = (key: string) => [...text.matchAll(new RegExp(`^${key} = (.+)$`, "gm"))].map((m) => m[1].trim());
  return {
    installedSize,
    depends: values("depend"),
    provides: values("provides"),
    optdepends: values("optdepend"),
  };
}

/**
 * Walks the tar archive inside a package to find .PKGINFO when it is not in the first 128KB.
 * Some packages put their binaries before .PKGINFO, so the Range probe cannot see it.
 * Exported so tests can feed a tar archive without a network round trip.
 */
export function createPkgInfoScanner() {
  let buffer: Buffer = Buffer.alloc(0);
  let mode: "header" | "data" | "pad" = "header";
  let remaining = 0;
  let padBytes = 0;
  let entrySize = 0;
  let capture: Buffer[] | null = null;

  function finishEntry(): ArchPkgInfoFields | null {
    mode = "header";
    if (!capture) {
      return null;
    }
    const text = Buffer.concat(capture).toString("utf-8");
    capture = null;
    return parsePkgInfoText(text);
  }

  return {
    push(chunk: Buffer): ArchPkgInfoFields | null {
      buffer = buffer.length ? Buffer.concat([buffer, chunk]) : chunk;

      for (;;) {
        if (mode === "pad") {
          const take = Math.min(padBytes, buffer.length);
          buffer = buffer.subarray(take);
          padBytes -= take;
          if (padBytes > 0) {
            return null;
          }
          mode = "header";
          continue;
        }

        if (mode === "data") {
          const take = Math.min(remaining, buffer.length);
          if (capture) {
            capture.push(buffer.subarray(0, take));
          }
          buffer = buffer.subarray(take);
          remaining -= take;
          if (remaining > 0) {
            return null;
          }
          padBytes = (512 - (entrySize % 512)) % 512;
          const finished = finishEntry();
          if (finished) {
            return finished;
          }
          mode = padBytes > 0 ? "pad" : "header";
          continue;
        }

        if (buffer.length < 512) {
          return null;
        }
        const header = buffer.subarray(0, 512);
        buffer = buffer.subarray(512);
        const name = header.toString("utf-8", 0, 100).replace(/\0[\s\S]*$/, "");
        if (!name) {
          return null; // end of archive
        }
        const sizeText = header.toString("utf-8", 124, 136).replace(/[\0 ]+$/, "").trim();
        entrySize = Number.parseInt(sizeText, 8) || 0;
        if (name.replace(/^\.\//, "") === ".PKGINFO") {
          capture = [];
        }
        remaining = entrySize;
        if (remaining === 0) {
          const finished = finishEntry();
          if (finished) {
            return finished;
          }
          padBytes = 0;
          continue;
        }
        mode = "data";
      }
    },
  };
}

/**
 * Reads .PKGINFO from a package tar stream, walking entries until it is found.
 */
export async function readArchPkgInfoFromStream(
  stream: ReadableStream<Uint8Array>,
): Promise<ArchPkgInfoFields | null> {
  const { Decompress } = await import("fzstd");
  const scanner = createPkgInfoScanner();
  const reader = stream.getReader();
  let result: ArchPkgInfoFields | null = null;

  const decoder = new Decompress((chunk) => {
    if (!result) {
      result = scanner.push(Buffer.from(chunk));
    }
  });

  try {
    while (!result) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      decoder.push(value, false);
    }
  } catch {
    // The download is cancelled once .PKGINFO has been read, which rejects the reader.
  } finally {
    await reader.cancel().catch(() => {});
    reader.releaseLock();
  }

  return result;
}

/**
 * Downloads a package until .PKGINFO is read. Only used when the Range probe cannot see it.
 */
async function downloadArchPkgInfo(downloadUrl: string): Promise<ArchPkgInfoFields | null> {
  try {
    const res = await fetch(downloadUrl, { signal: AbortSignal.timeout(60_000) });
    if (!res.ok || !res.body) {
      return null;
    }
    return await readArchPkgInfoFromStream(res.body);
  } catch {
    return null;
  }
}

/**
 * Computes size, installed uncompressed size and depends for an Arch Linux (.pkg.tar.zst) package asset.
 */
async function fetchArchPackageInfo(downloadUrl: string, filename: string): Promise<ArchPkgInfo> {
  const extracted = await extractArchPkgInfo(downloadUrl);
  const size = extracted.size || await fetchHeadPackageSize(downloadUrl, filename);

  if (extracted.installedSize) {
    return {
      size,
      installedSize: extracted.installedSize,
      depends: extracted.depends,
      provides: extracted.provides,
      optdepends: extracted.optdepends,
    };
  }

  const downloaded = await downloadArchPkgInfo(downloadUrl);
  if (downloaded) {
    return { size, ...downloaded };
  }

  // Last resort: pacman 7 compares these values against the package and refuses to install on a mismatch.
  console.warn(`Unable to read .PKGINFO for ${filename}; pacman will reject it if the estimated metadata is wrong`);
  return {
    size,
    installedSize: Math.round(size * 1.95),
    depends: DEFAULT_PACMAN_DEPENDS,
    provides: [],
    optdepends: [],
    estimated: true,
  };
}

/**
 * Fetches and validates the SHA-256 digest from SHA256SUMS.txt.
 */
async function fetchValidatedSha256(repo: string, version: string, filename: string): Promise<string> {
  const sumsMap = await getShaSums(repo, version);
  const sha256 = sumsMap.get(filename);
  if (!sha256 || !/^[a-fA-F0-9]{64}$/.test(sha256) || /^0+$/.test(sha256)) {
    throw new Error(`Missing or invalid checksum for ${filename}`);
  }
  return sha256;
}

/**
 * Reads metadata (size, SHA-256 digest, installed size) for a package asset straight from
 * GitHub: its SHA256SUMS.txt entry, plus a Range probe into the package for .PKGINFO.
 */
type ComputedPackageMetadata = LinuxPackageAssetMeta & { estimated?: boolean };

async function computePackageMetadata(
  filename: string,
  repo: string,
  version: string,
  knownSha256?: string,
): Promise<ComputedPackageMetadata> {
  const downloadUrl = resolvePackageDownloadUrl(filename);
  if (!downloadUrl) {
    throw new Error(`Package asset not found in release catalog: ${filename}`);
  }

  const sha256 = knownSha256 ?? await fetchValidatedSha256(repo, version, filename);

  if (filename.endsWith(".pkg.tar.zst")) {
    const archInfo = await fetchArchPackageInfo(downloadUrl, filename);
    return { sha256, ...archInfo };
  }

  const size = await fetchHeadPackageSize(downloadUrl, filename);
  const divisor = filename.endsWith(".deb") ? 1024 : 1;
  const installedSize = Math.round(size * 1.95 / divisor);

  return { size, sha256, installedSize };
}

/**
 * Estimated Arch metadata must never reach the shared cache: pacman rejects a package whose
 * database entry disagrees with it, and the estimate would outlive the GitHub outage that caused
 * it. Throwing keeps unstable_cache from storing the value while still letting the caller serve it.
 */
class EstimatedArchMetadataError extends Error {
  constructor(readonly metadata: ComputedPackageMetadata) {
    super("Not caching estimated Arch package metadata");
  }
}

/**
 * Keeps that metadata in Next's data cache, so it survives the process: a fresh instance (or a
 * new deploy) serves the DB without re-reading every package's .PKGINFO. Cache keys include the
 * filename and the SHA-256 digest from SHA256SUMS.txt, so a new release and a same-name re-upload
 * are both misses.
 */
const durablePackageMetadata = unstable_cache(
  async (filename: string, repo: string, version: string, sha256: string) => {
    const metadata = await computePackageMetadata(filename, repo, version, sha256);
    if (metadata.estimated) {
      throw new EstimatedArchMetadataError(metadata);
    }
    return metadata;
  },
  ["linux-package-metadata"],
  { revalidate: 60 * 60 * 24 },
);

/**
 * Dynamically computes metadata (size, SHA-256 digest, installed size) for a package asset.
 * Fetches SHA256SUMS.txt and HTTP metadata on demand and caches in memory and in the data cache.
 */
export async function getLinuxPackageMetadata(
  filename: string,
  repo: string,
  version: string,
): Promise<LinuxPackageAssetMeta> {
  const sha256 = await fetchValidatedSha256(repo, version, filename);
  // Include the digest so a same-name re-upload is a miss in the per-process cache too.
  const cacheKey = `${filename}:${sha256}`;
  let promise = packageMetaCache.get(cacheKey);
  if (!promise) {
    promise = (async () => {
      try {
        return await durablePackageMetadata(filename, repo, version, sha256);
      } catch (error) {
        if (error instanceof EstimatedArchMetadataError) {
          // Serve the estimate now, but leave it to the per-process cache rather than persisting it.
          return error.metadata;
        }
        // The data cache can be unavailable (no cache handler, disabled in preview); read directly.
        return computePackageMetadata(filename, repo, version, sha256);
      }
    })();
    packageMetaCache.set(cacheKey, promise);
    promise.catch(() => packageMetaCache.delete(cacheKey));
  }
  return promise;
}

const COMMON_RPM_REQUIRES: RpmRequire[] = [
  { name: "libxkbcommon.so.0()(64bit)" },
  { name: "libxkbcommon-x11.so.0()(64bit)" },
  { name: "libwayland-client.so.0()(64bit)" },
  { name: "libX11.so.6()(64bit)" },
  { name: "libX11-xcb.so.1()(64bit)" },
  { name: "libxcb.so.1()(64bit)" },
  { name: "libXcursor.so.1()(64bit)" },
  { name: "libXi.so.6()(64bit)" },
  { name: "(libvulkan.so.1()(64bit) or libEGL.so.1()(64bit))" },
];

const AUDIO_APPS = new Set(["filmcraft", "soundcraft", "effectcraft", "deckcraft"]);

interface DebArchMapItem {
  debArch: DebPackageArch;
  fileArch: RpmPackageArch;
}

async function buildDebAssets(
  name: string,
  repo: string,
  version: string,
): Promise<LinuxRepoApp["debAssets"]> {
  const debArchMap: DebArchMapItem[] = [
    { debArch: "amd64", fileArch: "x86_64" },
    { debArch: "arm64", fileArch: "aarch64" },
    { debArch: "riscv64", fileArch: "riscv64" },
  ];

  const sums = await getShaSums(repo, version);
  const candidates = debArchMap.filter(({ fileArch }) => sums.has(`${name}-${version}-linux-${fileArch}.deb`));

  const baseDepends = "libc6 (>= 2.35), libgcc-s1, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libx11-6, libx11-xcb1, libxcb1, libxcursor1, libxi6, libvulkan1 | libegl1";
  const defaultDepends = AUDIO_APPS.has(name)
    ? `${baseDepends}, libasound2 | libasound2t64`
    : baseDepends;
  const defaultRecommends = "mesa-vulkan-drivers, libegl-mesa0, xdg-desktop-portal";

  const results = await Promise.all(
    candidates.map(async ({ debArch, fileArch }) => {
      const filename = `${name}-${version}-linux-${fileArch}.deb`;
      const meta = await getLinuxPackageMetadata(filename, repo, version);
      return {
        arch: debArch,
        filename,
        size: meta.size,
        sha256: meta.sha256,
        depends: (name === "artcraft-launcher" && debArch === "riscv64")
          ? "libc6 (>= 2.39), libgcc-s1, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libx11-6, libx11-xcb1, libxcb1, libxcursor1, libxi6, libvulkan1 | libegl1"
          : defaultDepends,
        recommends: defaultRecommends,
        installedSize: meta.installedSize,
      };
    })
  );

  return results;
}

/**
 * Once the repository requires signatures, it must not advertise a package no client can verify:
 * dnf and pacman reject it, and a client that skips verification installs it unverified. Releases
 * list their detached signatures in SHA256SUMS.txt, so anything without one is withheld until the
 * next signed release instead of breaking the whole repository.
 */
export async function withholdUnsignedPackages<T extends { filename: string; signed: boolean }>(
  assets: T[],
): Promise<T[]> {
  if (!(await isRepoSigningConfigured())) {
    return assets;
  }
  return assets.filter((asset) => {
    if (asset.signed) return true;
    if (!reportedUnsignedPackages.has(asset.filename)) {
      reportedUnsignedPackages.add(asset.filename);
      console.warn(
        `Withholding ${asset.filename}: the repository requires signatures but the release publishes none for it`,
      );
    }
    return false;
  });
}

/** Each withheld package is worth reporting once; catalog builds repeat on every request. */
const reportedUnsignedPackages = new Set<string>();

async function buildRpmAssets(
  name: string,
  repo: string,
  version: string,
): Promise<LinuxRepoApp["rpmAssets"]> {
  const arches: RpmPackageArch[] = ["x86_64", "aarch64", "riscv64"];
  const sums = await getShaSums(repo, version);
  const candidates = arches.filter((arch) => sums.has(`${name}-${version}-linux-${arch}.rpm`));

  const results = await Promise.all(
    candidates.map(async (arch) => {
      const filename = `${name}-${version}-linux-${arch}.rpm`;
      const meta = await getLinuxPackageMetadata(filename, repo, version);
      const glibcVer = (name === "artcraft-launcher" && arch === "riscv64") ? "2.39" : "2.35";
      const audioRequires: RpmRequire[] = AUDIO_APPS.has(name)
        ? [{ name: "alsa-lib" }, { name: "libasound.so.2()(64bit)" }]
        : [];
      const requires: RpmRequire[] = [
        { name: "glibc", flags: "GE", epoch: 0, ver: glibcVer },
        ...COMMON_RPM_REQUIRES,
        ...audioRequires,
      ];
      return {
        arch,
        filename,
        size: meta.size,
        sha256: meta.sha256,
        requires,
        installedSize: meta.installedSize,
        // The release workflow signs each RPM and publishes the detached signature next to it.
        signed: sums.has(`${filename}.asc`),
      };
    })
  );

  return withholdUnsignedPackages(results);
}

async function buildArchAssets(
  name: string,
  repo: string,
  version: string,
): Promise<LinuxRepoApp["archAssets"]> {
  const sums = await getShaSums(repo, version);
  const candidates: ArchPackageArch[] = (["x86_64", "aarch64"] as ArchPackageArch[])
    .filter((arch) => sums.has(`${name}-${version}-linux-${arch}.pkg.tar.zst`));

  const results = await Promise.all(
    candidates.map(async (arch) => {
      const filename = `${name}-${version}-linux-${arch}.pkg.tar.zst`;
      const meta = await getLinuxPackageMetadata(filename, repo, version);
      return {
        arch,
        filename,
        size: meta.size,
        sha256: meta.sha256,
        installedSize: meta.installedSize,
        depends: meta.depends ?? DEFAULT_PACMAN_DEPENDS,
        provides: meta.provides ?? [],
        optdepends: meta.optdepends ?? [],
        // pacman verifies the detached `.pkg.tar.zst.sig` beside each package.
        signed: sums.has(`${filename}.sig`),
      };
    })
  );

  return withholdUnsignedPackages(results);
}
let cachedAllAppsPromise: Promise<LinuxRepoApp[]> | null = null;

/**
 * Dynamically builds the repository catalog informed by crafting-apps and crafting-app-releases.
 * Package metadata is fetched and computed on demand.
 */
export async function getLinuxRepoApps(formats?: RepoFormatFilter): Promise<LinuxRepoApp[]> {
  const includeDeb = formats ? !!formats.deb : true;
  const includeRpm = formats ? !!formats.rpm : true;
  const includeArch = formats ? !!formats.arch : true;

  const compute = async (): Promise<LinuxRepoApp[]> => {
    const appPromises: Promise<LinuxRepoApp | null>[] = [];

    // Craft Launcher
    if (CRAFT_LAUNCHER_RELEASE) {
      appPromises.push((async () => {
        const [debAssets, rpmAssets, archAssets] = await Promise.all([
          includeDeb ? buildDebAssets("artcraft-launcher", "craft-launcher", CRAFT_LAUNCHER_RELEASE.version) : Promise.resolve([]),
          includeRpm ? buildRpmAssets("artcraft-launcher", "craft-launcher", CRAFT_LAUNCHER_RELEASE.version) : Promise.resolve([]),
          includeArch ? buildArchAssets("artcraft-launcher", "craft-launcher", CRAFT_LAUNCHER_RELEASE.version) : Promise.resolve([]),
        ]);
        if (debAssets.length === 0 && rpmAssets.length === 0 && archAssets.length === 0) {
          return null;
        }
        return {
          slug: "artcraft-launcher",
          name: "artcraft-launcher",
          version: CRAFT_LAUNCHER_RELEASE.version,
          description: "ArtCraft Launcher: install, update and run native Crafting apps.",
          section: "utils",
          debAssets,
          rpmAssets,
          archAssets,
        };
      })());
    }

    // Crafting Apps
    for (const app of CRAFTING_APPS) {
      const release = craftAppRelease(app);
      if (!release) continue;

      appPromises.push((async () => {
        const [debAssets, rpmAssets, archAssets] = await Promise.all([
          includeDeb ? buildDebAssets(app.slug, app.slug, release.version) : Promise.resolve([]),
          includeRpm ? buildRpmAssets(app.slug, app.slug, release.version) : Promise.resolve([]),
          includeArch ? buildArchAssets(app.slug, app.slug, release.version) : Promise.resolve([]),
        ]);
        if (debAssets.length === 0 && rpmAssets.length === 0 && archAssets.length === 0) {
          return null;
        }
        return {
          slug: app.slug,
          name: app.slug,
          version: release.version,
          description: `${craftAppName(app)}: ${app.lede || app.pitch}`,
          section: getAppSection(app.schemaCategory, app.slug),
          debAssets,
          rpmAssets,
          archAssets,
        };
      })());
    }

    const settled = await Promise.all(appPromises);
    const apps: LinuxRepoApp[] = [];
    for (const app of settled) {
      if (app) apps.push(app);
    }
    return apps;
  };

  if (includeDeb && includeRpm && includeArch) {
    if (!cachedAllAppsPromise) {
      cachedAllAppsPromise = compute();
      cachedAllAppsPromise.catch(() => { cachedAllAppsPromise = null; });
    }
    return cachedAllAppsPromise;
  }

  return compute();
}

export async function handleAptPackagesRoute(
  distribution: string,
  component: string,
  archFolder: string,
  options?: { gzip?: boolean },
): Promise<Response> {
  if (
    !VALID_APT_DISTRIBUTIONS.has(distribution) ||
    !VALID_APT_COMPONENTS.has(component) ||
    !archFolder.startsWith("binary-") ||
    !VALID_APT_ARCHES.has(archFolder.slice(7))
  ) {
    return new Response("Not found", { status: 404 });
  }

  const arch = archFolder.slice(7);
  const content = await generateAptPackages(arch);

  if (options?.gzip) {
    const gzBuffer = gzipSync(Buffer.from(content, "utf-8"));
    return new Response(new Uint8Array(gzBuffer), {
      headers: {
        "Content-Type": "application/gzip",
        "Cache-Control": "public, max-age=300, s-maxage=3600, stale-while-revalidate=86400",
      },
    });
  }

  return new Response(content, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=300, s-maxage=3600, stale-while-revalidate=86400",
    },
  });
}

export async function generateAptPackages(arch: string): Promise<string> {
  const lines: string[] = [];
  const apps = await getLinuxRepoApps({ deb: true });

  for (const app of apps) {
    const asset = app.debAssets.find((a) => a.arch === arch);
    if (!asset) continue;

    const packageLines = [
      `Package: ${app.name}`,
      `Version: ${app.version}-1`,
      `Section: ${app.section}`,
      "Priority: optional",
      `Architecture: ${asset.arch}`,
      `Maintainer: ${app.name} maintainers <hello@storyteller.ai>`,
      `Installed-Size: ${asset.installedSize}`,
      `Depends: ${asset.depends}`,
      ...(asset.recommends ? [`Recommends: ${asset.recommends}`] : []),
      `Homepage: https://getartcraft.com/apps/${app.slug}`,
      `Filename: pool/main/${app.slug}/${asset.filename}`,
      `Size: ${asset.size}`,
      `SHA256: ${asset.sha256}`,
      `Description: ${app.description}`,
      "",
    ];

    lines.push(...packageLines);
  }

  return lines.join("\n");
}

export async function generateAptRelease(date: Date = APT_REPO_DATE): Promise<string> {
  const architectures = ["amd64", "arm64", "riscv64"];

  const results = await Promise.all(
    architectures.map(async (arch) => {
      const pkgContent = await generateAptPackages(arch);
      const pkgBuffer = Buffer.from(pkgContent, "utf-8");
      const pkgGzBuffer = gzipSync(pkgBuffer);

      const pkgSha = createHash("sha256").update(pkgBuffer).digest("hex");
      const pkgGzSha = createHash("sha256").update(pkgGzBuffer).digest("hex");

      return [
        ` ${pkgSha} ${pkgBuffer.length} main/binary-${arch}/Packages`,
        ` ${pkgGzSha} ${pkgGzBuffer.length} main/binary-${arch}/Packages.gz`,
      ];
    }),
  );

  const fileEntries = results.flat();

  return [
    "Origin: ArtCraft",
    "Label: ArtCraft",
    "Suite: stable",
    "Codename: stable",
    `Architectures: ${architectures.join(" ")}`,
    "Components: main",
    `Date: ${date.toUTCString()}`,
    "SHA256:",
    ...fileEntries,
    "",
  ].join("\n");
}

/** Keyring the apt sources point at; the guide installs it there before the first update. */
export const APT_KEYRING_PATH = "/etc/apt/keyrings/artcraft.gpg";

export async function generateAptSourcesList(baseUrl: string): Promise<string> {
  if (!(await isRepoSigningConfigured())) {
    return `deb [trusted=yes] ${baseUrl}/apt stable main\n`;
  }
  return `deb [signed-by=${APT_KEYRING_PATH}] ${baseUrl}/apt stable main\n`;
}

export async function generateAptSourcesDeb822(baseUrl: string): Promise<string> {
  return [
    "Types: deb",
    `URIs: ${baseUrl}/apt`,
    "Suites: stable",
    "Components: main",
    "Architectures: amd64 arm64 riscv64",
    (await isRepoSigningConfigured()) ? `Signed-By: ${APT_KEYRING_PATH}` : "Trusted: yes",
    "",
  ].join("\n");
}

function formatRpmRequire(req: RpmRequire): string {
  if (req.flags && req.ver) {
    return `<rpm:entry name="${escapeXml(req.name)}" flags="${req.flags}" epoch="${req.epoch ?? 0}" ver="${escapeXml(req.ver)}"/>`;
  }
  return `<rpm:entry name="${escapeXml(req.name)}"/>`;
}

function toRpmArchTag(arch: string): string {
  switch (arch) {
    case "x86_64":
      return "x86-64";
    case "aarch64":
      return "aarch-64";
    case "riscv64":
      return "riscv-64";
    default:
      return arch;
  }
}

function generateRpmPackageXml(
  app: LinuxRepoApp,
  asset: LinuxRepoApp["rpmAssets"][number],
  timestamp: number,
): string {
  const rpmArchTag = toRpmArchTag(asset.arch);

  return `  <package type="rpm">
    <name>${escapeXml(app.name)}</name>
    <arch>${escapeXml(asset.arch)}</arch>
    <version epoch="0" ver="${escapeXml(app.version)}" rel="1"/>
    <checksum type="sha256" pkgid="YES">${asset.sha256}</checksum>
    <summary>${escapeXml(app.name)}: ${escapeXml(app.description.slice(0, 80))}</summary>
    <description>${escapeXml(app.description)}</description>
    <packager>ArtCraft Maintainers &lt;hello@storyteller.ai&gt;</packager>
    <url>https://getartcraft.com/apps/${escapeXml(app.slug)}</url>
    <time file="${timestamp}" build="${timestamp}"/>
    <size package="${asset.size}" installed="${asset.installedSize}" archive="${asset.installedSize}"/>
    <location href="packages/${escapeXml(asset.filename)}"/>
    <format>
      <rpm:license>GPL-3.0</rpm:license>
      <rpm:group>Applications/Graphics</rpm:group>
      <rpm:provides>
        <rpm:entry name="${escapeXml(app.name)}" flags="EQ" epoch="0" ver="${escapeXml(app.version)}" rel="1"/>
        <rpm:entry name="${escapeXml(app.name)}(${rpmArchTag})" flags="EQ" epoch="0" ver="${escapeXml(app.version)}" rel="1"/>
      </rpm:provides>
      <rpm:requires>
        ${asset.requires.map(formatRpmRequire).join("\n        ")}
      </rpm:requires>
      <file>/usr/bin/${escapeXml(app.name)}</file>
    </format>
  </package>`;
}

export async function generateRpmPrimaryXml(
  timestamp: number = RPM_REPO_TIMESTAMP,
  archFilter?: string,
): Promise<string> {
  const packageXmls: string[] = [];
  const apps = await getLinuxRepoApps({ rpm: true });

  for (const app of apps) {
    for (const asset of app.rpmAssets) {
      if (archFilter && asset.arch !== archFilter) {
        continue;
      }
      packageXmls.push(generateRpmPackageXml(app, asset, timestamp));
    }
  }

  return `<?xml version="1.0" encoding="UTF-8"?>
<metadata xmlns="http://linux.duke.edu/metadata/common" xmlns:rpm="http://linux.duke.edu/metadata/rpm" packages="${packageXmls.length}">
${packageXmls.join("\n")}
</metadata>
`;
}

export function generateRpmRepomdXml(
  primaryXmlBuffer: Buffer,
  primaryGzBuffer: Buffer,
  timestamp: number = RPM_REPO_TIMESTAMP,
): string {
  const openSha = createHash("sha256").update(primaryXmlBuffer).digest("hex");
  const gzSha = createHash("sha256").update(primaryGzBuffer).digest("hex");

  return `<?xml version="1.0" encoding="UTF-8"?>
<repomd xmlns="http://linux.duke.edu/metadata/repo">
  <data type="primary">
    <checksum type="sha256">${gzSha}</checksum>
    <open-checksum type="sha256">${openSha}</open-checksum>
    <location href="repodata/primary.xml.gz"/>
    <timestamp>${timestamp}</timestamp>
    <size>${primaryGzBuffer.length}</size>
    <open-size>${primaryXmlBuffer.length}</open-size>
  </data>
</repomd>
`;
}

export async function getRpmPrimaryXml(archFilter?: string): Promise<string> {
  return generateRpmPrimaryXml(RPM_REPO_TIMESTAMP, archFilter);
}

export async function getRpmPrimaryGz(archFilter?: string): Promise<Buffer> {
  const primaryXml = await getRpmPrimaryXml(archFilter);
  return gzipSync(Buffer.from(primaryXml, "utf-8"));
}

export async function getRpmRepomdXml(archFilter?: string): Promise<string> {
  const primaryXml = await getRpmPrimaryXml(archFilter);
  const primaryBuffer = Buffer.from(primaryXml, "utf-8");
  const primaryGzBuffer = gzipSync(primaryBuffer);
  return generateRpmRepomdXml(primaryBuffer, primaryGzBuffer, RPM_REPO_TIMESTAMP);
}

export async function generateRpmRepoConfig(baseUrl: string): Promise<string> {
  if (!(await isRepoSigningConfigured())) {
    return `[artcraft]
name=ArtCraft Linux Packages
baseurl=${baseUrl}/rpm
enabled=1
gpgcheck=0
repo_gpgcheck=0
`;
  }
  return `[artcraft]
# Signed metadata and signed packages: dnf verifies repomd.xml against its signature, imports the
# key below, and verifies the OpenPGP header signature inside every package it downloads.
name=ArtCraft Linux Packages
baseurl=${baseUrl}/rpm
enabled=1
gpgcheck=1
repo_gpgcheck=1
gpgkey=${baseUrl}/keys/artcraft.asc
`;
}

export function createTarHeader(
  name: string,
  size: number,
  type: "0" | "5" = "0",
  mtime: number = PACMAN_REPO_TIMESTAMP,
): Buffer {
  const buf = Buffer.alloc(512);
  buf.write(name, 0, 100, "utf-8");
  buf.write(type === "5" ? "0000755\0" : "0000644\0", 100, 8, "utf-8");
  buf.write("0000000\0", 108, 8, "utf-8");
  buf.write("0000000\0", 116, 8, "utf-8");
  const sizeOctal = (type === "5" ? 0 : size).toString(8).padStart(11, "0") + "\0";
  buf.write(sizeOctal, 124, 12, "utf-8");
  const mtimeOctal = mtime.toString(8).padStart(11, "0") + "\0";
  buf.write(mtimeOctal, 136, 12, "utf-8");
  buf.fill(0x20, 148, 156);
  buf.write(type, 156, 1, "utf-8");
  buf.write("ustar\0", 257, 6, "utf-8");
  buf.write("00", 263, 2, "utf-8");
  buf.write("root", 265, 32, "utf-8");
  buf.write("root", 297, 32, "utf-8");

  let chksum = 0;
  for (let i = 0; i < 512; i++) {
    chksum += buf[i];
  }
  const chksumStr = chksum.toString(8).padStart(6, "0") + "\0 ";
  buf.write(chksumStr, 148, 8, "utf-8");
  return buf;
}

export function buildTar(
  entries: Array<{ name: string; type: "file" | "dir"; content?: string; mtime?: number }>,
): Buffer {
  const blocks: Buffer[] = [];
  for (const entry of entries) {
    const isDir = entry.type === "dir";
    const contentBuf = isDir || !entry.content ? Buffer.alloc(0) : Buffer.from(entry.content, "utf-8");
    const header = createTarHeader(entry.name, contentBuf.length, isDir ? "5" : "0", entry.mtime ?? PACMAN_REPO_TIMESTAMP);
    blocks.push(header);
    if (!isDir && contentBuf.length > 0) {
      blocks.push(contentBuf);
      const remainder = contentBuf.length % 512;
      if (remainder !== 0) {
        blocks.push(Buffer.alloc(512 - remainder));
      }
    }
  }
  blocks.push(Buffer.alloc(1024));
  return Buffer.concat(blocks);
}

export function generatePacmanDesc(
  app: LinuxRepoApp,
  asset: LinuxRepoApp["archAssets"][number],
  timestamp: number = PACMAN_REPO_TIMESTAMP,
): string {
  const pkgname = `ai.storyteller.${app.name}`;
  // pacman compares these against the package's own .PKGINFO, so mirror what the package declares
  // and fall back to the shared builder's lists only when .PKGINFO could not be read.
  const provides = asset.provides.length > 0 ? asset.provides : [`${app.name}=${app.version}-1`];
  const optdepends = asset.optdepends.length > 0 ? asset.optdepends : DEFAULT_PACMAN_OPTDEPENDS;
  const optdependsSection = optdepends.length > 0 ? `%OPTDEPENDS%\n${optdepends.join("\n")}\n` : "";

  return `%FILENAME%
${asset.filename}

%NAME%
${pkgname}

%BASE%
${pkgname}

%VERSION%
${app.version}-1

%DESC%
${app.description}

%CSIZE%
${asset.size}

%ISIZE%
${asset.installedSize}

%SHA256SUM%
${asset.sha256}

%URL%
https://getartcraft.com/apps/${app.slug}

%LICENSE%
MIT OR Apache-2.0

%ARCH%
${asset.arch}

%BUILDDATE%
${timestamp}

%PACKAGER%
ArtCraft Maintainers <hello@storyteller.ai>

%CONFLICTS%
${app.name}
${app.name}-bin
${app.name}-git

%PROVIDES%
${provides.join("\n")}

%DEPENDS%
${asset.depends.join("\n")}

${optdependsSection}`.trimEnd() + "\n";
}

export async function generatePacmanDb(
  arch: string,
  timestamp: number = PACMAN_REPO_TIMESTAMP,
): Promise<Buffer> {
  const entries: Array<{ name: string; type: "file" | "dir"; content?: string; mtime?: number }> = [];
  const apps = await getLinuxRepoApps({ arch: true });

  for (const app of apps) {
    const asset = app.archAssets.find((a) => a.arch === arch);
    if (!asset) continue;

    const dirName = `ai.storyteller.${app.name}-${app.version}-1`;
    entries.push(
      {
        name: `${dirName}/`,
        type: "dir",
        mtime: timestamp,
      },
      {
        name: `${dirName}/desc`,
        type: "file",
        content: generatePacmanDesc(app, asset, timestamp),
        mtime: timestamp,
      },
    );
  }

  const tarBuffer = buildTar(entries);
  return gzipSync(tarBuffer);
}

export async function generatePacmanRepoConfig(baseUrl: string): Promise<string> {
  if (!(await isRepoSigningConfigured())) {
    return `[artcraft]
SigLevel = Optional TrustAll
Server = ${baseUrl}/arch/$arch
`;
  }
  return `[artcraft]
# Import the key before syncing, otherwise pacman rejects the repository:
#   sudo pacman-key --add /tmp/artcraft.asc && sudo pacman-key --lsign-key <fingerprint>
SigLevel = Required DatabaseRequired
Server = ${baseUrl}/arch/$arch
`;
}
