use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use url::Url;
use utoipa::ToSchema;

// ── GET /v1/craft_apps/release_info ──
//
// The latest release of every Craft App (PhotoCraft, VectorCraft, …, and ArtCraft Launcher
// itself), as storyteller-web last fetched it from GitHub, plus renames and notices for the
// ArtCraft Launcher. The launcher polls this instead of GitHub; GitHub is its fallback.
//
// Compatibility: clients must ignore fields they don't know, and treat every field as optional.
// New fields may be added; existing ones keep their meaning.

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GetCraftAppsReleaseInfoResponse {
  pub success: bool,

  pub release_info: CraftAppsReleaseInfo,

  /// Apps that changed their name (for example PdfCraft → DocuCraft).
  pub app_renames: Vec<CraftAppRename>,

  /// Apps whose GitHub repository moved (for example `storytold/photocraft` → `art/photocraft`).
  pub repo_renames: Vec<CraftAppRepoRename>,

  pub important_notices: CraftAppsImportantNotices,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct CraftAppsReleaseInfo {
  /// Keyed by the app's lowercase name (`photocraft`, `artcraft-launcher`).
  pub apps: BTreeMap<String, CraftAppReleaseInfo>,

  /// When the least recently fetched app was fetched: every app's data is at least this fresh.
  /// Absent until every app has been fetched at least once.
  pub last_fetched: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CraftAppReleaseInfo {
  /// Display name (`PhotoCraft`).
  pub name: String,

  /// GitHub repository, `organization/project` (`storytold/photocraft`).
  pub repo: String,

  /// The latest published, non-draft, non-prerelease release. Absent when the repository has no
  /// release yet, or when it hasn't been fetched yet (see `app_last_fetched`).
  pub latest_release: Option<CraftAppRelease>,

  /// When this app's release data was last fetched successfully. Absent if it never was; then
  /// `latest_release` says nothing and clients should ask GitHub themselves.
  pub app_last_fetched: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CraftAppRelease {
  /// Git tag (`v0.5.0`).
  pub tag: String,

  /// The tag without its leading `v` (`0.5.0`, `0.6.0-rc.1`).
  pub version: String,

  /// Release title (`PhotoCraft v0.5.0`).
  pub name: String,

  pub published_at: Option<DateTime<Utc>>,

  /// The release's page on GitHub.
  pub html_url: String,

  pub prerelease: bool,

  /// Release notes (Markdown), truncated to 64 KiB.
  pub notes_markdown: String,

  pub assets: Vec<CraftAppReleaseAsset>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CraftAppReleaseAsset {
  /// File name (`photocraft-0.5.0-windows-x64.msi`).
  pub name: String,

  pub size_bytes: u64,

  /// `https://github.com/<repo>/releases/download/<tag>/<name>`.
  pub download_url: String,

  /// Lowercase hex SHA-256, when GitHub published one for the file.
  pub sha256: Option<String>,

  /// What the file name says it is, when it follows the Craft App naming scheme
  /// (`<app>-<version>-<os>-<arch>[-portable].<ext>`).
  pub platform: Option<CraftAppAssetPlatform>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CraftAppAssetPlatform {
  pub os: CraftAppOs,

  /// Absent for files that aren't for one CPU (the web build, checksums).
  pub arch: Option<CraftAppArch>,

  pub package_type: CraftAppPackageType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CraftAppOs {
  Windows,
  Macos,
  Linux,
  Freebsd,
  /// The browser build.
  Web,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CraftAppArch {
  X64,
  X86,
  Arm64,
  /// macOS universal binary (Apple silicon and Intel).
  Universal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CraftAppPackageType {
  /// Windows Installer.
  Msi,
  /// Windows portable zip.
  PortableZip,
  /// macOS disk image.
  Dmg,
  /// macOS command-line tools zip.
  CliZip,
  AppImage,
  /// AppImage delta-update file.
  AppImageZsync,
  Deb,
  Rpm,
  Flatpak,
  /// Linux or FreeBSD tarball.
  TarGz,
  /// The web (WebAssembly) build.
  WebZip,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CraftAppRename {
  /// User-facing name before (`PdfCraft`).
  pub old_name: String,
  /// Lowercase name before (`pdfcraft`).
  pub old_name_lower: String,
  /// User-facing name after (`DocuCraft`).
  pub new_name: String,
  /// Lowercase name after (`docucraft`).
  pub new_name_lower: String,
  /// A sentence or two to show beside the rename.
  pub description: String,
  pub renamed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CraftAppRepoRename {
  /// `organization/project` before.
  pub old_name: String,
  /// `organization/project` after.
  pub new_name: String,
  pub renamed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct CraftAppsImportantNotices {
  /// News, shown inline; the user can dismiss each.
  pub announcements: Vec<CraftAppsAnnouncement>,
  /// Shown as a popup the user acknowledges.
  pub alerts: Vec<CraftAppsAlert>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CraftAppsAnnouncement {
  pub title: String,
  pub body: String,
  pub created_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CraftAppsAlert {
  pub title: String,
  pub body: String,
  /// A page the popup can offer to open.
  pub maybe_web_link: Option<Url>,
  pub created_at: Option<DateTime<Utc>>,
}
