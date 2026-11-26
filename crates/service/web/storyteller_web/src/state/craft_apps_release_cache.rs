//! The releases of every Craft App, as last fetched from GitHub by
//! `poll_craft_app_releases_thread` (directly, or from the fleet's shared copy in Redis), served
//! by `GET /v1/craft_apps/release_info` and, each app's whole history,
//! `GET /v1/craft_apps/{app}/releases`. The endpoint always answers from this memory, so trouble
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
use serde_derive::{Deserialize, Serialize};

use artcraft_api_defs::craft_apps::release_info::{CraftAppRelease, CraftAppReleaseInfo, CraftAppReleaseSummary, CraftAppsReleaseInfo};

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

  /// What the last fetch found. Empty both before the first fetch and when the repository has
  /// no release; `app_last_fetched` tells the two apart.
  pub releases: CraftAppReleases,

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

  /// The app's whole release history. Shared, since it can be large and `app` is cloned often.
  pub history: Arc<CraftAppHistory>,

  /// Whether `history` belongs to the fetch `releases` came from. False once another instance's
  /// newer fetch was taken until its history is read from Redis; not the ETag's job, since a fetch
  /// may have none.
  pub history_current: bool,

  /// The fleet's shared history for the current releases isn't in Redis (expired or evicted):
  /// this instance fetches the app again in full, so the history is written anew.
  pub history_missing: bool,
}

/// An app's release history: every published release, newest first, nightlies included, drafts
/// not. Also the JSON the fleet shares in Redis, apart from the rest (`shared_entry`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CraftAppHistory {
  /// GitHub's ETag for the first page of the fetch this history came from: it belongs with the
  /// releases fetched under the same ETag, and is replaced when those are.
  pub etag: Option<String>,

  /// Whether `releases` is every release GitHub lists (the last page read wasn't full).
  pub complete: bool,

  pub releases: Vec<CraftAppRelease>,

  /// When the whole list was last read from GitHub (a `200`, not a `304`). A `304` on the first
  /// page says nothing about edits to older releases, so the list is read in full again after a
  /// while (`FULL_REFRESH`).
  #[serde(default)]
  pub fetched_at: Option<DateTime<Utc>>,
}

/// Everything one fetch learns about an app's releases; the endpoint serves each field under the
/// same name. Also the JSON the fleet shares in Redis (`shared_entry`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CraftAppReleases {
  /// The stable release with the highest version (not a pre-release, not a nightly).
  pub latest_release: Option<CraftAppRelease>,

  /// The beta or release candidate with the highest version.
  #[serde(default)]
  pub latest_prerelease: Option<CraftAppRelease>,

  /// The newest nightly build.
  #[serde(default)]
  pub latest_nightly: Option<CraftAppRelease>,

  /// The recent stable releases and pre-releases, newest first.
  #[serde(default)]
  pub releases: Vec<CraftAppReleaseSummary>,
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
        releases: CraftAppReleases::default(),
        app_last_fetched: None,
        etag: None,
        last_failed_at: None,
        consecutive_failures: 0,
        history: Arc::default(),
        history_current: false,
        history_missing: false,
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
  /// releases (empty if it has none) and history. Ignored for keys that were never registered.
  pub fn store(&self, key: &str, releases: CraftAppReleases, history: CraftAppHistory, etag: Option<String>, fetched_at: DateTime<Utc>) {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(app) = map.get_mut(key) {
      app.releases = releases;
      app.history = Arc::new(history);
      app.history_current = true;
      app.history_missing = false;
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
  pub fn merge_fetched(&self, key: &str, releases: CraftAppReleases, etag: Option<String>, fetched_at: DateTime<Utc>, history_at: Option<DateTime<Utc>>) -> bool {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    let Some(app) = map.get_mut(key) else {
      return false;
    };
    if app.app_last_fetched.is_some_and(|mine| mine >= fetched_at) {
      return false;
    }
    // A `304` elsewhere only moved the time: the history this instance has still belongs. A new or
    // missing ETag may mean new releases, and a newer full read may have found edits to older ones
    // (a `304` covers only the first page), so then the history is read from Redis.
    let read_since = history_at.is_some_and(|shared| app.history.fetched_at.is_none_or(|mine| mine < shared));
    if etag.is_none() || etag != app.etag || read_since {
      app.history_current = false;
      app.history_missing = false;
    }
    app.releases = releases;
    app.etag = etag;
    app.app_last_fetched = Some(fetched_at);
    true
  }

  /// Takes a history shared elsewhere (another instance, through Redis) for the app with app key
  /// `key`, when it belongs with the releases this instance has (the same ETag). Returns whether
  /// it was taken.
  pub fn merge_history(&self, key: &str, history: CraftAppHistory) -> bool {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    let Some(app) = map.get_mut(key) else {
      return false;
    };
    if history.etag != app.etag || app.history_current {
      return false;
    }
    app.history = Arc::new(history);
    app.history_current = true;
    app.history_missing = false;
    true
  }

  /// Records that the shared history the app with app key `key` needs isn't in Redis.
  pub fn mark_history_missing(&self, key: &str) {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(app) = map.get_mut(key) {
      app.history_missing = !app.history_current;
    }
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
            latest_release: app.releases.latest_release.clone(),
            app_last_fetched: app.app_last_fetched,
            latest_prerelease: app.releases.latest_prerelease.clone(),
            latest_nightly: app.releases.latest_nightly.clone(),
            releases: app.releases.releases.clone(),
          })
        })
        .collect();
    let last_fetched = least_recently_fetched(&apps);
    CraftAppsReleaseInfo { apps, last_fetched }
  }
}

impl CachedCraftApp {
  /// Whether the history is not the one fetched with the current releases (it came from another
  /// instance's fetch and is still to be read from Redis).
  pub fn history_is_behind(&self) -> bool {
    self.app_last_fetched.is_some() && !self.history_current
  }

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
  fn the_snapshot_serves_every_channel() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into())]);
    let stable = release("v0.5.0", false);
    let beta = release("v0.6.0-rc.1", true);
    let releases = CraftAppReleases {
      releases: vec![(&beta).into(), (&stable).into()],
      latest_release: Some(stable),
      latest_prerelease: Some(beta),
      latest_nightly: Some(release("v0.6.0-nightly.20261126.1.1", true)),
    };
    cache.store("photocraft", releases, CraftAppHistory::default(), None, Utc::now());

    let app = &cache.snapshot().apps["photocraft"];
    assert_eq!(app.latest_release.as_ref().map(|r| r.tag.as_str()), Some("v0.5.0"));
    assert_eq!(app.latest_prerelease.as_ref().map(|r| r.tag.as_str()), Some("v0.6.0-rc.1"));
    assert_eq!(app.latest_nightly.as_ref().map(|r| r.tag.as_str()), Some("v0.6.0-nightly.20261126.1.1"));
    let versions: Vec<&str> = app.releases.iter().map(|r| r.tag.as_str()).collect();
    assert_eq!(versions, ["v0.6.0-rc.1", "v0.5.0"]);
  }

  fn release(tag: &str, prerelease: bool) -> CraftAppRelease {
    CraftAppRelease {
      tag: tag.into(),
      version: tag.trim_start_matches('v').into(),
      name: format!("PhotoCraft {tag}"),
      published_at: None,
      html_url: format!("https://github.com/storytold/photocraft/releases/tag/{tag}"),
      prerelease,
      notes_markdown: String::new(),
      assets: Vec::new(),
    }
  }

  #[test]
  fn a_shared_history_is_taken_only_with_the_releases_it_belongs_to() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into())]);
    let at = Utc::now();
    let history = |etag: &str| CraftAppHistory { etag: Some(etag.into()), complete: true, releases: vec![release("v0.5.0", false)], fetched_at: None };
    assert!(cache.merge_fetched("photocraft", CraftAppReleases::default(), Some("\"b\"".into()), at, None));
    assert!(cache.app("photocraft").unwrap().history_is_behind());
    assert!(!cache.merge_history("photocraft", history("\"a\"")), "from an older fetch");
    assert!(cache.merge_history("photocraft", history("\"b\"")));
    assert!(!cache.app("photocraft").unwrap().history_is_behind());
    assert!(!cache.merge_history("photocraft", history("\"b\"")), "already there");
    assert_eq!(cache.app("photocraft").unwrap().history.releases.len(), 1);
  }

  /// A `304` elsewhere (same ETag, later time) keeps this instance's history; a new ETag doesn't.
  #[test]
  fn only_new_releases_send_an_instance_back_for_their_history() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into())]);
    let at = Utc::now();
    let history = CraftAppHistory { etag: Some("\"a\"".into()), complete: true, releases: Vec::new(), fetched_at: Some(at) };
    cache.store("photocraft", CraftAppReleases::default(), history, Some("\"a\"".into()), at);
    assert!(cache.merge_fetched("photocraft", CraftAppReleases::default(), Some("\"a\"".into()), at + chrono::Duration::minutes(2), None));
    assert!(!cache.app("photocraft").unwrap().history_is_behind(), "same ETag: nothing to read");
    assert!(cache.merge_fetched("photocraft", CraftAppReleases::default(), Some("\"b\"".into()), at + chrono::Duration::minutes(4), None));
    assert!(cache.app("photocraft").unwrap().history_is_behind());
    cache.mark_history_missing("photocraft");
    assert!(cache.app("photocraft").unwrap().history_missing);
  }

  /// Another instance's full read (same ETag: page 1 didn't change) may have found edits to older
  /// releases, so a history read in full after this instance's own is read from Redis too.
  #[test]
  fn a_newer_full_read_elsewhere_sends_an_instance_back_for_the_history() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into())]);
    let at = Utc::now();
    let history = CraftAppHistory { etag: Some("\"a\"".into()), complete: true, releases: Vec::new(), fetched_at: Some(at) };
    cache.store("photocraft", CraftAppReleases::default(), history, Some("\"a\"".into()), at);
    assert!(cache.merge_fetched("photocraft", CraftAppReleases::default(), Some("\"a\"".into()), at + chrono::Duration::minutes(2), Some(at)));
    assert!(!cache.app("photocraft").unwrap().history_is_behind(), "the full read this instance already has");
    let later = at + chrono::Duration::hours(6);
    assert!(cache.merge_fetched("photocraft", CraftAppReleases::default(), Some("\"a\"".into()), later, Some(later)));
    assert!(cache.app("photocraft").unwrap().history_is_behind());
  }

  /// A fetch without an ETag still has its history read from Redis: "none" is not "already here".
  #[test]
  fn a_shared_fetch_without_an_etag_still_takes_its_history() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into())]);
    assert!(cache.merge_fetched("photocraft", CraftAppReleases::default(), None, Utc::now(), None));
    assert!(cache.app("photocraft").unwrap().history_is_behind());
    let history = CraftAppHistory { etag: None, complete: true, releases: vec![release("v0.5.0", false)], fetched_at: None };
    assert!(cache.merge_history("photocraft", history));
    assert!(!cache.app("photocraft").unwrap().history_is_behind());
    assert_eq!(cache.app("photocraft").unwrap().history.releases.len(), 1);
  }

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
    cache.store("photocraft", CraftAppReleases::default(), CraftAppHistory::default(), Some("\"abc\"".into()), late);
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

    assert!(cache.merge_fetched("photocraft", CraftAppReleases::default(), Some("\"new\"".into()), late, None));
    assert!(!cache.merge_fetched("photocraft", CraftAppReleases::default(), Some("\"old\"".into()), early, None), "older than ours");
    assert!(!cache.merge_fetched("photocraft", CraftAppReleases::default(), Some("\"same\"".into()), late, None), "no newer than ours");
    assert!(!cache.merge_fetched("unknowncraft", CraftAppReleases::default(), None, late, None), "not registered");

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

    cache.store("photocraft", CraftAppReleases::default(), CraftAppHistory::default(), None, t(3));
    assert!(!cache.app("photocraft").unwrap().is_failing());
    assert_eq!(cache.record_failure("photocraft", t(4)), 1, "counting restarts after a success");

    cache.merge_failure("photocraft", t(3), 9);
    assert_eq!(cache.app("photocraft").unwrap().consecutive_failures, 1, "older failure ignored");
    cache.merge_failure("photocraft", t(5), 4);
    assert_eq!(cache.app("photocraft").unwrap().consecutive_failures, 4);
  }
}
