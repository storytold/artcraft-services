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

export type CraftDesktopPlatform = "macOS" | "Windows" | "Linux" | "FreeBSD";

/** Order of the per-platform download buttons. */
export const CRAFT_DESKTOP_PLATFORMS: CraftDesktopPlatform[] = ["macOS", "Windows", "Linux", "FreeBSD"];

export type CraftReleaseAsset = {
  /** Asset filename; `{slug}` and `{version}` are filled in per app. */
  file: string;
  /** Group in the full file list. "Other" holds the web build and checksums. */
  group: CraftDesktopPlatform | "Other";
  /** Short name, e.g. "Installer (x64)". */
  label: string;
  /** One line on who should pick this file. */
  description: string;
  /** CPU architecture, used to choose between recommended Linux builds. */
  arch?: "universal" | "x64" | "x86_64" | "aarch64" | "riscv64";
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
// Windows builds are x64 and ARM64 only; 32-bit Windows is no longer shipped.
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
  // Keep literal filenames here: the release updater extracts them from this array.
  ...[
    { file: "{slug}-{version}-linux-riscv64.tar.gz", label: "Tarball", description: "Plain binaries for 64-bit RISC-V.", recommended: true },
    { file: "{slug}-{version}-linux-riscv64.deb", label: "Debian package", description: "Debian and derivatives on 64-bit RISC-V." },
    { file: "{slug}-{version}-linux-riscv64.rpm", label: "RPM package", description: "RPM-based distributions on 64-bit RISC-V." },
  ].map(({ file, label, description, recommended }): CraftReleaseAsset => ({
    file,
    group: "Linux",
    label: `${label} (RISC-V 64)`,
    description,
    arch: "riscv64",
    recommended,
  })),
  {
    file: "{slug}-{version}-linux-x86_64.pkg.tar.zst",
    group: "Linux",
    label: "Arch package (x86_64)",
    description: "Arch Linux, Manjaro, and derivatives.",
    arch: "x86_64",
  },
  {
    file: "{slug}-{version}-linux-aarch64.pkg.tar.zst",
    group: "Linux",
    label: "Arch package (ARM64)",
    description: "Arch Linux ARM and derivatives on 64-bit ARM.",
    arch: "aarch64",
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
  {
    file: "{slug}-{version}-freebsd-x86_64.tar.gz",
    group: "FreeBSD",
    label: "FreeBSD (x86_64)",
    description: "Plain binaries for FreeBSD 15.1.",
    arch: "x86_64",
    recommended: true,
  },
];

export function craftRelease(
  version: string,
  assets: readonly CraftReleaseAsset[] = CRAFT_RELEASE_ASSETS,
): CraftRelease {
  return { version, assets };
}

// ArtCraft Launcher (github.com/storytold/craft-launcher) installs and updates
// every app above, and the /apps pages recommend it first. Its releases ship
// the same file set minus the macOS CLI, named artcraft-launcher-*.
export const CRAFT_LAUNCHER_RELEASE: CraftRelease | null = craftRelease(
  "0.20.3",
  CRAFT_RELEASE_ASSETS
    .filter((asset) => !asset.file.startsWith("{slug}-cli-"))
    // The launcher's RISC-V binary needs glibc 2.39, including in its tarball.
    // Keep this launcher-specific; the other apps have their own release floors.
    .map((asset) => asset.arch === "riscv64"
      ? { ...asset, description: `${asset.description} Requires glibc 2.39 or newer.` }
      : asset),
);

// Every app must be listed, so a new slug fails the type check until it is.
export const CRAFT_APP_RELEASES: Record<CraftAppSlug, CraftRelease | null> = {
  photocraft: craftRelease("0.23.1"),
  vectorcraft: craftRelease("0.24.1"),
  filmcraft: craftRelease("0.20.1"),
  lightcraft: craftRelease("0.21.1"),
  pdfcraft: craftRelease("0.22.0"),
  effectcraft: craftRelease("0.25.1"),
  // AniCraft publishes no web ZIP build yet.
  anicraft: craftRelease("0.3.0", CRAFT_RELEASE_ASSETS.filter(
    (asset) => !asset.file.startsWith("{slug}-web-"),
  )),
  designcraft: craftRelease("0.21.1"),
  figcraft: craftRelease("0.18.1"),
  texturecraft: craftRelease("0.6.1"),
  materialcraft: craftRelease("0.4.1"),
  wordcraft: craftRelease("0.19.1"),
  soundcraft: craftRelease("0.20.1"),
  deckcraft: craftRelease("0.19.1"),
  cadcraft: craftRelease("0.20.1"),
  gridcraft: craftRelease("0.19.1"),
  "craft-anything": craftRelease("0.6.1"),
};

export type CraftNightlyRelease = {
  /** Exact GitHub release tag of the latest nightly prerelease (any tag format). */
  tag: string;
  /** Package version used in the `-nightly-<version>-linux-<arch>` filenames. */
  version: string;
  /** True when the nightly prerelease publishes Flatpak OCI images (`nightly-<arch>`). */
  flatpak?: boolean;
  /** True when the nightly prerelease publishes AppImage binaries (`-nightly-<version>-linux-<arch>.AppImage`). */
  appImage?: boolean;
};

// Updated automatically by `.github/workflows/update-craft-release-versions.yml` from the
// newest published nightly prerelease per repository, regardless of its tag name.
export const CRAFT_NIGHTLY_RELEASES: Partial<
  Record<CraftAppSlug | "artcraft-launcher" | "artcraft", CraftNightlyRelease>
> = {};

/** Published immutable developer release; asset metadata is captured from the exact GitHub release. */
export type CraftDeveloperRelease = {
  tag: string;
  version: string;
  publishedAt: string;
  /** When the updater successfully enumerated releases from GitHub. */
  reconciledAt: string;
  assets: { name: string; size: number; sha256?: string }[];
};

// Unlike CRAFT_NIGHTLY_RELEASES (which tracks distro packages), this also tracks portable
// Windows, macOS and Linux builds. Updated by the same release reconciliation.
export const CRAFT_DEVELOPER_RELEASES: Partial<
  Record<CraftAppSlug | "artcraft-launcher" | "artcraft", CraftDeveloperRelease>
> = {
  "cadcraft": {
    "tag": "v0.20.1-nightly.20261127.46074241695.1.gf7adf3ad561c",
    "version": "0.20.1-nightly.20261127.46074241695.1.gf7adf3ad561c",
    "publishedAt": "2026-11-27T22:48:29Z",
    "reconciledAt": "2026-11-28T14:11:40.145Z",
    "assets": [
      {
        "name": "cadcraft-0.20.1-nightly.20261127.46074241695.1.gf7adf3ad561c-linux-x86_64-nightly.tar.gz",
        "size": 59581116,
        "sha256": "259683ed5e3375b7adbf2e70318d8b2247a845262967f0b56e6962439f026e44"
      },
      {
        "name": "cadcraft-0.20.1-nightly.20261127.46074241695.1.gf7adf3ad561c-linux-x86_64-nightly.tar.gz.asc",
        "size": 228,
        "sha256": "0f8a90a8aa3fd08ce438a374d27b8c73f02a3972014a20957005726fdcf55564"
      },
      {
        "name": "cadcraft-0.20.1-nightly.20261127.46074241695.1.gf7adf3ad561c-macos-aarch64-nightly.zip",
        "size": 53252336,
        "sha256": "13a2344f2de2a09b50aaf22c498b3ef103b5d61fdc315d5dc7b74eba7558a239"
      },
      {
        "name": "cadcraft-0.20.1-nightly.20261127.46074241695.1.gf7adf3ad561c-windows-x86_64-nightly.zip",
        "size": 53612594,
        "sha256": "5eda8ce2851e308ce6ba2cda50820e8ef32af1f80bb70daf301079ee878fb5c8"
      },
      {
        "name": "NIGHTLY.json",
        "size": 2495,
        "sha256": "28414a91a8ff492a83c2611009347cf9e048e3387d734f6f09a7cc7cfbcb335f"
      },
      {
        "name": "SHA256SUMS.txt",
        "size": 705,
        "sha256": "f4ffa36b700d8401f2367514cb74e8ba4b11f1c75fd791a12781cecf50119a70"
      }
    ]
  },
  "gridcraft": {
    "tag": "v0.19.1-nightly.20261127.46047957334.1.g97f2754c1ddf",
    "version": "0.19.1-nightly.20261127.46047957334.1.g97f2754c1ddf",
    "publishedAt": "2026-11-27T18:58:43Z",
    "reconciledAt": "2026-11-28T14:11:40.715Z",
    "assets": [
      {
        "name": "gridcraft-0.19.1-nightly.20261127.46047957334.1.g97f2754c1ddf-linux-x86_64-nightly.tar.gz",
        "size": 56272624,
        "sha256": "e076ee5e1204cc2de477372be2f39b34a1e00b19559763b89a52830de7152727"
      },
      {
        "name": "gridcraft-0.19.1-nightly.20261127.46047957334.1.g97f2754c1ddf-linux-x86_64-nightly.tar.gz.asc",
        "size": 228,
        "sha256": "7fcbfcc9cb426a9bfc046119a6eb0cac383136aecacf0078bb8cebbefeb70243"
      },
      {
        "name": "gridcraft-0.19.1-nightly.20261127.46047957334.1.g97f2754c1ddf-macos-aarch64-nightly.zip",
        "size": 51450231,
        "sha256": "87f8ca89440b9ee6ea8f2c71c292a82543b39ee961bc0bdc8f707cc029c5d706"
      },
      {
        "name": "gridcraft-0.19.1-nightly.20261127.46047957334.1.g97f2754c1ddf-windows-x86_64-nightly.zip",
        "size": 51381701,
        "sha256": "d80d0f5ba62c18750207ee52f367fdcf483f594c4ee958515df39187cae5a1d7"
      },
      {
        "name": "NIGHTLY.json",
        "size": 2512,
        "sha256": "5247ce9a85f762e8b2f52885674adda6e6b92f5dfb16d57aab206d7b62f72079"
      },
      {
        "name": "SHA256SUMS.txt",
        "size": 709,
        "sha256": "37397dfa33f1af56e848bc9e7cb4dee27841092b5c672a6ab215adc3fc4584d5"
      }
    ]
  }
};

export type CraftChannelRelease = {
  /** Exact GitHub release tag (e.g. "v0.24.0-rc.1" or "v0.23.1"). */
  tag: string;
  /** Version without the leading "v" (e.g. "0.24.0-rc.1" or "0.23.1"). */
  version: string;
};

// Highest-version non-nightly beta/RC prerelease per repository, updated automatically by
// `.github/workflows/update-craft-release-versions.yml`.
export const CRAFT_PRERELEASE_RELEASES: Partial<
  Record<CraftAppSlug | "artcraft-launcher", CraftChannelRelease>
> = {
  photocraft: { tag: "v0.24.0-rc.2", version: "0.24.0-rc.2" },
};

export type CraftCommunityAppReleases = {
  /** GitHub repository (`owner/repo`), matching the craft-launcher-assets catalog. */
  repo: string;
  stable?: CraftChannelRelease;
  prerelease?: CraftChannelRelease;
  nightly?: CraftChannelRelease;
};

// Community apps listed in the craft-launcher-assets catalog, updated automatically by
// `.github/workflows/update-craft-release-versions.yml`.
export const CRAFT_COMMUNITY_RELEASES: Record<string, CraftCommunityAppReleases> = {
  solidcraft: { repo: "tangency-labs/solidcraft", stable: { tag: "v0.51.2", version: "0.51.2" }, prerelease: { tag: "v0.51.0", version: "0.51.0" }, nightly: { tag: "v0.51.0-nightly.20261127.168.1", version: "0.51.0-nightly.20261127.168.1" } },
  mailcraft: { repo: "UCSLabs/mailcraft", stable: { tag: "v0.15.0", version: "0.15.0" } },
};
