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
  let flatpak_config = server_state.craft_apps_release_cache.load_flatpak_config_from_env();

  let suite_flatpak_repo = if flatpak_config.enabled {
    Some(artcraft_api_defs::craft_apps::flatpak::CraftAppFlatpakRepoInfo {
      app_id: "ai.storyteller.artcraft".to_string(),
      repo_url: format!("{}/", flatpak_config.repo_base_url.trim_end_matches('/')),
      flatpakrepo_url: format!("{}/v1/craft_apps/flatpak/artcraft.flatpakrepo", flatpak_config.api_base_url.trim_end_matches('/')),
      branch: "stable".to_string(),
      version: None,
      min_version: None,
      is_synced: None,
      gpg_key_url: flatpak_config.gpg_key_url.clone(),
    })
  } else {
    None
  };

  let release_info = server_state.craft_apps_release_cache.snapshot_with_flatpak_config(&flatpak_config);
  let cache_control = cache_control_for(&release_info);
  let response = GetCraftAppsReleaseInfoResponse {
    success: true,
    release_info,
    // None yet. When there are, they're listed here (newest first).
    app_renames: Vec::new(),
    repo_renames: Vec::new(),
    important_notices: CraftAppsImportantNotices::default(),
    suite_flatpak_repo,
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
}
