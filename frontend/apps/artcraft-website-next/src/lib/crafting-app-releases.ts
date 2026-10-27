import type { CraftAppSlug } from "./crafting-apps";

// GitHub releases for the Crafting Apps, which drive every download button on
// the /apps pages.
//
// Versions are never edited here: each page asks GitHub for the repo's latest
// published release when it renders (fetchCraftRelease) and caches the answer
// for CRAFT_RELEASE_REVALIDATE_SECONDS, so a new release shows up on the live
// deploy within the hour, without a commit or a redeploy.
//
// To hide an app's downloads (Discord waitlist instead): set it to null in
// CRAFT_APP_ASSETS.
// If a release renames or adds files, edit CRAFT_RELEASE_ASSETS, or give a
// single app its own list in CRAFT_APP_ASSETS. Catalog files missing from a
// release are left out, so a dropped file never becomes a dead link.
//
// Releases live at github.com/storytold/<slug>.

export type CraftDesktopPlatform = "macOS" | "Windows" | "Linux";

/** Order of the per-platform download buttons. */
export const CRAFT_DESKTOP_PLATFORMS: CraftDesktopPlatform[] = ["macOS", "Windows", "Linux"];

export type CraftReleaseAsset = {
  /** Asset filename; `{slug}` and `{version}` are filled in per app. */
  file: string;
  /** Group in the full file list. "Other" holds the FreeBSD build, web build and checksums. */
  group: CraftDesktopPlatform | "Other";
  /** Short name, e.g. "Installer (x64)". */
  label: string;
  /** One line on who should pick this file. */
  description: string;
  /** CPU architecture, used to choose between recommended Linux builds. */
  arch?: "universal" | "x64" | "x86" | "x86_64" | "aarch64";
  /**
   * Offered as a platform's main download button. The first recommended
   * asset in a group is the default; a later one wins only when its `arch`
   * matches the visitor's detected CPU.
   */
  recommended?: boolean;
};

export type CraftRelease = {
  /** Version without the leading "v", e.g. "0.2.0". */
  version: string;
  /** Git tag. Defaults to `v<version>`. */
  tag?: string;
  /** Fills `{slug}` in filenames. Defaults to the app's slug; a release from before a rename keeps the old one. */
  fileSlug?: string;
  assets: readonly CraftReleaseAsset[];
  /** The release's actual file names; catalog assets not in it are left out. */
  files?: readonly string[];
};

// The file set every Crafting App publishes, in display order within each group.
export const CRAFT_RELEASE_ASSETS: readonly CraftReleaseAsset[] = [
  {
    file: "{slug}-{version}-macos-universal.dmg",
    group: "macOS",
    label: "Disk image (Universal)",
    description: "Apple silicon and Intel Macs.",
    arch: "universal",
    recommended: true,
  },
  {
    file: "{slug}-cli-{version}-macos-universal.zip",
    group: "macOS",
    label: "Command-line tool (Universal)",
    description: "The headless CLI for scripts and agents.",
    arch: "universal",
  },
  {
    file: "{slug}-{version}-windows-x64.msi",
    group: "Windows",
    label: "Installer (64-bit)",
    description: "Most Windows 10 and 11 PCs.",
    arch: "x64",
    recommended: true,
  },
  {
    file: "{slug}-{version}-windows-x64-portable.zip",
    group: "Windows",
    label: "Portable (64-bit)",
    description: "Unzip and run, no install.",
    arch: "x64",
  },
  {
    file: "{slug}-{version}-windows-x86.msi",
    group: "Windows",
    label: "Installer (32-bit)",
    description: "Older 32-bit Windows.",
    arch: "x86",
  },
  {
    file: "{slug}-{version}-windows-x86-portable.zip",
    group: "Windows",
    label: "Portable (32-bit)",
    description: "Unzip and run on 32-bit Windows.",
    arch: "x86",
  },
  {
    file: "{slug}-{version}-windows-arm64.msi",
    group: "Windows",
    label: "Installer (ARM64)",
    description: "Windows 11 on ARM laptops.",
    arch: "aarch64",
  },
  {
    file: "{slug}-{version}-windows-arm64-portable.zip",
    group: "Windows",
    label: "Portable (ARM64)",
    description: "Unzip and run on Windows on ARM.",
    arch: "aarch64",
  },
  {
    file: "{slug}-{version}-linux-x86_64.flatpak",
    group: "Linux",
    label: "Flatpak (x86_64)",
    description: "Sandboxed. Install with flatpak install --user.",
    arch: "x86_64",
    recommended: true,
  },
  {
    file: "{slug}-{version}-linux-x86_64.AppImage",
    group: "Linux",
    label: "AppImage (x86_64)",
    description: "Any distro. Mark it executable and run.",
    arch: "x86_64",
  },
  {
    file: "{slug}-{version}-linux-x86_64.deb",
    group: "Linux",
    label: "Debian package (x86_64)",
    description: "Debian, Ubuntu and derivatives.",
    arch: "x86_64",
  },
  {
    file: "{slug}-{version}-linux-x86_64.rpm",
    group: "Linux",
    label: "RPM package (x86_64)",
    description: "Fedora, RHEL and openSUSE.",
    arch: "x86_64",
  },
  {
    file: "{slug}-{version}-linux-x86_64.tar.gz",
    group: "Linux",
    label: "Tarball (x86_64)",
    description: "Plain binaries to unpack anywhere.",
    arch: "x86_64",
  },
  {
    file: "{slug}-{version}-linux-aarch64.flatpak",
    group: "Linux",
    label: "Flatpak (ARM64)",
    description: "Sandboxed, on 64-bit ARM.",
    arch: "aarch64",
    recommended: true,
  },
  {
    file: "{slug}-{version}-linux-aarch64.AppImage",
    group: "Linux",
    label: "AppImage (ARM64)",
    description: "Any distro on 64-bit ARM.",
    arch: "aarch64",
  },
  {
    file: "{slug}-{version}-linux-aarch64.deb",
    group: "Linux",
    label: "Debian package (ARM64)",
    description: "Debian, Ubuntu and derivatives on ARM.",
    arch: "aarch64",
  },
  {
    file: "{slug}-{version}-linux-aarch64.rpm",
    group: "Linux",
    label: "RPM package (ARM64)",
    description: "Fedora, RHEL and openSUSE on ARM.",
    arch: "aarch64",
  },
  {
    file: "{slug}-{version}-linux-aarch64.tar.gz",
    group: "Linux",
    label: "Tarball (ARM64)",
    description: "Plain binaries for 64-bit ARM.",
    arch: "aarch64",
  },
  {
    file: "{slug}-{version}-freebsd-x86_64.tar.gz",
    group: "Other",
    label: "FreeBSD (x86_64)",
    description: "Plain binaries for FreeBSD 14.",
  },
  {
    file: "{slug}-web-{version}.zip",
    group: "Other",
    label: "Web build",
    description: "The WebAssembly app, to host yourself.",
  },
  {
    file: "SHA256SUMS.txt",
    group: "Other",
    label: "SHA-256 checksums",
    description: "Verify any download above.",
  },
];

/** How long a fetched release is cached before the next request rechecks GitHub. */
export const CRAFT_RELEASE_REVALIDATE_SECONDS = 3600;

/**
 * The latest published release of github.com/storytold/<repo> (never a draft
 * or pre-release). Throws when GitHub can't answer: a build then fails and the
 * previous deploy stays live, and a background revalidation keeps serving the
 * last good page. Set GITHUB_TOKEN in Netlify to lift the anonymous rate limit.
 */
export async function fetchCraftRelease(
  repo: string,
  assets: readonly CraftReleaseAsset[],
): Promise<CraftRelease> {
  const headers: Record<string, string> = {
    Accept: "application/vnd.github+json",
    "User-Agent": "getartcraft.com",
    "X-GitHub-Api-Version": "2022-11-28",
  };
  if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  const res = await fetch(`https://api.github.com/repos/storytold/${repo}/releases/latest`, {
    headers,
    next: { revalidate: CRAFT_RELEASE_REVALIDATE_SECONDS },
  });
  if (!res.ok) throw new Error(`GitHub latest release of storytold/${repo}: HTTP ${res.status}`);
  const body = (await res.json()) as { tag_name?: unknown; assets?: { name?: unknown }[] };
  const tag = typeof body.tag_name === "string" ? body.tag_name : "";
  if (!/^v\d+\.\d+\.\d+/.test(tag)) {
    throw new Error(`GitHub latest release of storytold/${repo}: unexpected tag "${tag}"`);
  }
  return {
    version: tag.slice(1),
    tag,
    assets,
    files: (body.assets ?? []).flatMap((asset) =>
      typeof asset.name === "string" ? [asset.name] : [],
    ),
  };
}

// ArtCraft Launcher (github.com/storytold/craft-launcher) installs and updates
// every app above, and the /apps pages recommend it first. Its releases ship
// the same file set minus the macOS CLI, named artcraft-launcher-*.
export const CRAFT_LAUNCHER_ASSETS: readonly CraftReleaseAsset[] | null =
  CRAFT_RELEASE_ASSETS.filter((asset) => !asset.file.startsWith("{slug}-cli-"));

// Every app must be listed, so a new slug fails the type check until it is.
export const CRAFT_APP_ASSETS: Record<CraftAppSlug, readonly CraftReleaseAsset[] | null> = {
  photocraft: CRAFT_RELEASE_ASSETS,
  vectorcraft: CRAFT_RELEASE_ASSETS,
  filmcraft: CRAFT_RELEASE_ASSETS,
  lightcraft: CRAFT_RELEASE_ASSETS,
  pdfcraft: CRAFT_RELEASE_ASSETS,
  effectcraft: CRAFT_RELEASE_ASSETS,
  designcraft: CRAFT_RELEASE_ASSETS,
  figcraft: CRAFT_RELEASE_ASSETS,
  wordcraft: CRAFT_RELEASE_ASSETS,
  soundcraft: CRAFT_RELEASE_ASSETS,
  deckcraft: CRAFT_RELEASE_ASSETS,
  cadcraft: CRAFT_RELEASE_ASSETS,
  gridcraft: CRAFT_RELEASE_ASSETS,
};
