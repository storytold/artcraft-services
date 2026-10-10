//! The latest release of every Craft App, as last fetched from GitHub by
//! `poll_craft_app_releases_thread`, served by `GET /v1/craft_apps/release_info`.

use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError, RwLock};

use chrono::{DateTime, Utc};

use artcraft_api_defs::craft_apps::release_info::{CraftAppRelease, CraftAppReleaseInfo, CraftAppsReleaseInfo};

/// Shared between the polling thread and the HTTP handlers. Cheap to clone.
#[derive(Clone, Default)]
pub struct CraftAppsReleaseCache {
  inner: Arc<RwLock<BTreeMap<String, CachedCraftApp>>>,
}

/// One app: what's known about it, and the ETag to revalidate it with.
#[derive(Clone, Debug)]
pub struct CachedCraftApp {
  pub name: String,
  pub repo: String,
  pub latest_release: Option<CraftAppRelease>,
  pub app_last_fetched: Option<DateTime<Utc>>,
  pub etag: Option<String>,
}

impl CraftAppsReleaseCache {
  pub fn new() -> Self {
    Self::default()
  }

  /// Makes sure every app in `apps` (lowercase name, display name, repo) has an entry, so the
  /// endpoint reports apps that haven't been fetched yet.
  pub fn register_apps(&self, apps: &[(String, String, String)]) {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    for (key, name, repo) in apps {
      map.entry(key.clone()).or_insert_with(|| CachedCraftApp {
        name: name.clone(),
        repo: repo.clone(),
        latest_release: None,
        app_last_fetched: None,
        etag: None,
      });
    }
  }

  /// The ETag of the last successful answer for `key`, to send as `If-None-Match`.
  pub fn etag(&self, key: &str) -> Option<String> {
    let map = self.inner.read().unwrap_or_else(PoisonError::into_inner);
    map.get(key).and_then(|app| app.etag.clone())
  }

  /// Records a complete, successful fetch of `key`: its latest release (`None` if it has none).
  pub fn store(&self, key: &str, latest_release: Option<CraftAppRelease>, etag: Option<String>, fetched_at: DateTime<Utc>) {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(app) = map.get_mut(key) {
      app.latest_release = latest_release;
      app.etag = etag;
      app.app_last_fetched = Some(fetched_at);
    }
  }

  /// Records that GitHub confirmed `key` is unchanged (`304 Not Modified`).
  pub fn confirm_unchanged(&self, key: &str, fetched_at: DateTime<Utc>) {
    let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(app) = map.get_mut(key) {
      app.app_last_fetched = Some(fetched_at);
    }
  }

  /// Everything, as the endpoint serves it.
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
}
