import type { CraftAppSlug } from "./crafting-apps";

// GitHub releases for the Crafting Apps, which drive every download button on
// the /apps pages.
//
// To bump an app: change its version in CRAFT_APP_RELEASES (each app has its
// own, e.g. `craftRelease("0.4.0")`).
// To hide an app's downloads (Discord waitlist instead): set it to null.
// If a release renames, adds or drops files, edit CRAFT_RELEASE_ASSETS, or
// pass a custom list as craftRelease's second argument for a single app.
//
// Release tags are `v<version>` and live at github.com/storytold/<slug>.

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
    file: "{slug}-{version}-linux-x86_64.AppImage",
    group: "Linux",
    label: "AppImage (x86_64)",
    description: "Any distro. Mark it executable and run.",
    arch: "x86_64",
    recommended: true,
  },
  {
    file: "{slug}-{version}-linux-x86_64.flatpak",
    group: "Linux",
    label: "Flatpak (x86_64)",
    description: "Sandboxed. Install with flatpak install --user.",
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
    file: "{slug}-{version}-linux-aarch64.AppImage",
    group: "Linux",
    label: "AppImage (ARM64)",
    description: "Any distro on 64-bit ARM.",
    arch: "aarch64",
    recommended: true,
  },
  {
    file: "{slug}-{version}-linux-aarch64.flatpak",
    group: "Linux",
    label: "Flatpak (ARM64)",
    description: "Sandboxed, on 64-bit ARM.",
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

export function craftRelease(
  version: string,
  assets: readonly CraftReleaseAsset[] = CRAFT_RELEASE_ASSETS,
): CraftRelease {
  return { version, assets };
}

// Every app must be listed, so a new slug fails the type check until it is.
export const CRAFT_APP_RELEASES: Record<CraftAppSlug, CraftRelease | null> = {
  photocraft: craftRelease("0.5.0"),
  vectorcraft: craftRelease("0.6.0"),
  filmcraft: craftRelease("0.4.0"),
  lightcraft: craftRelease("0.4.0"),
  pdfcraft: craftRelease("0.4.0"),
  effectcraft: craftRelease("0.6.0"),
  designcraft: craftRelease("0.4.0"),
};
