//! Flatpak repository configuration and .flatpakrepo metadata definitions for ArtCraft apps.
//!
//! Provides structured models and `.flatpakrepo` file generation for both the overarching
//! ArtCraft Suite repository and individual Craft application remotes.

use serde_derive::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const DEFAULT_FLATPAK_BRANCH: &str = "stable";
pub const DEFAULT_FLATPAK_ID_PREFIX: &str = "ai.storyteller.";

/// Configuration defining a Flatpak OSTree repository remote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CraftAppFlatpakRepoConfig {
  /// The app key (e.g. `photocraft`), or `artcraft` for the complete suite repository.
  pub app_key: String,

  /// Flatpak Application ID (e.g. `ai.storyteller.photocraft`).
  pub app_id: String,

  /// Display title for the repository remote (e.g. `PhotoCraft` or `ArtCraft`).
  pub title: String,

  /// Short comment/summary describing the remote.
  pub comment: String,

  /// Longer description of the repository.
  pub description: String,

  /// URL of the OSTree repository (where `summary` and `objects/` are fetched).
  pub repo_url: String,

  /// Public homepage URL.
  pub homepage: String,

  /// Default OSTree branch (usually `stable`).
  pub default_branch: String,

  /// Optional GPG verification key URL or base64 key data.
  pub gpg_key_url: Option<String>,
}

impl CraftAppFlatpakRepoConfig {
  /// Generates the standard INI-formatted `.flatpakrepo` file content parsed by Flatpak.
  pub fn to_flatpakrepo_content(&self) -> String {
    let mut out = String::with_capacity(512);
    out.push_str("[Flatpak Repo]\n");
    out.push_str(&format!("Title={}\n", self.title));
    out.push_str(&format!("Url={}\n", self.repo_url));
    out.push_str(&format!("Homepage={}\n", self.homepage));
    out.push_str(&format!("Comment={}\n", self.comment));
    out.push_str(&format!("Description={}\n", self.description));
    out.push_str(&format!("DefaultBranch={}\n", self.default_branch));
    if let Some(ref gpg_key) = self.gpg_key_url {
      out.push_str(&format!("GPGKey={}\n", gpg_key));
    }
    out
  }

  /// Builds a repository configuration for the complete ArtCraft suite.
  pub fn suite_config(repo_url: &str, homepage: &str, gpg_key_url: Option<String>) -> Self {
    Self { app_key: "artcraft".to_string(), app_id: "ai.storyteller.artcraft".to_string(), title: "ArtCraft".to_string(), comment: "Official ArtCraft Suite releases".to_string(), description: "Official Flatpak repository for ArtCraft suite applications".to_string(), repo_url: repo_url.trim_end_matches('/').to_string() + "/", homepage: homepage.to_string(), default_branch: DEFAULT_FLATPAK_BRANCH.to_string(), gpg_key_url }
  }

  /// Builds a repository configuration for a single Craft application.
  pub fn app_config(app_key: &str, display_name: &str, repo_url: &str, homepage: &str, gpg_key_url: Option<String>) -> Self {
    let clean_key = app_key.trim().to_lowercase();
    let app_id = format!("{DEFAULT_FLATPAK_ID_PREFIX}{clean_key}");
    Self { app_key: clean_key.clone(), app_id, title: display_name.to_string(), comment: format!("Official {display_name} releases"), description: format!("Official Flatpak repository for {display_name}"), repo_url: repo_url.trim_end_matches('/').to_string() + "/", homepage: homepage.to_string(), default_branch: DEFAULT_FLATPAK_BRANCH.to_string(), gpg_key_url }
  }
}

use std::cmp::Ordering;

/// Metadata advertising Flatpak repository availability in release info responses.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CraftAppFlatpakRepoInfo {
  /// Flatpak Application ID (e.g. `ai.storyteller.photocraft`).
  pub app_id: String,

  /// Primary repository URL to add as a remote.
  pub repo_url: String,

  /// Direct URL to download the `.flatpakrepo` configuration file.
  pub flatpakrepo_url: String,

  /// Default tracking branch (e.g. `stable`).
  pub branch: String,

  /// Version of the application currently available in this Flatpak repository.
  ///
  /// Clients and launchers must verify that this version is strictly greater than
  /// the installed version before triggering repo-based updates (`flatpak update`)
  /// or switching remotes, preventing accidental downgrades when the Flatpak repository
  /// lags behind GitHub releases or installed packages.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,

  /// Minimum installed version required before attempting a repository update or migration.
  /// If the client has an installed version below `min_version`, or if the repository
  /// version is older than what is currently installed, the client must skip the update.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub min_version: Option<String>,

  /// Whether the Flatpak repository is verified to be up-to-date with `latest_release`.
  /// When `false` or absent while repository version is older, clients should not downgrade.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub is_synced: Option<bool>,

  /// Direct URL to download the repository GPG verification key.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub gpg_key_url: Option<String>,
}

impl CraftAppFlatpakRepoInfo {
  /// Checks whether updating from `installed_version` to this repository would cause
  /// an accidental downgrade.
  ///
  /// Returns `true` if:
  /// - `version` is known and strictly older than `installed_version`.
  /// - `min_version` is specified and `installed_version` is strictly older than `min_version`.
  ///
  /// Returns `false` if `version` is newer than or equal to `installed_version`,
  /// or if versions cannot be compared.
  pub fn would_downgrade(&self, installed_version: &str) -> bool {
    if let Some(ref min_v) = self.min_version {
      if let Some(ord) = compare_versions(installed_version, min_v) {
        if ord == Ordering::Less {
          return true;
        }
      }
    }

    if let Some(ref repo_v) = self.version {
      if let Some(ord) = compare_versions(repo_v, installed_version) {
        if ord == Ordering::Less {
          return true;
        }
      }
    }

    false
  }

  /// Returns `true` if this repository's version is strictly newer than `installed_version`.
  pub fn is_newer_than(&self, installed_version: &str) -> bool {
    let Some(ref repo_v) = self.version else {
      return false;
    };
    compare_versions(repo_v, installed_version) == Some(Ordering::Greater)
  }
}

/// Parsed representation of a version triplet `(major, minor, patch, Option<prerelease>)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimpleSemver {
  pub major: u64,
  pub minor: u64,
  pub patch: u64,
  pub prerelease: Option<String>,
}

impl SimpleSemver {
  /// Parses a version string like "v1.2.3", "0.5.0-rc.1", "0.4.1", "1.2".
  pub fn parse(text: &str) -> Option<Self> {
    let t = text.trim();
    let t = t.strip_prefix(['v', 'V']).unwrap_or(t);
    if t.is_empty() {
      return None;
    }
    let (core, pre) = match t.find(['-', '+']) {
      Some(idx) => (&t[..idx], Some(t[idx + 1..].to_string())),
      None => (t, None),
    };
    let parts: Vec<&str> = core.split('.').collect();
    let major = parts.first()?.parse::<u64>().ok()?;
    let minor = parts.get(1).map_or(Ok(0), |p| p.parse::<u64>()).ok()?;
    let patch = parts.get(2).map_or(Ok(0), |p| p.parse::<u64>()).ok()?;
    Some(Self { major, minor, patch, prerelease: pre })
  }

  /// Compares two versions following SemVer precedence rules:
  /// 1. Compare major, minor, patch numerically.
  /// 2. If equal, a version with a prerelease is *older* than a version without one.
  /// 3. If both have a prerelease, compare prerelease strings lexicographically.
  pub fn compare(&self, other: &Self) -> Ordering {
    match self.major.cmp(&other.major) {
      Ordering::Equal => {},
      non_eq => return non_eq,
    }
    match self.minor.cmp(&other.minor) {
      Ordering::Equal => {},
      non_eq => return non_eq,
    }
    match self.patch.cmp(&other.patch) {
      Ordering::Equal => {},
      non_eq => return non_eq,
    }
    match (&self.prerelease, &other.prerelease) {
      (None, None) => Ordering::Equal,
      (Some(_), None) => Ordering::Less,
      (None, Some(_)) => Ordering::Greater,
      (Some(a), Some(b)) => a.cmp(b),
    }
  }
}

/// Compares version string `a` to `b`.
/// Returns `Some(Ordering)` or `None` if either string cannot be parsed.
pub fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
  let v_a = SimpleSemver::parse(a)?;
  let v_b = SimpleSemver::parse(b)?;
  Some(v_a.compare(&v_b))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn serializes_suite_flatpakrepo() {
    let config = CraftAppFlatpakRepoConfig::suite_config("https://repo.getartcraft.com", "https://getartcraft.com", Some("https://getartcraft.com/keys/flatpak.gpg".to_string()));
    let content = config.to_flatpakrepo_content();
    assert!(content.contains("[Flatpak Repo]"));
    assert!(content.contains("Title=ArtCraft"));
    assert!(content.contains("Url=https://repo.getartcraft.com/"));
    assert!(content.contains("Homepage=https://getartcraft.com"));
    assert!(content.contains("DefaultBranch=stable"));
    assert!(content.contains("GPGKey=https://getartcraft.com/keys/flatpak.gpg"));
  }

  #[test]
  fn serializes_app_flatpakrepo_with_gpg() {
    let config = CraftAppFlatpakRepoConfig::app_config("photocraft", "PhotoCraft", "https://repo.getartcraft.com/photocraft", "https://getartcraft.com/apps/photocraft", Some("https://getartcraft.com/keys/flatpak.gpg".to_string()));
    let content = config.to_flatpakrepo_content();
    assert!(content.contains("Title=PhotoCraft"));
    assert!(content.contains("Url=https://repo.getartcraft.com/photocraft/"));
    assert!(content.contains("GPGKey=https://getartcraft.com/keys/flatpak.gpg"));
    assert_eq!(config.app_id, "ai.storyteller.photocraft");
  }

  #[test]
  fn semver_comparison_and_downgrade_prevention() {
    assert_eq!(compare_versions("0.5.0", "0.4.0"), Some(Ordering::Greater));
    assert_eq!(compare_versions("0.4.0", "0.5.0"), Some(Ordering::Less));
    assert_eq!(compare_versions("v0.5.0", "0.5.0"), Some(Ordering::Equal));
    assert_eq!(compare_versions("0.5.0", "0.5.0-rc.1"), Some(Ordering::Greater));
    assert_eq!(compare_versions("0.5.0-rc.1", "0.5.0"), Some(Ordering::Less));

    let repo_lagging = CraftAppFlatpakRepoInfo {
      app_id: "ai.storyteller.photocraft".to_string(),
      repo_url: "https://repo.getartcraft.com/photocraft/".to_string(),
      flatpakrepo_url: "https://api.getartcraft.com/v1/craft_apps/flatpak/photocraft.flatpakrepo".to_string(),
      branch: "stable".to_string(),
      version: Some("0.4.0".to_string()),
      min_version: None,
      is_synced: Some(false),
      gpg_key_url: Some("https://getartcraft.com/keys/flatpak.gpg".to_string()),
    };

    // If installed is 0.5.0 and repo is 0.4.0, update would downgrade:
    assert!(repo_lagging.would_downgrade("0.5.0"));
    assert!(!repo_lagging.is_newer_than("0.5.0"));

    // If installed is 0.4.0, repo is equal, not a downgrade:
    assert!(!repo_lagging.would_downgrade("0.4.0"));
    assert!(!repo_lagging.is_newer_than("0.4.0"));

    // If installed is 0.3.0, repo is newer:
    assert!(!repo_lagging.would_downgrade("0.3.0"));
    assert!(repo_lagging.is_newer_than("0.3.0"));

    // Testing min_version constraint:
    let repo_with_min = CraftAppFlatpakRepoInfo {
      app_id: "ai.storyteller.cadcraft".to_string(),
      repo_url: "https://repo.getartcraft.com/cadcraft/".to_string(),
      flatpakrepo_url: "https://api.getartcraft.com/v1/craft_apps/flatpak/cadcraft.flatpakrepo".to_string(),
      branch: "stable".to_string(),
      version: Some("0.6.0".to_string()),
      min_version: Some("0.5.0".to_string()),
      is_synced: Some(true),
      gpg_key_url: Some("https://getartcraft.com/keys/flatpak.gpg".to_string()),
    };
    // Installed is 0.4.0, which is below min_version 0.5.0:
    assert!(repo_with_min.would_downgrade("0.4.0"));
    // Installed is 0.5.0, meets min_version and 0.6.0 is newer:
    assert!(!repo_with_min.would_downgrade("0.5.0"));
    assert!(repo_with_min.is_newer_than("0.5.0"));
  }
}

