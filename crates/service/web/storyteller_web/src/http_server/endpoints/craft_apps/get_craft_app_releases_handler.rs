use std::sync::Arc;

use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::{web, HttpResponse};

use artcraft_api_defs::craft_apps::releases::{GetCraftAppReleasesError, GetCraftAppReleasesResponse};

use crate::state::craft_apps_release_cache::CachedCraftApp;
use crate::state::server_state::ServerState;

/// Shared caches (Cloudflare) may serve the same answer for this long once the app has been
/// fetched: the data changes at most once per polling interval.
const CACHE_MAX_AGE_SECS: u32 = 60;

/// One Craft App's release history (every published release, with files and checksums), as
/// storyteller-web last fetched it from GitHub. `404` for an app it doesn't poll. Before the
/// first fetch the history is empty and `app_last_fetched` is absent.
#[utoipa::path(
  get,
  tag = "Craft Apps",
  path = "/v1/craft_apps/{app_key}/releases",
  params(("app_key" = String, Path, description = "The app's lowercase machine name, e.g. `photocraft`")),
  responses(
    (status = 200, body = GetCraftAppReleasesResponse),
    (status = 404, body = GetCraftAppReleasesError),
  ),
)]
pub async fn get_craft_app_releases_handler(
  path: web::Path<String>,
  server_state: web::Data<Arc<ServerState>>,
) -> HttpResponse {
  let app_key = path.into_inner().to_lowercase();
  let Some(app) = server_state.craft_apps_release_cache.app(&app_key) else {
    return HttpResponse::NotFound()
        .insert_header(CacheControl(vec![CacheDirective::NoStore]))
        .json(GetCraftAppReleasesError { success: false, error_reason: format!("no releases are known for app '{}'", truncate_key(&app_key)) });
  };
  let response = releases_response(app_key, &app);
  let cache_control = if response.app_last_fetched.is_some() && !app.history_is_behind() {
    CacheControl(vec![CacheDirective::Public, CacheDirective::MaxAge(CACHE_MAX_AGE_SECS)])
  } else {
    CacheControl(vec![CacheDirective::NoStore])
  };
  HttpResponse::Ok()
      .insert_header(cache_control)
      .json(response)
}

/// What the endpoint serves for `app`. A history still to be read from Redis (another instance
/// fetched newer releases) is served as it is, but never as complete, nor cached.
fn releases_response(app_key: String, app: &CachedCraftApp) -> GetCraftAppReleasesResponse {
  GetCraftAppReleasesResponse {
    success: true,
    app_key,
    name: app.name.clone(),
    repo: app.repo.clone(),
    app_last_fetched: app.app_last_fetched,
    complete: app.history.complete && !app.history_is_behind(),
    releases: app.history.releases.clone(),
  }
}

/// The requested key, cut short for the error message (it comes from the URL).
fn truncate_key(key: &str) -> String {
  key.chars().take(64).collect()
}

#[cfg(test)]
mod tests {
  use chrono::Utc;

  use crate::state::craft_apps_release_cache::{CraftAppHistory, CraftAppReleases, CraftAppsReleaseCache};

  use super::*;

  #[test]
  fn serves_the_history_of_a_fetched_app() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into())]);
    let unfetched = releases_response("photocraft".into(), &cache.app("photocraft").unwrap());
    assert!(unfetched.app_last_fetched.is_none() && unfetched.releases.is_empty());

    let history = CraftAppHistory { etag: None, complete: false, releases: Vec::new(), fetched_at: None };
    cache.store("photocraft", CraftAppReleases::default(), history, None, Utc::now());
    let fetched = releases_response("photocraft".into(), &cache.app("photocraft").unwrap());
    assert!(fetched.app_last_fetched.is_some());
    assert!(!fetched.complete, "an incomplete history says so");
    assert_eq!(fetched.repo, "storytold/photocraft");
  }

  #[test]
  fn a_history_behind_its_releases_is_never_called_complete() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into())]);
    let complete = CraftAppHistory { etag: Some("\"a\"".into()), complete: true, releases: Vec::new(), fetched_at: None };
    cache.store("photocraft", CraftAppReleases::default(), complete, Some("\"a\"".into()), Utc::now());
    assert!(releases_response("photocraft".into(), &cache.app("photocraft").unwrap()).complete);
    // Another instance fetched newer releases; their history isn't read yet.
    cache.merge_fetched("photocraft", CraftAppReleases::default(), Some("\"b\"".into()), Utc::now() + chrono::Duration::seconds(5));
    assert!(!releases_response("photocraft".into(), &cache.app("photocraft").unwrap()).complete);
  }

  #[test]
  fn an_error_never_echoes_a_long_key() {
    assert_eq!(truncate_key(&"x".repeat(500)).len(), 64);
  }
}
