//! Redirects Flatpak OSTree repository requests to the backing cloud object storage (S3/R2).

use actix_web::http::header::{CacheControl, CacheDirective, LOCATION};
use actix_web::{web, HttpResponse};

const DEFAULT_OBJECT_CACHE_SECS: u32 = 86400;
const METADATA_CACHE_SECS: u32 = 60;

/// Redirects Flatpak repository file requests (summary, refs, objects, deltas)
/// to the backing S3 or Cloudflare R2 object storage CDN.
#[utoipa::path(
  get,
  tag = "Craft Apps",
  path = "/v1/craft_apps/flatpak/repo/{tail:.*}",
  params(
    ("tail" = String, Path, description = "Relative path within the OSTree repository (e.g. 'summary', 'config', 'objects/..')"),
  ),
  responses(
    (status = 307, description = "Redirect to the backing OSTree storage CDN"),
  ),
)]
pub async fn get_craft_apps_flatpak_repo_redirect_handler(tail: web::Path<String>) -> HttpResponse {
  let relative_path = tail.into_inner();
  if relative_path.ends_with(".flatpakrepo") {
    return HttpResponse::NotFound().finish();
  }

  let base_url = easyenv::get_env_string_or_default("CRAFT_APPS_FLATPAK_REPO_BASE_URL", "https://repo.getartcraft.com");
  let trimmed_base = base_url.trim_end_matches('/');

  let destination_url = if relative_path.is_empty() { format!("{trimmed_base}/") } else { format!("{trimmed_base}/{}", relative_path.trim_start_matches('/')) };

  let max_age = if relative_path.starts_with("objects/") || relative_path.starts_with("deltas/") { DEFAULT_OBJECT_CACHE_SECS } else { METADATA_CACHE_SECS };

  HttpResponse::TemporaryRedirect().insert_header((LOCATION, destination_url)).insert_header(CacheControl(vec![CacheDirective::Public, CacheDirective::MaxAge(max_age)])).finish()
}

#[cfg(test)]
mod tests {
  use super::*;
  use actix_web::http::StatusCode;

  #[actix_web::test]
  async fn redirects_with_307_and_location() {
    let resp = get_craft_apps_flatpak_repo_redirect_handler(web::Path::from("summary".to_string())).await;
    assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
    let loc = resp.headers().get(LOCATION).expect("Location header");
    assert_eq!(loc, "https://repo.getartcraft.com/summary");
  }

  #[actix_web::test]
  async fn redirects_objects_path_with_307() {
    let resp = get_craft_apps_flatpak_repo_redirect_handler(web::Path::from("objects/01/23.filez".to_string())).await;
    assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
    let loc = resp.headers().get(LOCATION).expect("Location header");
    assert_eq!(loc, "https://repo.getartcraft.com/objects/01/23.filez");
  }

  #[actix_web::test]
  async fn rejects_flatpakrepo_under_repo_path() {
    let resp = get_craft_apps_flatpak_repo_redirect_handler(web::Path::from("artcraft.flatpakrepo".to_string())).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
  }
}
