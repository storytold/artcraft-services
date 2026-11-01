import { createHash } from "node:crypto";
import { gzipSync } from "node:zlib";
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
  craftAnythingDownloads,
} from "./craft-anything";
import {
  CRAFT_LAUNCHER_RELEASE,
  CRAFT_ANYTHING_RELEASE,
} from "./crafting-app-releases";

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
  }[];
  archAssets: {
    arch: ArchPackageArch;
    filename: string;
    size: number;
    sha256: string;
    installedSize: number;
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
 * Extract public base URL from incoming Request, supporting reverse proxy headers.
 */
export function getRequestBaseUrl(request: Request): string {
  const url = new URL(request.url);
  const host = request.headers.get("x-forwarded-host") || request.headers.get("host") || url.host;
  const proto = request.headers.get("x-forwarded-proto") || (url.protocol ? url.protocol.replace(":", "") : "https");
  return `${proto}://${host}`;
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

  if (CRAFT_ANYTHING_RELEASE) {
    const anythingDownloads = craftAnythingDownloads();
    const match = anythingDownloads.find((d) => d.fileName === filename);
    if (match) return match.href;
  }

  for (const app of CRAFTING_APPS) {
    const release = craftAppRelease(app);
    if (!release) continue;
    const downloads = craftReleaseDownloads(app, release);
    const match = downloads.find((d) => d.fileName === filename);
    if (match) return match.href;
  }

  return null;
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
      try {
        const res = await fetch(url);
        if (!res.ok) {
          return new Map<string, string>();
        }
        const text = await res.text();
        const map = new Map<string, string>();
        for (const line of text.split("\n")) {
          const trimmed = line.trim();
          if (!trimmed) continue;
          const [sha, fname] = trimmed.split(/\s+/);
          if (sha && fname) {
            map.set(fname.replace(/^\*/, ""), sha);
          }
        }
        return map;
      } catch {
        return new Map<string, string>();
      }
    })();
    shaSumsCache.set(key, promise);
  }
  return promise;
}

/**
 * Dynamically computes metadata (size, SHA-256 digest, installed size) for a package asset.
 * Fetches SHA256SUMS.txt and HTTP metadata on demand and caches in memory.
 */
export async function getLinuxPackageMetadata(
  filename: string,
  repo: string,
  version: string,
): Promise<LinuxPackageAssetMeta> {
  let promise = packageMetaCache.get(filename);
  if (!promise) {
    promise = (async () => {
      const downloadUrl = resolvePackageDownloadUrl(filename);
      if (!downloadUrl) {
        return {
          size: 100_000_000,
          sha256: "0000000000000000000000000000000000000000000000000000000000000000",
          installedSize: 200_000,
        };
      }

      // 1. Fetch sha256 checksum from SHA256SUMS.txt
      const sumsMap = await getShaSums(repo, version);
      const sha256 =
        sumsMap.get(filename) ||
        "0000000000000000000000000000000000000000000000000000000000000000";

      let size = 0;
      let installedSize = 0;

      if (filename.endsWith(".pkg.tar.zst")) {
        try {
          const { Decompress } = await import("fzstd");
          // Attempt fast 128KB Range request (where .PKGINFO lives for standard packages)
          const rangeRes = await fetch(downloadUrl, {
            headers: { Range: "bytes=0-131071" },
          });
          const contentRange = rangeRes.headers.get("content-range");
          const sizeMatch = contentRange?.match(/\/(\d+)$/);
          if (sizeMatch) {
            size = parseInt(sizeMatch[1], 10);
          }

          const buf = new Uint8Array(await rangeRes.arrayBuffer());
          const chunks: Buffer[] = [];
          const d = new Decompress((chunk) => {
            chunks.push(Buffer.from(chunk));
          });
          try {
            d.push(buf, false);
          } catch {
            // Partial stream EOF expected
          }
          const text = Buffer.concat(chunks).toString("utf-8");
          const isizeMatch = text.match(/size = (\d+)/);
          if (isizeMatch) {
            installedSize = parseInt(isizeMatch[1], 10);
          } else {
            // Fallback for packages where .PKGINFO is placed after usr/bin
            const fullRes = await fetch(downloadUrl);
            if (!size) {
              size = parseInt(fullRes.headers.get("content-length") || "0", 10);
            }
            let streamedIsize = 0;
            const needle = Buffer.from("size = ");
            const streamer = new Decompress((chunk) => {
              const buf = Buffer.from(chunk);
              const idx = buf.indexOf(needle);
              if (idx !== -1) {
                const slice = buf.slice(idx, idx + 30).toString("ascii");
                const m = slice.match(/size = (\d+)/);
                if (m) streamedIsize = parseInt(m[1], 10);
              }
            });

            if (fullRes.body) {
              const reader = fullRes.body.getReader();
              while (true) {
                const { done, value } = await reader.read();
                if (done) break;
                streamer.push(value, false);
                if (streamedIsize > 0) {
                  await reader.cancel();
                  break;
                }
              }
            }
            installedSize = streamedIsize || Math.round(size * 1.95);
          }
        } catch {
          if (!size) size = 100_000_000;
          if (!installedSize) installedSize = Math.round(size * 1.95);
        }
      } else {
        // .deb or .rpm
        try {
          const headRes = await fetch(downloadUrl, {
            method: "HEAD",
            redirect: "follow",
          });
          size = parseInt(headRes.headers.get("content-length") || "0", 10);
        } catch {
          size = 100_000_000;
        }

        if (filename.endsWith(".deb")) {
          installedSize = Math.round((size * 1.95) / 1024);
        } else {
          installedSize = Math.round(size * 1.95);
        }
      }

      return { size, sha256, installedSize };
    })();
    packageMetaCache.set(filename, promise);
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
  supportedArches?: DebPackageArch[],
): Promise<LinuxRepoApp["debAssets"]> {
  let debArchMap: DebArchMapItem[] = [
    { debArch: "amd64", fileArch: "x86_64" },
    { debArch: "arm64", fileArch: "aarch64" },
    { debArch: "riscv64", fileArch: "riscv64" },
  ];
  if (supportedArches) {
    debArchMap = debArchMap.filter((item) => supportedArches.includes(item.debArch));
  }

  const baseDepends = "libc6 (>= 2.35), libgcc-s1, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libx11-6, libx11-xcb1, libxcb1, libxcursor1, libxi6, libvulkan1 | libegl1";
  const defaultDepends = AUDIO_APPS.has(name)
    ? `${baseDepends}, libasound2 | libasound2t64`
    : baseDepends;
  const defaultRecommends = "mesa-vulkan-drivers, libegl-mesa0, xdg-desktop-portal";

  return Promise.all(
    debArchMap.map(async ({ debArch, fileArch }) => {
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
}

async function buildRpmAssets(
  name: string,
  repo: string,
  version: string,
  supportedArches?: RpmPackageArch[],
): Promise<LinuxRepoApp["rpmAssets"]> {
  let arches: RpmPackageArch[] = ["x86_64", "aarch64", "riscv64"];
  if (supportedArches) {
    arches = arches.filter((arch) => supportedArches.includes(arch));
  }

  return Promise.all(
    arches.map(async (arch) => {
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
      };
    })
  );
}

async function buildArchAssets(
  name: string,
  repo: string,
  version: string,
): Promise<LinuxRepoApp["archAssets"]> {
  const arches: ArchPackageArch[] = ["x86_64", "aarch64"];

  return Promise.all(
    arches.map(async (arch) => {
      const filename = `${name}-${version}-linux-${arch}.pkg.tar.zst`;
      const meta = await getLinuxPackageMetadata(filename, repo, version);
      return {
        arch,
        filename,
        size: meta.size,
        sha256: meta.sha256,
        installedSize: meta.installedSize,
      };
    })
  );
}

let cachedAllAppsPromise: Promise<LinuxRepoApp[]> | null = null;
export let LINUX_REPO_APPS: LinuxRepoApp[] = [];

/**
 * Dynamically builds the repository catalog informed by crafting-apps and crafting-app-releases.
 * Package metadata is fetched and computed on demand.
 */
export async function getLinuxRepoApps(formats?: RepoFormatFilter): Promise<LinuxRepoApp[]> {
  const includeDeb = formats ? !!formats.deb : true;
  const includeRpm = formats ? !!formats.rpm : true;
  const includeArch = formats ? !!formats.arch : true;

  if (includeDeb && includeRpm && includeArch && cachedAllAppsPromise) {
    return cachedAllAppsPromise;
  }

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

    // Craft Anything
    if (CRAFT_ANYTHING_RELEASE) {
      appPromises.push((async () => {
        const [debAssets, rpmAssets] = await Promise.all([
          includeDeb ? buildDebAssets("craft-anything", "craft-anything", CRAFT_ANYTHING_RELEASE.version, ["amd64", "arm64"]) : Promise.resolve([]),
          includeRpm ? buildRpmAssets("craft-anything", "craft-anything", CRAFT_ANYTHING_RELEASE.version, ["x86_64", "aarch64"]) : Promise.resolve([]),
        ]);
        return {
          slug: "craft-anything",
          name: "craft-anything",
          version: CRAFT_ANYTHING_RELEASE.version,
          description: "Craft Anything: find out how any software works, then craft your own.",
          section: "devel",
          debAssets,
          rpmAssets,
          archAssets: [],
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
        return {
          slug: app.slug,
          name: app.slug,
          version: release.version,
          description: `${craftAppName(app)}: ${app.lede || app.pitch}`,
          section: app.schemaCategory === "BusinessApplication" ? "utils" : "graphics",
          debAssets,
          rpmAssets,
          archAssets,
        };
      })());
    }

    const apps = await Promise.all(appPromises);
    const result = apps.filter((a): a is LinuxRepoApp => a !== null);
    if (includeDeb && includeRpm && includeArch) {
      LINUX_REPO_APPS = result;
    }
    return result;
  };

  if (includeDeb && includeRpm && includeArch) {
    cachedAllAppsPromise = compute();
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
        "Cache-Control": "public, max-age=0, must-revalidate",
      },
    });
  }

  return new Response(content, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=0, must-revalidate",
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
  const fileEntries: string[] = [];

  for (const arch of architectures) {
    const pkgContent = await generateAptPackages(arch);
    const pkgBuffer = Buffer.from(pkgContent, "utf-8");
    const pkgGzBuffer = gzipSync(pkgBuffer);

    const pkgSha = createHash("sha256").update(pkgBuffer).digest("hex");
    const pkgGzSha = createHash("sha256").update(pkgGzBuffer).digest("hex");

    fileEntries.push(
      ` ${pkgSha} ${pkgBuffer.length} main/binary-${arch}/Packages`,
      ` ${pkgGzSha} ${pkgGzBuffer.length} main/binary-${arch}/Packages.gz`
    );
  }

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

export function generateAptSourcesList(baseUrl: string): string {
  return `deb [trusted=yes] ${baseUrl}/apt stable main\n`;
}

export function generateAptSourcesDeb822(baseUrl: string): string {
  return [
    "Types: deb",
    `URIs: ${baseUrl}/apt`,
    "Suites: stable",
    "Components: main",
    "Architectures: amd64 arm64 riscv64",
    "Trusted: yes",
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

export function generateRpmRepoConfig(baseUrl: string): string {
  return `[artcraft]
name=ArtCraft Linux Packages
baseurl=${baseUrl}/rpm
enabled=1
gpgcheck=0
repo_gpgcheck=0
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
  const provides = app.name === "designcraft" ? "designcraft" : `${app.name}=${app.version}-1`;
  const optdepends = app.name === "designcraft"
    ? ""
    : `%OPTDEPENDS%
alsa-lib: sound support
pipewire-alsa: audio playback and recording via PipeWire
pulseaudio: audio playback and recording via PulseAudio
wayland: native Wayland display support
libx11: X11 and XWayland display support
libxcursor: X11 cursor handling
libxi: X11 input handling
libxrandr: X11 display resolution handling
libglvnd: GPU rendering through EGL
mesa: GPU rendering through EGL
vulkan-driver: GPU rendering through Vulkan
xdg-desktop-portal: native desktop integration and file dialogs
cups: printing support
`;

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
${provides}

%DEPENDS%
glibc
gcc-libs
hicolor-icon-theme
libxkbcommon
vulkan-icd-loader

${optdepends}`.trimEnd() + "\n";
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

export function generatePacmanRepoConfig(baseUrl: string): string {
  return `[artcraft]
SigLevel = Optional TrustAll
Server = ${baseUrl}/arch/$arch
`;
}
