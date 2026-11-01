use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Query parameters for downloading a Craft App release asset.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema, IntoParams)]
pub struct CraftAppDownloadQuery {
  /// Target CPU architecture, e.g. "x86_64", "x64", "aarch64", "arm64", "universal".
  pub arch: Option<String>,
  /// Target operating system, e.g. "linux", "windows", "macos", "freebsd", "web".
  pub os: Option<String>,
}

/// Error response returned when an application or asset cannot be found.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CraftAppDownloadErrorResponse {
  pub success: bool,
  pub error: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub available_assets: Option<Vec<String>>,
}
