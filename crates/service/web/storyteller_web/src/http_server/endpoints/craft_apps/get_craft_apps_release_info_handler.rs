use std::sync::Arc;

use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::{web, HttpResponse};

use artcraft_api_defs::craft_apps::release_info::{CraftAppsImportantNotices, CraftAppsReleaseInfo, GetCraftAppsReleaseInfoResponse};

use crate::state::server_state::ServerState;

/// Shared caches (Cloudflare) may serve the same answer for this long once every app has been
/// fetched: the data changes at most once per polling interval.
const CACHE_MAX_AGE_SECS: u32 = 60;

/// The latest release of every Craft App (and ArtCraft Launcher), as storyteller-web last fetched
/// it from GitHub, plus app/repo renames and notices for the launcher. Never fails: before the
/// first fetch, apps are listed without a release and without `app_last_fetched`.
#[utoipa::path(
  get,
  tag = "Craft Apps",
  path = "/v1/craft_apps/release_info",
  responses(
    (status = 200, body = GetCraftAppsReleaseInfoResponse),
  ),
)]
pub async fn get_craft_apps_release_info_handler(
  server_state: web::Data<Arc<ServerState>>,
) -> HttpResponse {
  let release_info = server_state.craft_apps_release_cache.snapshot();
  let cache_control = cache_control_for(&release_info);
  let response = GetCraftAppsReleaseInfoResponse {
    success: true,
    release_info,
    // None yet. When there are, they're listed here (newest first).
    app_renames: Vec::new(),
    repo_renames: Vec::new(),
    important_notices: CraftAppsImportantNotices::default(),
  };
  HttpResponse::Ok()
      .insert_header(cache_control)
      .json(response)
}

/// Cacheable only once every app has been fetched (`last_fetched` is set). Until then (an
/// instance that just started, or one whose first fetch failed) the answer is `no-store`, so a
/// shared cache never keeps handing out "not fetched yet" after this instance has warmed up.
fn cache_control_for(release_info: &CraftAppsReleaseInfo) -> CacheControl {
  if release_info.last_fetched.is_some() {
    CacheControl(vec![CacheDirective::Public, CacheDirective::MaxAge(CACHE_MAX_AGE_SECS)])
  } else {
    CacheControl(vec![CacheDirective::NoStore])
  }
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;

  use chrono::Utc;

  use artcraft_api_defs::craft_apps::release_info::CraftAppReleaseInfo;

  use crate::state::craft_apps_release_cache::{CraftAppReleases, CraftAppsReleaseCache};

  use super::*;

  #[test]
  fn a_fully_fetched_answer_is_cacheable() {
    let release_info = CraftAppsReleaseInfo { apps: BTreeMap::new(), last_fetched: Some(Utc::now()) };
    assert_eq!(cache_control_for(&release_info).to_string(), "public, max-age=60");
  }

  #[test]
  fn an_answer_with_unfetched_apps_is_never_stored() {
    let release_info = CraftAppsReleaseInfo { apps: BTreeMap::new(), last_fetched: None };
    assert_eq!(cache_control_for(&release_info).to_string(), "no-store");
  }

  /// Deployed launchers read `latest_release` and `app_last_fetched` per app: the channels and
  /// the version list are added beside them, never in their place.
  #[test]
  fn every_app_keeps_its_original_fields_beside_the_new_ones() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into())]);
    cache.store("photocraft", CraftAppReleases::default(), None, Utc::now());
    let json = serde_json::to_value(cache.snapshot()).unwrap();
    let app = json["apps"]["photocraft"].as_object().unwrap();
    for field in ["name", "repo", "latest_release", "app_last_fetched", "latest_prerelease", "latest_nightly", "releases"] {
      assert!(app.contains_key(field), "{}", field);
    }
    assert_eq!(app["repo"], "storytold/photocraft");
    assert!(app["releases"].as_array().unwrap().is_empty());
    assert!(json["last_fetched"].is_string());
  }

  /// An answer from before the channels existed still reads, with them empty.
  #[test]
  fn an_app_without_the_new_fields_still_reads() {
    let old = r#"{"name":"PhotoCraft","repo":"storytold/photocraft","latest_release":null,"app_last_fetched":null}"#;
    let app: CraftAppReleaseInfo = serde_json::from_str(old).unwrap();
    assert!(app.latest_prerelease.is_none() && app.latest_nightly.is_none() && app.releases.is_empty());
  }
}
