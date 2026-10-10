//! How storyteller-web polls GitHub for Craft App releases, from the environment.
//!
//! | Variable                                  | Default        | Meaning                                         |
//! |-------------------------------------------|----------------|-------------------------------------------------|
//! | `CRAFT_APPS_RELEASE_POLLING_ENABLED`      | `true`         | Poll at all (the endpoint still answers)         |
//! | `CRAFT_APPS_RELEASE_POLL_INTERVAL_SECS`   | `1800` (30 m)  | Time between rounds; at least 300                |
//! | `CRAFT_APPS_RELEASE_MAX_BACKOFF_SECS`     | `14400` (4 h)  | Longest wait after repeated failures/rate limits |
//! | `CRAFT_APPS_RELEASE_GITHUB_TOKEN`         | (none)         | Raises GitHub's limit from 60 to 5000 per hour   |
//! | `CRAFT_APPS_RELEASE_APPS`                 | built-in list  | `key=Name=org/repo,…` to replace the app list    |
//!
//! Every instance in the fleet polls on its own, so the defaults are deliberately slow: one round
//! is one request per app, and without a token GitHub allows 60 an hour per IP address.

use std::time::Duration;

const MIN_INTERVAL_SECS: u64 = 300;

/// The Craft Apps the launcher knows: (lowercase key, display name, GitHub repo). ArtCraft
/// Launcher is included so it can check for its own updates.
const DEFAULT_APPS: &[(&str, &str, &str)] = &[
  ("photocraft", "PhotoCraft", "storytold/photocraft"),
  ("vectorcraft", "VectorCraft", "storytold/vectorcraft"),
  ("filmcraft", "FilmCraft", "storytold/filmcraft"),
  ("lightcraft", "LightCraft", "storytold/lightcraft"),
  ("pdfcraft", "PdfCraft", "storytold/pdfcraft"),
  ("effectcraft", "EffectCraft", "storytold/effectcraft"),
  ("designcraft", "DesignCraft", "storytold/designcraft"),
  ("wordcraft", "WordCraft", "storytold/wordcraft"),
  ("deckcraft", "DeckCraft", "storytold/deckcraft"),
  ("gridcraft", "GridCraft", "storytold/gridcraft"),
  ("cadcraft", "CADCraft", "storytold/cadcraft"),
  ("soundcraft", "SoundCraft", "storytold/soundcraft"),
  ("artcraft-launcher", "ArtCraft Launcher", "storytold/craft-launcher"),
];

#[derive(Clone, Debug)]
pub struct CraftAppsPollConfig {
  pub enabled: bool,
  pub interval: Duration,
  pub max_backoff: Duration,
  pub github_token: Option<String>,
  /// (lowercase key, display name, repo).
  pub apps: Vec<(String, String, String)>,
}

impl CraftAppsPollConfig {
  pub fn from_env() -> Self {
    let interval_secs = easyenv::get_env_num::<u64>("CRAFT_APPS_RELEASE_POLL_INTERVAL_SECS", 1800).unwrap_or(1800).max(MIN_INTERVAL_SECS);
    let max_backoff_secs = easyenv::get_env_num::<u64>("CRAFT_APPS_RELEASE_MAX_BACKOFF_SECS", 14_400).unwrap_or(14_400).max(interval_secs);
    let github_token = easyenv::get_env_string_optional("CRAFT_APPS_RELEASE_GITHUB_TOKEN")
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    let apps = easyenv::get_env_string_optional("CRAFT_APPS_RELEASE_APPS")
        .map(|v| parse_app_list(&v))
        .filter(|apps| !apps.is_empty())
        .unwrap_or_else(default_apps);
    Self {
      enabled: easyenv::get_env_bool_or_default("CRAFT_APPS_RELEASE_POLLING_ENABLED", true),
      interval: Duration::from_secs(interval_secs),
      max_backoff: Duration::from_secs(max_backoff_secs),
      github_token,
      apps,
    }
  }
}

pub fn default_apps() -> Vec<(String, String, String)> {
  DEFAULT_APPS.iter().map(|(k, n, r)| (k.to_string(), n.to_string(), r.to_string())).collect()
}

/// Reads `key=Name=org/repo,…`. Malformed entries are skipped.
fn parse_app_list(value: &str) -> Vec<(String, String, String)> {
  value
      .split(',')
      .filter_map(|entry| {
        let mut parts = entry.trim().splitn(3, '=');
        let key = parts.next()?.trim().to_lowercase();
        let name = parts.next()?.trim().to_string();
        let repo = parts.next()?.trim().to_string();
        let valid_repo = repo.split('/').count() == 2 && !repo.starts_with('/') && !repo.ends_with('/');
        (!key.is_empty() && !name.is_empty() && valid_repo).then_some((key, name, repo))
      })
      .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn reads_an_app_list_and_skips_bad_entries() {
    let apps = parse_app_list(" PhotoCraft=PhotoCraft=storytold/photocraft, junk, x=X=noslash, k=K=org/repo ");
    assert_eq!(apps, vec![
      ("photocraft".to_string(), "PhotoCraft".to_string(), "storytold/photocraft".to_string()),
      ("k".to_string(), "K".to_string(), "org/repo".to_string()),
    ]);
  }

  #[test]
  fn the_default_list_has_the_suite_and_the_launcher() {
    let apps = default_apps();
    assert_eq!(apps.len(), 13);
    assert!(apps.iter().any(|(k, _, r)| k == "artcraft-launcher" && r == "storytold/craft-launcher"));
  }
}
