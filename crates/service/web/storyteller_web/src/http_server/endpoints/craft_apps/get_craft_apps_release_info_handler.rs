use std::sync::Arc;

use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::{web, HttpResponse};

use artcraft_api_defs::craft_apps::release_info::{CraftAppsImportantNotices, GetCraftAppsReleaseInfoResponse};

use crate::state::server_state::ServerState;

/// Shared caches (Cloudflare) may serve the same answer for this long: the data changes at most
/// once per polling round (30 minutes by default).
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
  let response = GetCraftAppsReleaseInfoResponse {
    success: true,
    release_info: server_state.craft_apps_release_cache.snapshot(),
    // None yet. When there are, they're listed here (newest first).
    app_renames: Vec::new(),
    repo_renames: Vec::new(),
    important_notices: CraftAppsImportantNotices::default(),
  };
  HttpResponse::Ok()
      .insert_header(CacheControl(vec![CacheDirective::Public, CacheDirective::MaxAge(CACHE_MAX_AGE_SECS)]))
      .json(response)
}
