//! How storyteller-web polls GitHub for Craft App releases, from the environment.
//!
//! | Variable                                  | Default                     | Meaning                                                       |
//! |-------------------------------------------|-----------------------------|---------------------------------------------------------------|
//! | `CRAFT_APPS_RELEASE_POLLING_ENABLED`      | `true`                      | Poll at all (the endpoint still answers)                      |
//! | `CRAFT_APPS_RELEASE_COORDINATION_ENABLED` | `true`                      | Share releases and backoff through Redis (see below)          |
//! | `CRAFT_APPS_RELEASE_POLL_INTERVAL_SECS`   | `120`; `1800` with no token | How old a release may get; at least `60`; `300` with no token |
//! | `CRAFT_APPS_RELEASE_MAX_BACKOFF_SECS`     | `14400` (4 h)               | Longest wait after repeated failures/rate limits              |
//! | `CRAFT_APPS_RELEASE_GITHUB_TOKEN`         | (none; optional)            | Raises GitHub's limit from 60 to 5000 per hour                |
//! | `CRAFT_APPS_RELEASE_APPS`                 | built-in list               | `key=Name=org/repo,…` to replace the app list                 |
//!
//! The GitHub token is optional. Unset, empty or blank, polling still runs: requests go out
//! without an `Authorization` header, and the interval defaults to the slow no-token value.
//!
//! With coordination (the default), the fleet shares each app's release, ETag and fetch time in
//! Redis, so the whole fleet requests an app from GitHub about once per interval instead of every
//! pod doing so. That is what makes the 2 minute default safe with a token. Without a token
//! GitHub allows 60 requests an hour per IP address and also counts `304`s, so the interval stays
//! at 30 minutes (13 apps: about 26 requests an hour). While Redis is unreachable (or with
//! coordination off) every pod polls on its own at the same interval.

use std::time::Duration;

const DEFAULT_INTERVAL_SECS_WITH_TOKEN: u64 = 120;
const MIN_INTERVAL_SECS_WITH_TOKEN: u64 = 60;
const DEFAULT_INTERVAL_SECS_WITHOUT_TOKEN: u64 = 1800;
const MIN_INTERVAL_SECS_WITHOUT_TOKEN: u64 = 300;
const DEFAULT_MAX_BACKOFF_SECS: u64 = 14_400;

/// The Craft Apps the launcher knows: `(app key, display name, GitHub repo)`. The app key is
/// the app's lowercase machine name; it keys `CraftAppsReleaseCache` and the endpoint's
/// `release_info.apps`, and must match the app id ArtCraft Launcher uses. ArtCraft Launcher is
/// included (key `artcraft-launcher`, repo `storytold/craft-launcher`) so it can check for its
/// own updates.
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
  /// Share releases and backoff with the fleet through Redis; when false (or while Redis is
  /// unreachable) this pod polls on its own.
  pub coordination_enabled: bool,
  /// How old an app's release may get before it is requested from GitHub again.
  pub interval: Duration,
  pub max_backoff: Duration,
  /// Never empty or blank: those are read as no token.
  pub github_token: Option<String>,
  /// `(app key, display name, repo)`; see [`DEFAULT_APPS`].
  pub apps: Vec<(String, String, String)>,
}

impl CraftAppsPollConfig {
  pub fn from_env() -> Self {
    let github_token = non_blank_token(easyenv::get_env_string_optional("CRAFT_APPS_RELEASE_GITHUB_TOKEN"));
    let has_token = github_token.is_some();
    let default_interval = default_interval_secs(has_token);
    let requested_interval = easyenv::get_env_num::<u64>("CRAFT_APPS_RELEASE_POLL_INTERVAL_SECS", default_interval)
        .unwrap_or(default_interval);
    let interval_secs = clamp_interval_secs(requested_interval, has_token);
    let max_backoff_secs = easyenv::get_env_num::<u64>("CRAFT_APPS_RELEASE_MAX_BACKOFF_SECS", DEFAULT_MAX_BACKOFF_SECS)
        .unwrap_or(DEFAULT_MAX_BACKOFF_SECS)
        .max(interval_secs);
    let apps = easyenv::get_env_string_optional("CRAFT_APPS_RELEASE_APPS")
        .map(|v| parse_app_list(&v))
        .filter(|apps| !apps.is_empty())
        .unwrap_or_else(default_apps);
    Self {
      enabled: easyenv::get_env_bool_or_default("CRAFT_APPS_RELEASE_POLLING_ENABLED", true),
      coordination_enabled: easyenv::get_env_bool_or_default("CRAFT_APPS_RELEASE_COORDINATION_ENABLED", true),
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

/// The token, trimmed; `None` when unset, empty or blank.
fn non_blank_token(value: Option<String>) -> Option<String> {
  value.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

fn default_interval_secs(has_token: bool) -> u64 {
  if has_token { DEFAULT_INTERVAL_SECS_WITH_TOKEN } else { DEFAULT_INTERVAL_SECS_WITHOUT_TOKEN }
}

fn clamp_interval_secs(requested: u64, has_token: bool) -> u64 {
  let min = if has_token { MIN_INTERVAL_SECS_WITH_TOKEN } else { MIN_INTERVAL_SECS_WITHOUT_TOKEN };
  requested.max(min)
}

/// Reads `key=Name=org/repo,…` (app key, display name, repo). Keys are lowercased. Malformed
/// entries are skipped.
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

  #[test]
  fn a_missing_empty_or_blank_token_is_no_token() {
    assert_eq!(non_blank_token(None), None);
    assert_eq!(non_blank_token(Some(String::new())), None);
    assert_eq!(non_blank_token(Some("  \n".to_string())), None);
    assert_eq!(non_blank_token(Some(" ghp_x ".to_string())).as_deref(), Some("ghp_x"));
  }

  #[test]
  fn the_interval_defaults_and_minimum_depend_on_the_token() {
    assert_eq!(default_interval_secs(true), 120);
    assert_eq!(default_interval_secs(false), 1800);
    assert_eq!(clamp_interval_secs(10, true), 60);
    assert_eq!(clamp_interval_secs(10, false), 300);
    assert_eq!(clamp_interval_secs(90, true), 90);
    assert_eq!(clamp_interval_secs(90, false), 300);
    assert_eq!(clamp_interval_secs(3600, false), 3600);
  }
}
