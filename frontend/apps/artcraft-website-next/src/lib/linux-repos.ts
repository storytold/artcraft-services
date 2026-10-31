import { createHash } from "node:crypto";
import { gzipSync } from "node:zlib";

export const RPM_REPO_TIMESTAMP = 1730000000;
export const APT_REPO_DATE = new Date("2026-10-31T17:30:00Z");

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

/**
 * Catalog of Crafting apps available in the Linux package repositories.
 * Release metadata (file size, sha256, dependencies) verified against release assets.
 */
export const LINUX_REPO_APPS: LinuxRepoApp[] = [
  {
    slug: "pdfcraft",
    name: "pdfcraft",
    version: "0.16.1",
    description: "Open-source, native PDF workbench: read, comment, fill and sign forms, organize, combine, split, edit, redact, protect and export PDFs.",
    section: "graphics",
    debAssets: [
      {
        arch: "amd64",
        filename: "pdfcraft-0.16.1-linux-x86_64.deb",
        size: 112137992,
        sha256: "fb71f9b366214c8b0dafe01afba80d5280d554a7683ab7756f601a170668823b",
        depends: "libc6 (>= 2.35), libgcc-s1, libxkbcommon0, libvulkan1 | libegl1",
        installedSize: 213532,
      },
      {
        arch: "arm64",
        filename: "pdfcraft-0.16.1-linux-aarch64.deb",
        size: 109221366,
        sha256: "7c90af4c6af71da3efb017b697163267c5930e24e4d6b041edb8be0c1e23c31e",
        depends: "libc6 (>= 2.35), libgcc-s1, libxkbcommon0, libvulkan1 | libegl1",
        installedSize: 207840,
      },
      {
        arch: "riscv64",
        filename: "pdfcraft-0.16.1-linux-riscv64.deb",
        size: 109483726,
        sha256: "853e185e3c665c6da3c9d6973a121b2b0bb745c0aaab5880b9efe4438517a826",
        depends: "libc6 (>= 2.38), libgcc-s1, libxkbcommon0, libvulkan1 | libegl1",
        installedSize: 208500,
      },
    ],
    rpmAssets: [
      {
        arch: "x86_64",
        filename: "pdfcraft-0.16.1-linux-x86_64.rpm",
        size: 111638587,
        sha256: "b5d30dde0d5dea634af7a8f0a7bb1f81b75861ae7338c5b4c76a9b1da66ef2d3",
        requires: [
          { name: "libc.so.6()(64bit)" },
          { name: "libm.so.6(GLIBC_2.35)(64bit)" },
          { name: "libgcc_s.so.1()(64bit)" },
          { name: "libxkbcommon.so.0()(64bit)" },
          { name: "(libvulkan.so.1()(64bit) or libEGL.so.1()(64bit))" },
        ],
        installedSize: 213532000,
      },
      {
        arch: "aarch64",
        filename: "pdfcraft-0.16.1-linux-aarch64.rpm",
        size: 108543068,
        sha256: "9034a6ff6e40443f700f869c357bc9a5dc4d859fe87fc4a5dcf3759d57750810",
        requires: [
          { name: "libc.so.6()(64bit)" },
          { name: "libm.so.6(GLIBC_2.35)(64bit)" },
          { name: "libgcc_s.so.1()(64bit)" },
          { name: "libxkbcommon.so.0()(64bit)" },
          { name: "(libvulkan.so.1()(64bit) or libEGL.so.1()(64bit))" },
        ],
        installedSize: 207840000,
      },
      {
        arch: "riscv64",
        filename: "pdfcraft-0.16.1-linux-riscv64.rpm",
        size: 109167000,
        sha256: "8f1e9fa6b3da20df374d12db2bf895340530a48cd72c955e9bcfe9a458976c7e",
        requires: [
          { name: "libc.so.6()(64bit)" },
          { name: "libm.so.6(GLIBC_2.35)(64bit)" },
          { name: "libgcc_s.so.1()(64bit)" },
          { name: "libxkbcommon.so.0()(64bit)" },
          { name: "(libvulkan.so.1()(64bit) or libEGL.so.1()(64bit))" },
        ],
        installedSize: 208500000,
      },
    ],
  },
  {
    slug: "artcraft-launcher",
    name: "artcraft-launcher",
    version: "0.11.1",
    description: "ArtCraft Launcher: install, update and run native Crafting apps.",
    section: "utils",
    debAssets: [
      {
        arch: "amd64",
        filename: "artcraft-launcher-0.11.1-linux-x86_64.deb",
        size: 13710590,
        sha256: "b1d4b4e93612e0a5d3f1f86452b784cfbdf14295eeb294ba9348a18d149034eb",
        depends: "libc6 (>= 2.35), libgcc-s1, libxkbcommon0, libvulkan1 | libegl1",
        installedSize: 32000,
      },
      {
        arch: "arm64",
        filename: "artcraft-launcher-0.11.1-linux-aarch64.deb",
        size: 13166808,
        sha256: "89e3eadc69667dc117b5d7950c14311e28c4732f2691fec3b4ac9453eab6861b",
        depends: "libc6 (>= 2.35), libgcc-s1, libxkbcommon0, libvulkan1 | libegl1",
        installedSize: 31000,
      },
      {
        arch: "riscv64",
        filename: "artcraft-launcher-0.11.1-linux-riscv64.deb",
        size: 13616060,
        sha256: "0636f01916d58e7014d3f9600c7c7a185197f1ac1828880f7f63689d0ea3113e",
        depends: "libc6 (>= 2.39), libgcc-s1, libxkbcommon0, libvulkan1 | libegl1",
        installedSize: 31500,
      },
    ],
    rpmAssets: [
      {
        arch: "x86_64",
        filename: "artcraft-launcher-0.11.1-linux-x86_64.rpm",
        size: 13629087,
        sha256: "a86b7e9e709d7111f6b3a9d77a5cf3be8a7f0c8ee8b7486dfa42423787014b41",
        requires: [
          { name: "glibc", flags: "GE", epoch: 0, ver: "2.35" },
          { name: "libxkbcommon.so.0()(64bit)" },
          { name: "libxkbcommon-x11.so.0()(64bit)" },
          { name: "libwayland-client.so.0()(64bit)" },
          { name: "libX11.so.6()(64bit)" },
          { name: "libX11-xcb.so.1()(64bit)" },
          { name: "libxcb.so.1()(64bit)" },
          { name: "libXcursor.so.1()(64bit)" },
          { name: "libXi.so.6()(64bit)" },
          { name: "(libvulkan.so.1()(64bit) or libEGL.so.1()(64bit))" },
        ],
        installedSize: 32000000,
      },
      {
        arch: "aarch64",
        filename: "artcraft-launcher-0.11.1-linux-aarch64.rpm",
        size: 13043298,
        sha256: "912123716ece88e1b808c34873cfcc3871d086ec40d79bab258814d5e92a823f",
        requires: [
          { name: "glibc", flags: "GE", epoch: 0, ver: "2.35" },
          { name: "libxkbcommon.so.0()(64bit)" },
          { name: "libxkbcommon-x11.so.0()(64bit)" },
          { name: "libwayland-client.so.0()(64bit)" },
          { name: "libX11.so.6()(64bit)" },
          { name: "libX11-xcb.so.1()(64bit)" },
          { name: "libxcb.so.1()(64bit)" },
          { name: "libXcursor.so.1()(64bit)" },
          { name: "libXi.so.6()(64bit)" },
          { name: "(libvulkan.so.1()(64bit) or libEGL.so.1()(64bit))" },
        ],
        installedSize: 31000000,
      },
      {
        arch: "riscv64",
        filename: "artcraft-launcher-0.11.1-linux-riscv64.rpm",
        size: 13592087,
        sha256: "a172ea550f528200e2ec444353d049982a962f8347e6907122c580ed835fc53a",
        requires: [
          { name: "glibc", flags: "GE", epoch: 0, ver: "2.39" },
          { name: "libxkbcommon.so.0()(64bit)" },
          { name: "libxkbcommon-x11.so.0()(64bit)" },
          { name: "libwayland-client.so.0()(64bit)" },
          { name: "libX11.so.6()(64bit)" },
          { name: "libX11-xcb.so.1()(64bit)" },
          { name: "libxcb.so.1()(64bit)" },
          { name: "libXcursor.so.1()(64bit)" },
          { name: "libXi.so.6()(64bit)" },
          { name: "(libvulkan.so.1()(64bit) or libEGL.so.1()(64bit))" },
        ],
        installedSize: 31500000,
      },
    ],
  },
];

/** Resolve package file download request to GitHub Release binary asset */
export function resolvePackageDownloadUrl(filename: string): string | null {
  for (const app of LINUX_REPO_APPS) {
    const debAsset = app.debAssets.find((a) => a.filename === filename);
    if (debAsset) {
      const repo = app.slug === "artcraft-launcher" ? "craft-launcher" : app.slug;
      return `https://github.com/storytold/${repo}/releases/download/v${app.version}/${filename}`;
    }
    const rpmAsset = app.rpmAssets.find((a) => a.filename === filename);
    if (rpmAsset) {
      const repo = app.slug === "artcraft-launcher" ? "craft-launcher" : app.slug;
      return `https://github.com/storytold/${repo}/releases/download/v${app.version}/${filename}`;
    }
  }
  return null;
}

export function generateAptPackages(arch: string): string {
  const lines: string[] = [];

  for (const app of LINUX_REPO_APPS) {
    const asset = app.debAssets.find((a) => a.arch === arch);
    if (!asset) continue;

    lines.push(`Package: ${app.name}`);
    lines.push(`Version: ${app.version}-1`);
    lines.push(`Section: ${app.section}`);
    lines.push(`Priority: optional`);
    lines.push(`Architecture: ${asset.arch}`);
    lines.push(`Maintainer: ${app.name} maintainers <hello@storyteller.ai>`);
    lines.push(`Installed-Size: ${asset.installedSize}`);
    lines.push(`Depends: ${asset.depends}`);
    lines.push(`Homepage: https://getartcraft.com/apps/${app.slug}`);
    lines.push(`Filename: pool/main/${app.slug}/${asset.filename}`);
    lines.push(`Size: ${asset.size}`);
    lines.push(`SHA256: ${asset.sha256}`);
    lines.push(`Description: ${app.description}`);
    lines.push("");
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
    return `<rpm:entry name="${req.name}" flags="${req.flags}" epoch="${req.epoch ?? 0}" ver="${req.ver}"/>`;
  }
  return `<rpm:entry name="${req.name}"/>`;
}

export function generateRpmPrimaryXml(
  timestamp: number = RPM_REPO_TIMESTAMP,
  archFilter?: string,
): string {
  let packageCount = 0;
  const packageXmls: string[] = [];

  for (const app of LINUX_REPO_APPS) {
    for (const asset of app.rpmAssets) {
      if (archFilter && asset.arch !== archFilter) {
        continue;
      }
      packageCount++;
      packageXmls.push(`  <package type="rpm">
    <name>${app.name}</name>
    <arch>${asset.arch}</arch>
    <version epoch="0" ver="${app.version}" rel="1"/>
    <checksum type="sha256" pkgid="YES">${asset.sha256}</checksum>
    <summary>${app.name}: ${app.description.slice(0, 80)}</summary>
    <description>${app.description}</description>
    <packager>ArtCraft Maintainers &lt;hello@storyteller.ai&gt;</packager>
    <url>https://getartcraft.com/apps/${app.slug}</url>
    <time file="${timestamp}" build="${timestamp}"/>
    <size package="${asset.size}" installed="${asset.installedSize}" archive="${asset.installedSize}"/>
    <location href="packages/${asset.filename}"/>
    <format>
      <rpm:license>GPL-3.0</rpm:license>
      <rpm:group>Applications/Graphics</rpm:group>
      <rpm:provides>
        <rpm:entry name="${app.name}" flags="EQ" epoch="0" ver="${app.version}" rel="1"/>
        <rpm:entry name="${app.name}(${asset.arch === "x86_64" ? "x86-64" : asset.arch})" flags="EQ" epoch="0" ver="${app.version}" rel="1"/>
      </rpm:provides>
      <rpm:requires>
        ${asset.requires.map(formatRpmRequire).join("\n        ")}
      </rpm:requires>
    </format>
  </package>`);
    }
  }

  return `<?xml version="1.0" encoding="UTF-8"?>
<metadata xmlns="http://linux.duke.edu/metadata/common" xmlns:rpm="http://linux.duke.edu/metadata/rpm" packages="${packageCount}">
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
