import { createHash } from "node:crypto";
import { gzipSync } from "node:zlib";
import {
  CRAFTING_APPS,
  craftAppName,
  craftAppRelease,
  craftReleaseDownloads,
} from "./crafting-apps";
import {
  CRAFT_LAUNCHER_NAME,
  craftLauncherDownloads,
} from "./craft-launcher";
import {
  CRAFT_LAUNCHER_RELEASE,
} from "./crafting-app-releases";
import { LINUX_PACKAGE_METADATA } from "./linux-package-data";

export const RPM_REPO_TIMESTAMP = 1730000000;
export const APT_REPO_DATE = new Date("2026-10-31T17:30:00Z");

export const VALID_APT_DISTRIBUTIONS = new Set(["stable"]);
export const VALID_APT_COMPONENTS = new Set(["main"]);
export const VALID_APT_ARCHES = new Set(["amd64", "arm64", "riscv64"]);
export const VALID_RPM_ARCHES = new Set(["x86_64", "aarch64", "riscv64", "noarch"]);

export type RpmRequire = {
  name: string;
  flags?: "GE" | "EQ" | "LE";
  epoch?: number;
  ver?: string;
  rel?: string;
};

export type LinuxRepoApp = {
  slug: string;
  name: string;
  version: string;
  description: string;
  section: string;
  debAssets: {
    arch: "amd64" | "arm64" | "riscv64";
    filename: string;
    size: number;
    sha256: string;
    depends: string;
    recommends?: string;
    installedSize: number;
  }[];
  rpmAssets: {
    arch: "x86_64" | "aarch64" | "riscv64";
    filename: string;
    size: number;
    sha256: string;
    requires: RpmRequire[];
    installedSize: number;
  }[];
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

function buildDebAssets(
  name: string,
  version: string,
): LinuxRepoApp["debAssets"] {
  const debArchMap: Array<{ debArch: "amd64" | "arm64" | "riscv64"; fileArch: "x86_64" | "aarch64" | "riscv64" }> = [
    { debArch: "amd64", fileArch: "x86_64" },
    { debArch: "arm64", fileArch: "aarch64" },
    { debArch: "riscv64", fileArch: "riscv64" },
  ];

  const baseDepends = "libc6 (>= 2.35), libgcc-s1, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libx11-6, libx11-xcb1, libxcb1, libxcursor1, libxi6, libvulkan1 | libegl1";
  const defaultDepends = AUDIO_APPS.has(name)
    ? `${baseDepends}, libasound2 | libasound2t64`
    : baseDepends;
  const defaultRecommends = "mesa-vulkan-drivers, libegl-mesa0, xdg-desktop-portal";

  return debArchMap.map(({ debArch, fileArch }) => {
    const filename = `${name}-${version}-linux-${fileArch}.deb`;
    const meta = LINUX_PACKAGE_METADATA[filename] ?? {
      size: 100_000_000,
      sha256: "0000000000000000000000000000000000000000000000000000000000000000",
      installedSize: 200_000,
    };
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
  });
}

function buildRpmAssets(
  name: string,
  version: string,
): LinuxRepoApp["rpmAssets"] {
  const arches: Array<"x86_64" | "aarch64" | "riscv64"> = ["x86_64", "aarch64", "riscv64"];

  return arches.map((arch) => {
    const filename = `${name}-${version}-linux-${arch}.rpm`;
    const meta = LINUX_PACKAGE_METADATA[filename] ?? {
      size: 100_000_000,
      sha256: "0000000000000000000000000000000000000000000000000000000000000000",
      installedSize: 200_000_000,
    };
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
  });
}

/**
 * Dynamically builds the repository catalog informed by crafting-apps and crafting-app-releases.
 * No hardcoded list of apps is maintained here.
 */
export function getLinuxRepoApps(): LinuxRepoApp[] {
  const apps: LinuxRepoApp[] = [];

  // Craft Launcher
  if (CRAFT_LAUNCHER_RELEASE) {
    apps.push({
      slug: "artcraft-launcher",
      name: "artcraft-launcher",
      version: CRAFT_LAUNCHER_RELEASE.version,
      description: "ArtCraft Launcher: install, update and run native Crafting apps.",
      section: "utils",
      debAssets: buildDebAssets("artcraft-launcher", CRAFT_LAUNCHER_RELEASE.version),
      rpmAssets: buildRpmAssets("artcraft-launcher", CRAFT_LAUNCHER_RELEASE.version),
    });
  }

  // Crafting Apps
  for (const app of CRAFTING_APPS) {
    const release = craftAppRelease(app);
    if (!release) continue;

    apps.push({
      slug: app.slug,
      name: app.slug,
      version: release.version,
      description: `${craftAppName(app)}: ${app.lede || app.pitch}`,
      section: app.schemaCategory === "BusinessApplication" ? "utils" : "graphics",
      debAssets: buildDebAssets(app.slug, release.version),
      rpmAssets: buildRpmAssets(app.slug, release.version),
    });
  }

  return apps;
}

export const LINUX_REPO_APPS: LinuxRepoApp[] = getLinuxRepoApps();

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

export function handleAptPackagesRoute(
  distribution: string,
  component: string,
  archFolder: string,
  options?: { gzip?: boolean },
): Response {
  if (
    !VALID_APT_DISTRIBUTIONS.has(distribution) ||
    !VALID_APT_COMPONENTS.has(component) ||
    !archFolder.startsWith("binary-") ||
    !VALID_APT_ARCHES.has(archFolder.slice(7))
  ) {
    return new Response("Not found", { status: 404 });
  }

  const arch = archFolder.slice(7);
  const content = generateAptPackages(arch);

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

export function generateAptPackages(arch: string): string {
  const lines: string[] = [];
  const apps = getLinuxRepoApps();

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

export function generateAptRelease(date: Date = APT_REPO_DATE): string {
  const architectures = ["amd64", "arm64", "riscv64"];
  const fileEntries: string[] = [];

  for (const arch of architectures) {
    const pkgContent = generateAptPackages(arch);
    const pkgBuffer = Buffer.from(pkgContent, "utf-8");
    const pkgGzBuffer = gzipSync(pkgBuffer);

    const pkgSha = createHash("sha256").update(pkgBuffer).digest("hex");
    const pkgGzSha = createHash("sha256").update(pkgGzBuffer).digest("hex");

    fileEntries.push(` ${pkgSha} ${pkgBuffer.length} main/binary-${arch}/Packages`);
    fileEntries.push(` ${pkgGzSha} ${pkgGzBuffer.length} main/binary-${arch}/Packages.gz`);
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

export function generateRpmPrimaryXml(
  timestamp: number = RPM_REPO_TIMESTAMP,
  archFilter?: string,
): string {
  const packageXmls: string[] = [];
  const apps = getLinuxRepoApps();

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

export function getRpmPrimaryXml(archFilter?: string): string {
  return generateRpmPrimaryXml(RPM_REPO_TIMESTAMP, archFilter);
}

export function getRpmPrimaryGz(archFilter?: string): Buffer {
  return gzipSync(Buffer.from(getRpmPrimaryXml(archFilter), "utf-8"));
}

export function getRpmRepomdXml(archFilter?: string): string {
  const primaryXml = getRpmPrimaryXml(archFilter);
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
