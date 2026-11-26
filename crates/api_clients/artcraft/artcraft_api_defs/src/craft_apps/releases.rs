use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::craft_apps::release_info::CraftAppRelease;

// ── GET /v1/craft_apps/{app_key}/releases ──
//
// One Craft App's release history: every published release storyteller-web read from GitHub,
// newest first, nightlies included, each with its files and checksums. ArtCraft Launcher reads
// it to choose or go back to a version and to find nightlies; `release_info` carries only the
// latest of each kind, for every app at once.
//
// Compatibility: clients must ignore fields they don't know, and treat every field as optional.
// New fields may be added; existing ones keep their meaning.

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GetCraftAppReleasesResponse {
  pub success: bool,

  /// The app's lowercase machine name (`photocraft`), as in `release_info.apps`.
  pub app_key: String,

  /// Display name (`PhotoCraft`).
  pub name: String,

  /// GitHub repository, `organization/project` (`storytold/photocraft`).
  pub repo: String,

  /// When this app's releases were last fetched successfully. Absent if they never were; then
  /// `releases` says nothing.
  pub app_last_fetched: Option<DateTime<Utc>>,

  /// Whether `releases` is every release the repository has. When false, older ones exist that
  /// aren't listed (storyteller-web reads at most the newest 1000).
  pub complete: bool,

  /// Newest first (as GitHub lists them). Release notes are truncated to 64 KiB.
  pub releases: Vec<CraftAppRelease>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GetCraftAppReleasesError {
  pub success: bool,
  pub error_reason: String,
}
