//! The latest release of every Craft App, as last fetched from GitHub by
//! `poll_craft_app_releases_thread` (directly, or from the fleet's shared copy in Redis), served
//! by `GET /v1/craft_apps/release_info`. The endpoint always answers from this memory, so trouble
//! with Redis or GitHub never breaks it.
//!
//! ## App keys
//!
//! Apps are keyed by their **app key**: the app's lowercase machine name, the same string the
//! Craft Apps use for their binaries, asset file names and `ai.storyteller.<key>` bundle ids.
//! For example `photocraft`, `pdfcraft`, `cadcraft`, and `artcraft-launcher` for ArtCraft Launcher
//! itself. The key is not the display name (`PhotoCraft`) and not the GitHub repo
//! (`storytold/photocraft`, whose project name can differ: the launcher's repo is
//! `storytold/craft-launcher`).
//!
//! The keys come from the polling config (`poll_craft_app_releases_thread::config`: the built-in
//! list, or `CRAFT_APPS_RELEASE_APPS`) and are served unchanged as the keys of
//! `release_info.apps`, which ArtCraft Launcher matches against its own app ids.

use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError, RwLock};

use chrono::{DateTime, Utc};

use artcraft_api_defs::craft_apps::release_info::{CraftAppRelease, CraftAppReleaseInfo, CraftAppsReleaseInfo};

/// Shared between the polling thread and the HTTP handlers. Cheap to clone.
#[derive(Clone, Default)]
pub struct CraftAppsReleaseCache {
  /// App key (lowercase machine name, e.g. `photocraft`; see the module docs) → what's known
  /// about that app. A `BTreeMap` so the endpoint lists apps in a stable order.
  inner: Arc<RwLock<BTreeMap<String, CachedCraftApp>>>,
}

/// One app: what's known about it, and the ETag to revalidate it with.
#[derive(Clone, Debug)]
pub struct CachedCraftApp {
  /// Display name (`PhotoCraft`).
  pub name: String,

  /// GitHub repository, `organization/project` (`storytold/photocraft`).
  pub repo: String,

  /// The latest published release. `None` both before the first fetch and when the repository
  /// has no release; `app_last_fetched` tells the two apart.
  pub latest_release: Option<CraftAppRelease>,

  /// When this app's release was last fetched (or confirmed unchanged) successfully. `None`
  /// until the first success.
  pub app_last_fetched: Option<DateTime<Utc>>,

  /// GitHub's ETag for the last successful answer, sent back as `If-None-Match` so an unchanged
  /// release costs a `304`.
  pub etag: Option<String>,

  /// When a request for this app last failed. Only meaningful when newer than
  /// `app_last_fetched`; it delays the next attempt so a broken repo isn't retried every tick.
  pub last_failed_at: Option<DateTime<Utc>>,

  /// How many requests in a row have failed since the last success (when `last_failed_at` is
  /// newer than `app_last_fetched`).
  pub consecutive_failures: u32,
}

impl CraftAppsReleaseCache {
  pub fn new() -> Self {
    Self::default()
  }

  /// Makes sure every app in `apps` has an entry, so the endpoint reports apps that haven't been
  /// fetched yet. Each tuple is `(app key, display name, repo)`, for example
  /// `("photocraft", "PhotoCraft", "storytold/photocraft")`. Apps already present keep what
  /// was fetched for them.
  pub fn register_apps(&self, apps: &[(String, String, String)]) {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    for (key, name, repo) in apps {
      map.entry(key.clone()).or_insert_with(|| CachedCraftApp {
        name: name.clone(),
        repo: repo.clone(),
        latest_release: None,
        app_last_fetched: None,
        etag: None,
        last_failed_at: None,
        consecutive_failures: 0,
      });
    }
  }

  /// The ETag of the last successful answer for the app with app key `key` (`photocraft`), to
  /// send as `If-None-Match`.
  pub fn etag(&self, key: &str) -> Option<String> {
    let map = self.inner.read().unwrap_or_else(PoisonError::into_inner);
    map.get(key).and_then(|app| app.etag.clone())
  }

  /// Records a complete, successful fetch of the app with app key `key` (`photocraft`): its
  /// latest release (`None` if it has none). Ignored for keys that were never registered.
  pub fn store(&self, key: &str, latest_release: Option<CraftAppRelease>, etag: Option<String>, fetched_at: DateTime<Utc>) {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(app) = map.get_mut(key) {
      app.latest_release = latest_release;
      app.etag = etag;
      app.app_last_fetched = Some(fetched_at);
    }
  }

  /// Records that GitHub confirmed the app with app key `key` (`photocraft`) is unchanged
  /// (`304 Not Modified`). Ignored for keys that were never registered.
  pub fn confirm_unchanged(&self, key: &str, fetched_at: DateTime<Utc>) {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(app) = map.get_mut(key) {
      app.app_last_fetched = Some(fetched_at);
    }
  }

  /// A copy of what's known about the app with app key `key` (`photocraft`).
  pub fn app(&self, key: &str) -> Option<CachedCraftApp> {
    let map = self.inner.read().unwrap_or_else(PoisonError::into_inner);
    map.get(key).cloned()
  }

  /// Takes a fetch made elsewhere (another instance, through Redis) for the app with app key
  /// `key`, if it is newer than what this instance has. Returns whether it was taken. Ignored for
  /// keys that were never registered, so the registered app list stays authoritative.
  pub fn merge_fetched(&self, key: &str, latest_release: Option<CraftAppRelease>, etag: Option<String>, fetched_at: DateTime<Utc>) -> bool {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    let Some(app) = map.get_mut(key) else {
      return false;
    };
    if app.app_last_fetched.is_some_and(|mine| mine >= fetched_at) {
      return false;
    }
    app.latest_release = latest_release;
    app.etag = etag;
    app.app_last_fetched = Some(fetched_at);
    true
  }

  /// Records a failed request for the app with app key `key` (`photocraft`) and returns how many
  /// have failed in a row since its last success. Ignored (returns 0) for unregistered keys.
  pub fn record_failure(&self, key: &str, failed_at: DateTime<Utc>) -> u32 {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    let Some(app) = map.get_mut(key) else {
      return 0;
    };
    app.consecutive_failures = if app.is_failing() { app.consecutive_failures.saturating_add(1) } else { 1 };
    app.last_failed_at = Some(failed_at);
    app.consecutive_failures
  }

  /// Takes a failure recorded elsewhere (another instance, through Redis) for the app with app
  /// key `key`, if it is newer than the one this instance knows.
  pub fn merge_failure(&self, key: &str, failed_at: DateTime<Utc>, consecutive_failures: u32) {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(app) = map.get_mut(key) {
      if app.last_failed_at.is_none_or(|mine| mine < failed_at) {
        app.last_failed_at = Some(failed_at);
        app.consecutive_failures = consecutive_failures;
      }
    }
  }

  /// Returns the canonical app key and cached app info for the specified key or alias, if known.
  pub fn get_app_with_key(&self, key: &str) -> Option<(String, CachedCraftApp)> {
    let map = self.inner.read().unwrap_or_else(PoisonError::into_inner);
    let lower = key.to_lowercase();
    if let Some((k, app)) = map.get_key_value(key).or_else(|| map.get_key_value(&lower)) {
      return Some((k.clone(), app.clone()));
    }
    for (k, app) in map.iter() {
      if k.eq_ignore_ascii_case(key)
        || app.name.eq_ignore_ascii_case(key)
        || app.repo.eq_ignore_ascii_case(key)
        || app.repo.split('/').last().is_some_and(|r| r.eq_ignore_ascii_case(key))
        || (key.eq_ignore_ascii_case("launcher") && k == "artcraft-launcher")
        || (key.eq_ignore_ascii_case("craft-launcher") && k == "artcraft-launcher")
      {
        return Some((k.clone(), app.clone()));
      }
    }
    None
  }

  /// Returns the cached app info for the specified key or alias, if known.
  pub fn get_app(&self, key: &str) -> Option<CachedCraftApp> {
    self.get_app_with_key(key).map(|(_, app)| app)
  }

  /// Returns the latest release for the specified app key (e.g. `photocraft`, `artcraft-launcher`),
  /// if fetched.
  pub fn get_app_release(&self, key: &str) -> Option<CraftAppRelease> {
    self.get_app(key).and_then(|app| app.latest_release)
  }

  /// Helper to load Flatpak config from environment once using registered app keys.
  pub fn load_flatpak_config_from_env(&self) -> CraftAppsFlatpakConfig {
    let map = self.inner.read().unwrap_or_else(PoisonError::into_inner);
    CraftAppsFlatpakConfig::from_env(map.keys())
  }

  /// Everything, as the endpoint serves it: `apps` keyed by app key.
  /// Uses default (disabled) flatpak config without reading environment variables.
  pub fn snapshot(&self) -> CraftAppsReleaseInfo {
    self.snapshot_with_flatpak_config(&CraftAppsFlatpakConfig::default())
  }

  /// Takes explicit flatpak config / overrides rather than reading environment variables
  /// inside `snapshot()`.
  pub fn snapshot_with_flatpak_config(&self, config: &CraftAppsFlatpakConfig) -> CraftAppsReleaseInfo {
    let map = self.inner.read().unwrap_or_else(PoisonError::into_inner);

    let apps: BTreeMap<String, CraftAppReleaseInfo> = map
      .iter()
      .map(|(key, app)| {
        let flatpak_repo = if config.enabled {
          let app_override = config.app_overrides.get(key);
          let app_version_override = app_override.and_then(|o| o.version.clone());
          let min_version = app_override.and_then(|o| o.min_version.clone());
          let latest_v = app.latest_release.as_ref().map(|r| r.version.clone());

          let (version, is_synced) = match (app_version_override, latest_v) {
            (Some(override_v), Some(latest)) => {
              let synced = artcraft_api_defs::craft_apps::flatpak::compare_versions(&override_v, &latest) != Some(std::cmp::Ordering::Less);
              (Some(override_v), Some(synced))
            }
            (Some(override_v), None) => (Some(override_v), None),
            (None, _) => (None, None),
          };

          Some(artcraft_api_defs::craft_apps::flatpak::CraftAppFlatpakRepoInfo {
            app_id: format!("ai.storyteller.{key}"),
            repo_url: format!("{}/{}", config.repo_base_url.trim_end_matches('/'), key),
            flatpakrepo_url: format!("{}/v1/craft_apps/flatpak/{key}.flatpakrepo", config.api_base_url.trim_end_matches('/')),
            branch: "stable".to_string(),
            version,
            min_version,
            is_synced,
            gpg_key_url: config.gpg_key_url.clone(),
          })
        } else {
          None
        };

        (key.clone(), CraftAppReleaseInfo {
          name: app.name.clone(),
          repo: app.repo.clone(),
          latest_release: app.latest_release.clone(),
          app_last_fetched: app.app_last_fetched,
          flatpak_repo,
        })
      })
      .collect();
    let last_fetched = least_recently_fetched(&apps);
    CraftAppsReleaseInfo { apps, last_fetched }
  }
}

/// Configuration for Flatpak repository advertising in release info responses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CraftAppsFlatpakConfig {
  pub enabled: bool,
  pub repo_base_url: String,
  pub api_base_url: String,
  pub gpg_key_url: Option<String>,
  pub app_overrides: BTreeMap<String, CraftAppFlatpakOverride>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CraftAppFlatpakOverride {
  pub version: Option<String>,
  pub min_version: Option<String>,
}

impl Default for CraftAppsFlatpakConfig {
  fn default() -> Self {
    Self {
      enabled: false,
      repo_base_url: "https://repo.getartcraft.com".to_string(),
      api_base_url: "https://api.getartcraft.com".to_string(),
      gpg_key_url: Some("https://getartcraft.com/keys/flatpak.gpg".to_string()),
      app_overrides: BTreeMap::new(),
    }
  }
}

impl CraftAppsFlatpakConfig {
  /// Reads flatpak configuration and per-app overrides from environment variables once,
  /// outside of snapshot().
  pub fn from_env(known_app_keys: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
    let enabled = easyenv::get_env_bool_or_default("CRAFT_APPS_FLATPAK_REPO_ENABLED", false);
    let repo_base_url = easyenv::get_env_string_or_default("CRAFT_APPS_FLATPAK_REPO_BASE_URL", "https://repo.getartcraft.com");
    let api_base_url = easyenv::get_env_string_or_default("CRAFT_APPS_API_BASE_URL", "https://api.getartcraft.com");
    let website_base_url = easyenv::get_env_string_or_default("CRAFT_APPS_WEBSITE_BASE_URL", "https://getartcraft.com");
    let default_gpg_key = format!("{}/keys/flatpak.gpg", website_base_url.trim_end_matches('/'));
    let gpg_key_url = Some(easyenv::get_env_string_or_default("CRAFT_APPS_FLATPAK_GPG_KEY_URL", &default_gpg_key));

    let mut app_overrides = BTreeMap::new();
    if enabled {
      for app_key in known_app_keys {
        let key_str = app_key.as_ref();
        let app_env_key = key_str.to_uppercase().replace('-', "_");
        let version = easyenv::get_env_string_optional(&format!("CRAFT_APPS_FLATPAK_VERSION_{app_env_key}"));
        let min_version = easyenv::get_env_string_optional(&format!("CRAFT_APPS_FLATPAK_MIN_VERSION_{app_env_key}"));
        if version.is_some() || min_version.is_some() {
          app_overrides.insert(key_str.to_string(), CraftAppFlatpakOverride { version, min_version });
        }
      }
    }

    Self {
      enabled,
      repo_base_url,
      api_base_url,
      gpg_key_url,
      app_overrides,
    }
  }
}

impl CachedCraftApp {
  /// Whether the last request for this app failed (no success since).
  pub fn is_failing(&self) -> bool {
    match (self.last_failed_at, self.app_last_fetched) {
      (Some(failed), Some(fetched)) => failed > fetched,
      (Some(_), None) => true,
      (None, _) => false,
    }
  }
}

/// The oldest `app_last_fetched`, or `None` while any app has never been fetched.
fn least_recently_fetched(apps: &BTreeMap<String, CraftAppReleaseInfo>) -> Option<DateTime<Utc>> {
  if apps.is_empty() {
    return None;
  }
  let mut oldest: Option<DateTime<Utc>> = None;
  for app in apps.values() {
    let fetched = app.app_last_fetched?;
    oldest = Some(oldest.map_or(fetched, |o| o.min(fetched)));
  }
  oldest
}

#[cfg(test)]
mod tests {
  use chrono::TimeZone;

  use super::*;

  #[test]
  fn last_fetched_is_the_oldest_app_and_absent_until_all_are_fetched() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[
      ("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into()),
      ("filmcraft".into(), "FilmCraft".into(), "storytold/filmcraft".into()),
    ]);
    assert_eq!(cache.snapshot().apps.len(), 2);
    assert_eq!(cache.snapshot().last_fetched, None);

    let early = Utc.with_ymd_and_hms(2026, 10, 10, 1, 0, 0).unwrap();
    let late = Utc.with_ymd_and_hms(2026, 10, 10, 2, 0, 0).unwrap();
    cache.store("photocraft", None, Some("\"abc\"".into()), late);
    assert_eq!(cache.snapshot().last_fetched, None, "filmcraft hasn't been fetched yet");

    cache.confirm_unchanged("filmcraft", early);
    assert_eq!(cache.snapshot().last_fetched, Some(early));
    assert_eq!(cache.etag("photocraft").as_deref(), Some("\"abc\""));
  }

  #[test]
  fn registering_again_keeps_what_was_fetched() {
    let cache = CraftAppsReleaseCache::new();
    let apps = [("photocraft".to_string(), "PhotoCraft".to_string(), "storytold/photocraft".to_string())];
    cache.register_apps(&apps);
    let now = Utc::now();
    cache.confirm_unchanged("photocraft", now);
    cache.register_apps(&apps);
    assert_eq!(cache.snapshot().apps["photocraft"].app_last_fetched, Some(now));
  }

  #[test]
  fn merging_takes_only_newer_fetches_of_registered_apps() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into())]);
    let early = Utc.with_ymd_and_hms(2026, 10, 10, 1, 0, 0).unwrap();
    let late = Utc.with_ymd_and_hms(2026, 10, 10, 2, 0, 0).unwrap();

    assert!(cache.merge_fetched("photocraft", None, Some("\"new\"".into()), late));
    assert!(!cache.merge_fetched("photocraft", None, Some("\"old\"".into()), early), "older than ours");
    assert!(!cache.merge_fetched("photocraft", None, Some("\"same\"".into()), late), "no newer than ours");
    assert!(!cache.merge_fetched("unknowncraft", None, None, late), "not registered");

    let snapshot = cache.snapshot();
    assert_eq!(snapshot.apps.len(), 1);
    assert_eq!(snapshot.apps["photocraft"].app_last_fetched, Some(late));
    assert_eq!(cache.etag("photocraft").as_deref(), Some("\"new\""));
  }

  #[test]
  fn failures_count_up_until_a_success() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into())]);
    let t = |h| Utc.with_ymd_and_hms(2026, 10, 10, h, 0, 0).unwrap();

    assert_eq!(cache.record_failure("photocraft", t(1)), 1);
    assert_eq!(cache.record_failure("photocraft", t(2)), 2);
    assert!(cache.app("photocraft").unwrap().is_failing());

    cache.store("photocraft", None, None, t(3));
    assert!(!cache.app("photocraft").unwrap().is_failing());
    assert_eq!(cache.record_failure("photocraft", t(4)), 1, "counting restarts after a success");

    cache.merge_failure("photocraft", t(3), 9);
    assert_eq!(cache.app("photocraft").unwrap().consecutive_failures, 1, "older failure ignored");
    cache.merge_failure("photocraft", t(5), 4);
    assert_eq!(cache.app("photocraft").unwrap().consecutive_failures, 4);
  }

  #[test]
  fn snapshot_populates_flatpak_repo_info() {
    let cache = CraftAppsReleaseCache::new();
    let apps = [("photocraft".to_string(), "PhotoCraft".to_string(), "storytold/photocraft".to_string())];
    cache.register_apps(&apps);

    // Default snapshot has flatpak disabled:
    let snapshot = cache.snapshot();
    assert!(snapshot.apps["photocraft"].flatpak_repo.is_none());

    // With flatpak enabled:
    let config = CraftAppsFlatpakConfig {
      enabled: true,
      repo_base_url: "https://repo.getartcraft.com".to_string(),
      api_base_url: "https://api.getartcraft.com".to_string(),
      gpg_key_url: Some("https://getartcraft.com/keys/flatpak.gpg".to_string()),
      app_overrides: BTreeMap::new(),
    };
    let snapshot = cache.snapshot_with_flatpak_config(&config);
    let flatpak = snapshot.apps["photocraft"].flatpak_repo.as_ref().unwrap();
    assert_eq!(flatpak.app_id, "ai.storyteller.photocraft");
    assert!(flatpak.repo_url.contains("photocraft"));
    assert!(flatpak.flatpakrepo_url.ends_with("photocraft.flatpakrepo"));
    assert_eq!(flatpak.branch, "stable");
    assert_eq!(flatpak.gpg_key_url.as_deref(), Some("https://getartcraft.com/keys/flatpak.gpg"));
  }

  #[test]
  fn snapshot_detects_lagging_flatpak_repo() {
    let cache = CraftAppsReleaseCache::new();
    let apps = [("photocraft".to_string(), "PhotoCraft".to_string(), "storytold/photocraft".to_string())];
    cache.register_apps(&apps);
    let release = artcraft_api_defs::craft_apps::release_info::CraftAppRelease {
      tag: "v0.5.0".to_string(),
      version: "0.5.0".to_string(),
      name: "PhotoCraft 0.5.0".to_string(),
      published_at: None,
      html_url: "https://github.com/storytold/photocraft/releases/tag/v0.5.0".to_string(),
      prerelease: false,
      notes_markdown: String::new(),
      assets: Vec::new(),
    };
    cache.store("photocraft", Some(release), None, Utc::now());

    // Without override, version and is_synced are None (unknown repo state):
    let config_no_override = CraftAppsFlatpakConfig {
      enabled: true,
      repo_base_url: "https://repo.getartcraft.com".to_string(),
      api_base_url: "https://api.getartcraft.com".to_string(),
      gpg_key_url: Some("https://getartcraft.com/keys/flatpak.gpg".to_string()),
      app_overrides: BTreeMap::new(),
    };
    let snapshot = cache.snapshot_with_flatpak_config(&config_no_override);
    let flatpak = snapshot.apps["photocraft"].flatpak_repo.as_ref().unwrap();
    assert_eq!(flatpak.version, None);
    assert_eq!(flatpak.is_synced, None);

    // With explicit override simulating repo lag (0.4.0 < 0.5.0):
    let mut overrides = BTreeMap::new();
    overrides.insert("photocraft".to_string(), CraftAppFlatpakOverride {
      version: Some("0.4.0".to_string()),
      min_version: None,
    });
    let config_lag = CraftAppsFlatpakConfig {
      enabled: true,
      repo_base_url: "https://repo.getartcraft.com".to_string(),
      api_base_url: "https://api.getartcraft.com".to_string(),
      gpg_key_url: Some("https://getartcraft.com/keys/flatpak.gpg".to_string()),
      app_overrides: overrides,
    };
    let snapshot_lag = cache.snapshot_with_flatpak_config(&config_lag);
    let flatpak_lag = snapshot_lag.apps["photocraft"].flatpak_repo.as_ref().unwrap();
    assert_eq!(flatpak_lag.version.as_deref(), Some("0.4.0"));
    assert_eq!(flatpak_lag.is_synced, Some(false));
    assert!(flatpak_lag.would_downgrade("0.5.0"));
  }

  #[test]
  fn get_app_and_release_lookup() {
    let cache = CraftAppsReleaseCache::new();
    let apps = [
      ("photocraft".to_string(), "PhotoCraft".to_string(), "storytold/photocraft".to_string()),
      ("artcraft-launcher".to_string(), "ArtCraft Launcher".to_string(), "storytold/craft-launcher".to_string()),
    ];
    cache.register_apps(&apps);

    // Initial state: app known, but no release
    let app_meta = cache.get_app("photocraft");
    assert!(app_meta.is_some());
    assert_eq!(app_meta.as_ref().unwrap().name, "PhotoCraft");
    assert!(cache.get_app_release("photocraft").is_none());

    let (canonical_key, app_entry) = cache.get_app_with_key("photocraft").unwrap();
    assert_eq!(canonical_key, "photocraft");
    assert_eq!(app_entry.name, "PhotoCraft");

    // Alias lookups
    assert!(cache.get_app("PhotoCraft").is_some());
    assert!(cache.get_app("launcher").is_some());
    assert!(cache.get_app("craft-launcher").is_some());
    let (alias_key, _) = cache.get_app_with_key("craft-launcher").unwrap();
    assert_eq!(alias_key, "artcraft-launcher");
    assert!(cache.get_app("unknown-app").is_none());

    // Once release stored:
    let release = artcraft_api_defs::craft_apps::release_info::CraftAppRelease {
      tag: "v0.5.0".to_string(),
      version: "0.5.0".to_string(),
      name: "PhotoCraft 0.5.0".to_string(),
      published_at: None,
      html_url: "https://github.com/storytold/photocraft/releases/tag/v0.5.0".to_string(),
      prerelease: false,
      notes_markdown: String::new(),
      assets: Vec::new(),
    };
    cache.store("photocraft", Some(release), None, Utc::now());

    let fetched = cache.get_app_release("photocraft");
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().version, "0.5.0");

    let fetched_alias = cache.get_app_release("PhotoCraft");
    assert!(fetched_alias.is_some());
    assert_eq!(fetched_alias.unwrap().version, "0.5.0");
  }
}
