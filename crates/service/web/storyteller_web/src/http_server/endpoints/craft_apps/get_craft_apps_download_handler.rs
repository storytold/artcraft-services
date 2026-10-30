//! Directs clients and browsers to the appropriate GitHub Releases asset download URL
//! via HTTP 302 redirect.

use std::sync::Arc;

use actix_web::http::header::{CacheControl, CacheDirective, LOCATION};
use actix_web::{web, HttpResponse};
use artcraft_api_defs::craft_apps::download::{CraftAppDownloadErrorResponse, CraftAppDownloadQuery};
use artcraft_api_defs::craft_apps::release_info::{CraftAppArch, CraftAppOs, CraftAppPackageType, CraftAppReleaseAsset};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::state::server_state::ServerState;

const REDIRECT_CACHE_SECS: u32 = 60;
const NOT_FOUND_CACHE_SECS: u32 = 30;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CraftAppDownloadPath {
  /// App machine name or alias (e.g. `photocraft`, `cadcraft`, `artcraft-launcher`).
  pub app: String,
  /// Package kind, extension, or exact asset filename (e.g. `flatpak`, `appimage`, `deb`, `msi`, `dmg`).
  pub kind: String,
}

/// Redirects to the direct GitHub Release asset download URL for the requested application
/// and package type, avoiding the need to host binaries on web application pods or R2 storage.
#[utoipa::path(
  get,
  tag = "Craft Apps",
  path = "/v1/craft_apps/download/{app}/{kind}",
  params(
    ("app" = String, Path, description = "Craft application identifier (e.g. 'photocraft', 'cadcraft', 'artcraft-launcher')"),
    ("kind" = String, Path, description = "Package kind, extension, or filename (e.g. 'flatpak', 'appimage', 'deb', 'rpm', 'dmg', 'msi', 'tar.gz', 'zip')"),
    CraftAppDownloadQuery,
  ),
  responses(
    (status = 302, description = "Redirect to GitHub Release asset download URL"),
    (status = 404, description = "Unknown app, no releases published yet, or no matching asset", body = CraftAppDownloadErrorResponse),
  ),
)]
pub async fn get_craft_apps_download_handler(server_state: web::Data<Arc<ServerState>>, path: web::Path<CraftAppDownloadPath>, query: web::Query<CraftAppDownloadQuery>) -> HttpResponse {
  let path = path.into_inner();
  let app_key = path.app.trim();
  let kind = path.kind.trim();

  let cached_app = match server_state.craft_apps_release_cache.get_app(app_key) {
    Some(app) => app,
    None => {
      return HttpResponse::NotFound().insert_header(CacheControl(vec![CacheDirective::Public, CacheDirective::MaxAge(NOT_FOUND_CACHE_SECS)])).json(CraftAppDownloadErrorResponse { success: false, error: format!("Unknown application '{app_key}'"), available_assets: None });
    },
  };

  let release = match cached_app.latest_release {
    Some(rel) => rel,
    None => {
      return HttpResponse::NotFound().insert_header(CacheControl(vec![CacheDirective::Public, CacheDirective::MaxAge(NOT_FOUND_CACHE_SECS)])).json(CraftAppDownloadErrorResponse { success: false, error: format!("No releases currently available for application '{app_key}'"), available_assets: None });
    },
  };

  let arch_filter = query.arch.as_deref();
  let os_filter = query.os.as_deref();

  match find_matching_asset(&release.assets, kind, arch_filter, os_filter) {
    Some(asset) => HttpResponse::Found().insert_header((LOCATION, asset.download_url.clone())).insert_header(CacheControl(vec![CacheDirective::Public, CacheDirective::MaxAge(REDIRECT_CACHE_SECS)])).finish(),
    None => {
      let available_assets = release.assets.iter().map(|a| a.name.clone()).collect();
      HttpResponse::NotFound().insert_header(CacheControl(vec![CacheDirective::Public, CacheDirective::MaxAge(NOT_FOUND_CACHE_SECS)])).json(CraftAppDownloadErrorResponse { success: false, error: format!("No asset found matching '{kind}' for application '{app_key}'"), available_assets: Some(available_assets) })
    },
  }
}

/// Finds the most appropriate asset in `assets` matching the specified `kind`,
/// `arch_filter`, and `os_filter`.
pub fn find_matching_asset<'a>(assets: &'a [CraftAppReleaseAsset], kind: &str, arch_filter: Option<&str>, os_filter: Option<&str>) -> Option<&'a CraftAppReleaseAsset> {
  let kind_trimmed = kind.trim();
  if kind_trimmed.is_empty() {
    return None;
  }

  // 1. Direct exact filename match
  if let Some(exact) = assets.iter().find(|a| a.name.eq_ignore_ascii_case(kind_trimmed)) {
    return Some(exact);
  }

  // 2. Filter candidates matching kind and OS
  let matching_kind: Vec<&'a CraftAppReleaseAsset> = assets.iter().filter(|a| matches_kind(a, kind_trimmed)).filter(|a| os_filter.is_none_or(|os| matches_os(a, os))).collect();

  if matching_kind.is_empty() {
    return None;
  }

  // 3. Filter or score by architecture
  if let Some(arch_str) = arch_filter.map(str::trim).filter(|s| !s.is_empty()) {
    if let Some(target_arch) = parse_arch(arch_str) {
      // Prioritize exact arch match over Universal fallback
      if let Some(exact_arch) = matching_kind.iter().find(|a| a.platform.as_ref().and_then(|p| p.arch).is_some_and(|arch| arch == target_arch)) {
        return Some(*exact_arch);
      }
      if let Some(compatible) = matching_kind.iter().find(|a| matches_arch(a, target_arch)) {
        return Some(*compatible);
      }
    } else {
      let lower = arch_str.to_lowercase();
      if let Some(matched) = matching_kind.iter().find(|a| a.name.to_lowercase().contains(&lower)) {
        return Some(*matched);
      }
    }
  }

  // 4. Default architecture selection when arch is omitted:
  // Prefer x86_64 / universal, then arm64, then x86.
  matching_kind.into_iter().max_by_key(|a| arch_priority_score(a))
}

fn matches_kind(asset: &CraftAppReleaseAsset, kind: &str) -> bool {
  let kind_lower = kind.to_lowercase();
  let k = kind_lower.trim_start_matches('.');
  let name_lower = asset.name.to_lowercase();

  match k {
    "flatpak" => asset.platform.as_ref().is_some_and(|p| p.package_type == CraftAppPackageType::Flatpak) || name_lower.ends_with(".flatpak"),
    "appimage" => asset.platform.as_ref().is_some_and(|p| p.package_type == CraftAppPackageType::AppImage) || (name_lower.ends_with(".appimage") && !name_lower.ends_with(".appimage.zsync")),
    "zsync" | "appimage.zsync" | "appimagezsync" => asset.platform.as_ref().is_some_and(|p| p.package_type == CraftAppPackageType::AppImageZsync) || name_lower.ends_with(".appimage.zsync"),
    "deb" => asset.platform.as_ref().is_some_and(|p| p.package_type == CraftAppPackageType::Deb) || name_lower.ends_with(".deb"),
    "rpm" => asset.platform.as_ref().is_some_and(|p| p.package_type == CraftAppPackageType::Rpm) || name_lower.ends_with(".rpm"),
    "dmg" => asset.platform.as_ref().is_some_and(|p| p.package_type == CraftAppPackageType::Dmg) || name_lower.ends_with(".dmg"),
    "msi" => asset.platform.as_ref().is_some_and(|p| p.package_type == CraftAppPackageType::Msi) || name_lower.ends_with(".msi"),
    "tar.gz" | "targz" | "tar" => asset.platform.as_ref().is_some_and(|p| p.package_type == CraftAppPackageType::TarGz) || name_lower.ends_with(".tar.gz"),
    "portable" | "portable-zip" | "portable.zip" => asset.platform.as_ref().is_some_and(|p| p.package_type == CraftAppPackageType::PortableZip) || name_lower.ends_with("-portable.zip"),
    "web" | "wasm" | "webzip" => asset.platform.as_ref().is_some_and(|p| p.package_type == CraftAppPackageType::WebZip) || (name_lower.contains("-web-") && name_lower.ends_with(".zip")),
    "zip" => name_lower.ends_with(".zip"),
    _ => name_lower.ends_with(&format!(".{k}")) || name_lower.ends_with(kind) || name_lower.contains(kind),
  }
}

fn matches_os(asset: &CraftAppReleaseAsset, os_str: &str) -> bool {
  let target_os = match os_str.to_lowercase().as_str() {
    "linux" => CraftAppOs::Linux,
    "windows" | "win" | "win64" | "win32" => CraftAppOs::Windows,
    "macos" | "mac" | "darwin" | "osx" => CraftAppOs::Macos,
    "freebsd" => CraftAppOs::Freebsd,
    "web" | "wasm" => CraftAppOs::Web,
    _ => return false,
  };

  if let Some(platform) = &asset.platform {
    platform.os == target_os
  } else {
    matches_os_name_fallback(&asset.name.to_lowercase(), target_os)
  }
}

fn matches_os_name_fallback(name_lower: &str, target_os: CraftAppOs) -> bool {
  match target_os {
    CraftAppOs::Windows if name_lower.ends_with(".msi") || name_lower.ends_with(".exe") => return true,
    CraftAppOs::Macos if name_lower.ends_with(".dmg") || name_lower.ends_with(".pkg") => return true,
    CraftAppOs::Linux if name_lower.ends_with(".deb") || name_lower.ends_with(".rpm") || name_lower.ends_with(".flatpak") => return true,
    _ => {}
  }

  let tokens: Vec<&str> = name_lower.split(|c: char| !c.is_ascii_alphanumeric()).filter(|s| !s.is_empty()).collect();

  match target_os {
    CraftAppOs::Linux => tokens.contains(&"linux"),
    CraftAppOs::Windows => tokens.iter().any(|&t| t == "windows" || t == "win" || t == "win64" || t == "win32"),
    CraftAppOs::Macos => tokens.iter().any(|&t| t == "macos" || t == "darwin" || t == "osx" || t == "mac"),
    CraftAppOs::Freebsd => tokens.contains(&"freebsd"),
    CraftAppOs::Web => tokens.contains(&"web") || tokens.contains(&"wasm"),
  }
}

fn parse_arch(arch_str: &str) -> Option<CraftAppArch> {
  match arch_str.to_lowercase().as_str() {
    "x64" | "x86_64" | "amd64" => Some(CraftAppArch::X64),
    "x86" | "i386" | "i686" => Some(CraftAppArch::X86),
    "arm64" | "aarch64" => Some(CraftAppArch::Arm64),
    "universal" => Some(CraftAppArch::Universal),
    _ => None,
  }
}

fn matches_arch(asset: &CraftAppReleaseAsset, target_arch: CraftAppArch) -> bool {
  if let Some(platform) = &asset.platform {
    if let Some(arch) = platform.arch {
      if arch == target_arch {
        return true;
      }
      // macOS Universal packages run on both X64 and Arm64
      if arch == CraftAppArch::Universal && (target_arch == CraftAppArch::X64 || target_arch == CraftAppArch::Arm64) {
        return true;
      }
    }
  }
  let name_lower = asset.name.to_lowercase();
  match target_arch {
    CraftAppArch::X64 => name_lower.contains("x86_64") || name_lower.contains("x64") || name_lower.contains("amd64") || name_lower.contains("universal"),
    CraftAppArch::Arm64 => name_lower.contains("aarch64") || name_lower.contains("arm64") || name_lower.contains("universal"),
    CraftAppArch::X86 => name_lower.contains("x86") || name_lower.contains("i386") || name_lower.contains("i686"),
    CraftAppArch::Universal => name_lower.contains("universal"),
  }
}

fn arch_priority_score(asset: &CraftAppReleaseAsset) -> u8 {
  if let Some(platform) = &asset.platform {
    match platform.arch {
      Some(CraftAppArch::X64) | Some(CraftAppArch::Universal) => 10,
      Some(CraftAppArch::Arm64) => 5,
      Some(CraftAppArch::X86) => 2,
      None => 1,
    }
  } else {
    let name = asset.name.to_lowercase();
    if name.contains("x86_64") || name.contains("x64") || name.contains("universal") {
      8
    } else if name.contains("aarch64") || name.contains("arm64") {
      4
    } else {
      1
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use artcraft_api_defs::craft_apps::release_info::CraftAppAssetPlatform;

  fn test_asset(name: &str, os: CraftAppOs, arch: Option<CraftAppArch>, package_type: CraftAppPackageType) -> CraftAppReleaseAsset {
    CraftAppReleaseAsset { name: name.to_string(), size_bytes: 12345, download_url: format!("https://github.com/storytold/photocraft/releases/download/v0.5.0/{name}"), sha256: None, platform: Some(CraftAppAssetPlatform { os, arch, package_type }) }
  }

  #[test]
  fn exact_filename_match_takes_priority() {
    let assets = vec![test_asset("photocraft-0.5.0-linux-x86_64.flatpak", CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::Flatpak), test_asset("photocraft-0.5.0-linux-aarch64.flatpak", CraftAppOs::Linux, Some(CraftAppArch::Arm64), CraftAppPackageType::Flatpak)];
    let matched = find_matching_asset(&assets, "photocraft-0.5.0-linux-aarch64.flatpak", None, None);
    assert_eq!(matched.unwrap().name, "photocraft-0.5.0-linux-aarch64.flatpak");
  }

  #[test]
  fn default_flatpak_selects_x86_64() {
    let assets = vec![test_asset("photocraft-0.5.0-linux-aarch64.flatpak", CraftAppOs::Linux, Some(CraftAppArch::Arm64), CraftAppPackageType::Flatpak), test_asset("photocraft-0.5.0-linux-x86_64.flatpak", CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::Flatpak)];
    let matched = find_matching_asset(&assets, "flatpak", None, None);
    assert_eq!(matched.unwrap().name, "photocraft-0.5.0-linux-x86_64.flatpak");
  }

  #[test]
  fn flatpak_respects_arch_filter() {
    let assets = vec![test_asset("photocraft-0.5.0-linux-x86_64.flatpak", CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::Flatpak), test_asset("photocraft-0.5.0-linux-aarch64.flatpak", CraftAppOs::Linux, Some(CraftAppArch::Arm64), CraftAppPackageType::Flatpak)];
    let matched_arm = find_matching_asset(&assets, "flatpak", Some("aarch64"), None);
    assert_eq!(matched_arm.unwrap().name, "photocraft-0.5.0-linux-aarch64.flatpak");

    let matched_arm64 = find_matching_asset(&assets, "flatpak", Some("arm64"), None);
    assert_eq!(matched_arm64.unwrap().name, "photocraft-0.5.0-linux-aarch64.flatpak");

    let matched_x64 = find_matching_asset(&assets, "flatpak", Some("x86_64"), None);
    assert_eq!(matched_x64.unwrap().name, "photocraft-0.5.0-linux-x86_64.flatpak");
  }

  #[test]
  fn distinguishes_appimage_from_zsync() {
    let assets = vec![test_asset("photocraft-0.5.0-linux-x86_64.AppImage.zsync", CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::AppImageZsync), test_asset("photocraft-0.5.0-linux-x86_64.AppImage", CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::AppImage)];
    let matched_ai = find_matching_asset(&assets, "appimage", None, None);
    assert_eq!(matched_ai.unwrap().name, "photocraft-0.5.0-linux-x86_64.AppImage");

    let matched_zsync = find_matching_asset(&assets, "zsync", None, None);
    assert_eq!(matched_zsync.unwrap().name, "photocraft-0.5.0-linux-x86_64.AppImage.zsync");
  }

  #[test]
  fn universal_macos_dmg_serves_arm64_and_x64() {
    let assets = vec![test_asset("photocraft-0.5.0-macos-universal.dmg", CraftAppOs::Macos, Some(CraftAppArch::Universal), CraftAppPackageType::Dmg), test_asset("photocraft-0.5.0-windows-x64.msi", CraftAppOs::Windows, Some(CraftAppArch::X64), CraftAppPackageType::Msi)];
    let matched_mac = find_matching_asset(&assets, "dmg", None, None);
    assert_eq!(matched_mac.unwrap().name, "photocraft-0.5.0-macos-universal.dmg");

    let matched_arm = find_matching_asset(&assets, "dmg", Some("arm64"), None);
    assert_eq!(matched_arm.unwrap().name, "photocraft-0.5.0-macos-universal.dmg");

    let matched_msi = find_matching_asset(&assets, "msi", None, None);
    assert_eq!(matched_msi.unwrap().name, "photocraft-0.5.0-windows-x64.msi");
  }

  #[test]
  fn package_types_and_tarball_lookup() {
    let assets = vec![test_asset("photocraft-0.5.0-linux-x86_64.deb", CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::Deb), test_asset("photocraft-0.5.0-linux-x86_64.rpm", CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::Rpm), test_asset("photocraft-0.5.0-linux-x86_64.tar.gz", CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::TarGz), test_asset("photocraft-0.5.0-windows-x64-portable.zip", CraftAppOs::Windows, Some(CraftAppArch::X64), CraftAppPackageType::PortableZip)];
    assert_eq!(find_matching_asset(&assets, "deb", None, None).unwrap().name, "photocraft-0.5.0-linux-x86_64.deb");
    assert_eq!(find_matching_asset(&assets, "rpm", None, None).unwrap().name, "photocraft-0.5.0-linux-x86_64.rpm");
    assert_eq!(find_matching_asset(&assets, "tar.gz", None, None).unwrap().name, "photocraft-0.5.0-linux-x86_64.tar.gz");
    assert_eq!(find_matching_asset(&assets, "portable", None, None).unwrap().name, "photocraft-0.5.0-windows-x64-portable.zip");
  }

  #[test]
  fn returns_none_on_unknown_kind_or_empty_assets() {
    let assets = vec![test_asset("photocraft-0.5.0-linux-x86_64.flatpak", CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::Flatpak)];
    assert!(find_matching_asset(&assets, "exe", None, None).is_none());
    assert!(find_matching_asset(&[], "flatpak", None, None).is_none());
    assert!(find_matching_asset(&assets, "", None, None).is_none());
  }

  #[test]
  fn os_fallback_distinguishes_windows_and_darwin() {
    let darwin_asset = CraftAppReleaseAsset {
      name: "photocraft-0.5.0-darwin-x86_64.tar.gz".to_string(),
      size_bytes: 12345,
      download_url: "https://example.com/darwin".to_string(),
      sha256: None,
      platform: None,
    };
    let win_asset = CraftAppReleaseAsset {
      name: "photocraft-0.5.0-win-x64.zip".to_string(),
      size_bytes: 12345,
      download_url: "https://example.com/win".to_string(),
      sha256: None,
      platform: None,
    };
    let macro_asset = CraftAppReleaseAsset {
      name: "photocraft-macro-tools-0.5.0-linux.tar.gz".to_string(),
      size_bytes: 12345,
      download_url: "https://example.com/macro".to_string(),
      sha256: None,
      platform: None,
    };

    assert!(matches_os(&darwin_asset, "macos"));
    assert!(matches_os(&darwin_asset, "darwin"));
    assert!(!matches_os(&darwin_asset, "windows"));
    assert!(!matches_os(&darwin_asset, "win"));

    assert!(matches_os(&win_asset, "windows"));
    assert!(matches_os(&win_asset, "win"));
    assert!(!matches_os(&win_asset, "macos"));
    assert!(!matches_os(&win_asset, "darwin"));

    assert!(matches_os(&macro_asset, "linux"));
    assert!(!matches_os(&macro_asset, "macos"));
    assert!(!matches_os(&macro_asset, "mac"));
  }
}
