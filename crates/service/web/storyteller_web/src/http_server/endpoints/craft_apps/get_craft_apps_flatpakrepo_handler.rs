//! Serves .flatpakrepo configuration files for ArtCraft Suite and individual Craft Apps.

use std::sync::Arc;

use actix_web::http::header::{CacheControl, CacheDirective, ContentDisposition, DispositionParam, DispositionType};
use actix_web::{web, HttpResponse};
use artcraft_api_defs::craft_apps::flatpak::CraftAppFlatpakRepoConfig;

use crate::state::server_state::ServerState;

const FLATPAK_MIME_TYPE: &str = "application/vnd.flatpak.repo; charset=utf-8";
const CACHE_MAX_AGE_SECS: u32 = 3600;

/// Serves a `.flatpakrepo` configuration file for the entire ArtCraft Suite
/// (`artcraft.flatpakrepo`) or an individual Craft app (`<app>.flatpakrepo`).
#[utoipa::path(
  get,
  tag = "Craft Apps",
  path = "/v1/craft_apps/flatpak/{target}",
  params(
    ("target" = String, Path, description = "Target .flatpakrepo filename, e.g. 'artcraft.flatpakrepo' or 'photocraft.flatpakrepo'"),
  ),
  responses(
    (status = 200, description = "Flatpak repository definition file (.flatpakrepo)"),
    (status = 404, description = "Unknown app or invalid filename extension"),
  ),
)]
pub async fn get_craft_apps_flatpakrepo_handler(server_state: web::Data<Arc<ServerState>>, target: web::Path<String>) -> HttpResponse {
  handle_craft_apps_flatpakrepo(&server_state.craft_apps_release_cache, target.as_str())
}

/// Core logic for handling `.flatpakrepo` file requests.
pub fn handle_craft_apps_flatpakrepo(cache: &crate::state::craft_apps_release_cache::CraftAppsReleaseCache, target_filename: &str) -> HttpResponse {
  if !target_filename.ends_with(".flatpakrepo") {
    return HttpResponse::NotFound().finish();
  }

  let stem = target_filename.strip_suffix(".flatpakrepo").unwrap_or("");
  if !is_valid_stem(stem) {
    return HttpResponse::NotFound().finish();
  }

  let repo_base_url = easyenv::get_env_string_or_default("CRAFT_APPS_FLATPAK_REPO_BASE_URL", "https://repo.getartcraft.com");
  let website_base_url = easyenv::get_env_string_or_default("CRAFT_APPS_WEBSITE_BASE_URL", "https://getartcraft.com");
  let default_gpg_key = format!("{}/keys/flatpak.gpg", website_base_url.trim_end_matches('/'));
  let gpg_key_url = easyenv::get_env_string_or_default("CRAFT_APPS_FLATPAK_GPG_KEY_URL", &default_gpg_key);

  let content = if stem == "artcraft" || stem == "suite" {
    CraftAppFlatpakRepoConfig::suite_config(&repo_base_url, &website_base_url, Some(gpg_key_url)).to_flatpakrepo_content()
  } else {
    let (app_key, app) = match cache.get_app_with_key(stem) {
      Some(entry) => entry,
      None => return HttpResponse::NotFound().finish(),
    };

    let app_env_key = app_key.to_uppercase().replace('-', "_");
    let app_gpg_key = easyenv::get_env_string_optional(&format!("CRAFT_APPS_FLATPAK_GPG_KEY_URL_{app_env_key}")).unwrap_or(gpg_key_url);
    let app_repo_url = format!("{}/{}", repo_base_url.trim_end_matches('/'), app_key);
    let app_homepage = format!("{}/apps/{}", website_base_url.trim_end_matches('/'), app_key);
    CraftAppFlatpakRepoConfig::app_config(&app_key, &app.name, &app_repo_url, &app_homepage, Some(app_gpg_key)).to_flatpakrepo_content()
  };

  let disposition = ContentDisposition { disposition: DispositionType::Inline, parameters: vec![DispositionParam::Filename(target_filename.to_string())] };

  HttpResponse::Ok().content_type(FLATPAK_MIME_TYPE).insert_header(CacheControl(vec![CacheDirective::Public, CacheDirective::MaxAge(CACHE_MAX_AGE_SECS)])).insert_header(disposition).body(content)
}

fn is_valid_stem(stem: &str) -> bool {
  !stem.is_empty() && stem.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
  use super::*;
  use actix_web::http::StatusCode;
  use crate::state::craft_apps_release_cache::CraftAppsReleaseCache;

  #[test]
  fn validates_stems() {
    assert!(is_valid_stem("photocraft"));
    assert!(is_valid_stem("artcraft-launcher"));
    assert!(is_valid_stem("cadcraft-1"));
    assert!(!is_valid_stem(""));
    assert!(!is_valid_stem("PhotoCraft"));
    assert!(!is_valid_stem("photo\ncraft"));
    assert!(!is_valid_stem("photo craft"));
    assert!(!is_valid_stem("photo/craft"));
    assert!(!is_valid_stem("photo%0Acraft"));
  }

  #[test]
  fn generates_suite_and_app_repo_content() {
    let suite = CraftAppFlatpakRepoConfig::suite_config("https://repo.getartcraft.com", "https://getartcraft.com", Some("https://getartcraft.com/keys/flatpak.gpg".to_string()));
    let suite_ini = suite.to_flatpakrepo_content();
    assert!(suite_ini.contains("[Flatpak Repo]"));
    assert!(suite_ini.contains("Title=ArtCraft"));
    assert!(suite_ini.contains("Url=https://repo.getartcraft.com/"));
    assert!(suite_ini.contains("GPGKey=https://getartcraft.com/keys/flatpak.gpg"));

    let app = CraftAppFlatpakRepoConfig::app_config("cadcraft", "CADCraft", "https://repo.getartcraft.com/cadcraft", "https://getartcraft.com/apps/cadcraft", Some("https://getartcraft.com/keys/flatpak.gpg".to_string()));
    let app_ini = app.to_flatpakrepo_content();
    assert!(app_ini.contains("Title=CADCraft"));
    assert!(app_ini.contains("Url=https://repo.getartcraft.com/cadcraft/"));
    assert!(app_ini.contains("Homepage=https://getartcraft.com/apps/cadcraft"));
    assert!(app_ini.contains("GPGKey=https://getartcraft.com/keys/flatpak.gpg"));
  }

  #[actix_web::test]
  async fn flatpakrepo_suite_and_app_responses() {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&[
      ("photocraft".into(), "PhotoCraft".into(), "storytold/photocraft".into()),
      ("artcraft-launcher".into(), "ArtCraft Launcher".into(), "storytold/craft-launcher".into()),
    ]);

    // Suite returns 200 with GPGKey
    let resp = handle_craft_apps_flatpakrepo(&cache, "artcraft.flatpakrepo");
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    assert!(body_str.contains("Title=ArtCraft"));
    assert!(body_str.contains("GPGKey="));

    // Known app returns 200 with GPGKey and cached app name
    let resp = handle_craft_apps_flatpakrepo(&cache, "photocraft.flatpakrepo");
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    assert!(body_str.contains("Title=PhotoCraft"));
    assert!(body_str.contains("GPGKey="));

    // Unknown app returns 404
    let resp = handle_craft_apps_flatpakrepo(&cache, "anything.flatpakrepo");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Missing .flatpakrepo extension returns 404
    let resp = handle_craft_apps_flatpakrepo(&cache, "photocraft");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Newline injection attempt returns 404
    let resp = handle_craft_apps_flatpakrepo(&cache, "photocraft\nGPGKey=evil.flatpakrepo");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
  }
}
