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

  /// Everything, as the endpoint serves it: `apps` keyed by app key.
  pub fn snapshot(&self) -> CraftAppsReleaseInfo {
    let map = self.inner.read().unwrap_or_else(PoisonError::into_inner);
    let apps: BTreeMap<String, CraftAppReleaseInfo> = map
        .iter()
        .map(|(key, app)| {
          (key.clone(), CraftAppReleaseInfo {
            name: app.name.clone(),
            repo: app.repo.clone(),
            latest_release: app.latest_release.clone(),
            app_last_fetched: app.app_last_fetched,
          })
        })
        .collect();
    let last_fetched = least_recently_fetched(&apps);
    CraftAppsReleaseInfo { apps, last_fetched }
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
}
