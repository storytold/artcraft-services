//! One request to GitHub for an app's latest release, and reading the answer.

use std::time::Duration;

use chrono::{DateTime, Utc};
use wreq::header::{ACCEPT, AUTHORIZATION, ETAG, IF_NONE_MATCH, RETRY_AFTER, USER_AGENT};
use wreq::StatusCode;
use serde_derive::Deserialize;

use artcraft_api_defs::craft_apps::release_info::{
  CraftAppArch, CraftAppAssetPlatform, CraftAppOs, CraftAppPackageType, CraftAppRelease, CraftAppReleaseAsset,
};

const API_BASE: &str = "https://api.github.com";
const MAX_NOTES_BYTES: usize = 64 * 1024;
const MAX_ASSETS: usize = 200;
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

/// What one request produced.
#[derive(Debug)]
pub enum FetchOutcome {
  /// The latest release (`None`: the repository has no published release).
  Fetched { release: Option<CraftAppRelease>, etag: Option<String> },
  /// `If-None-Match` matched: what we have is current.
  NotModified,
  /// GitHub refused: out of quota until this time, if it said.
  RateLimited { retry_at: Option<DateTime<Utc>> },
  /// Network error, unexpected status or unreadable answer. Nothing is stored.
  Failed(String),
}

pub fn build_client() -> wreq::Result<wreq::Client> {
  wreq::Client::builder()
      .connect_timeout(Duration::from_secs(10))
      .timeout(Duration::from_secs(30))
      .build()
}

/// `GET /repos/{repo}/releases/latest`, revalidated with `etag`. `token` is optional: without
/// one the request is unauthenticated.
pub async fn fetch_latest_release(client: &wreq::Client, repo: &str, etag: Option<&str>, token: Option<&str>) -> FetchOutcome {
  let request = build_request(client, repo, etag, token);
  let response = match request.send().await {
    Ok(r) => r,
    Err(err) => return FetchOutcome::Failed(format!("request failed: {err}")),
  };
  let status = response.status();
  let header = |name: &str| response.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
  match status {
    StatusCode::NOT_MODIFIED => FetchOutcome::NotModified,
    StatusCode::NOT_FOUND => FetchOutcome::Fetched { release: None, etag: None },
    StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => {
      let remaining = header("x-ratelimit-remaining");
      let reset = header("x-ratelimit-reset").and_then(|r| r.parse::<i64>().ok()).and_then(|s| DateTime::from_timestamp(s, 0));
      let retry_after = header(RETRY_AFTER.as_str())
          .and_then(|r| r.parse::<i64>().ok())
          .map(|s| Utc::now() + chrono::Duration::seconds(s.clamp(0, 86_400)));
      if status == StatusCode::TOO_MANY_REQUESTS || remaining.as_deref() == Some("0") || retry_after.is_some() {
        FetchOutcome::RateLimited { retry_at: retry_after.or(reset) }
      } else {
        FetchOutcome::Failed(format!("HTTP {status}"))
      }
    }
    StatusCode::OK => {
      let etag = header(ETAG.as_str());
      let body = match response.bytes().await {
        Ok(b) if b.len() <= MAX_BODY_BYTES => b,
        Ok(b) => return FetchOutcome::Failed(format!("answer too large ({} bytes)", b.len())),
        Err(err) => return FetchOutcome::Failed(format!("unreadable answer: {err}")),
      };
      match parse_release(&body, repo) {
        Ok(release) => FetchOutcome::Fetched { release: Some(release), etag },
        Err(err) => FetchOutcome::Failed(err),
      }
    }
    other => FetchOutcome::Failed(format!("HTTP {other}")),
  }
}

/// The request for `repo`'s latest release. Sends `Authorization: Bearer` only for a non-blank
/// token (an authenticated `304` doesn't count against the rate limit); never an empty header.
fn build_request(client: &wreq::Client, repo: &str, etag: Option<&str>, token: Option<&str>) -> wreq::RequestBuilder {
  let url = format!("{API_BASE}/repos/{repo}/releases/latest");
  let mut request = client
      .get(&url)
      .header(USER_AGENT, "storyteller-web (craft apps release poller)")
      .header(ACCEPT, "application/vnd.github+json")
      .header("X-GitHub-Api-Version", "2022-11-28");
  if let Some(etag) = etag {
    request = request.header(IF_NONE_MATCH, etag);
  }
  if let Some(token) = token.map(str::trim).filter(|t| !t.is_empty()) {
    request = request.header(AUTHORIZATION, format!("Bearer {token}"));
  }
  request
}

/// The parts of GitHub's release object we use. Everything optional: GitHub's answer is input.
#[derive(Deserialize)]
struct GithubRelease {
  #[serde(default)]
  tag_name: Option<String>,
  #[serde(default)]
  name: Option<String>,
  #[serde(default)]
  html_url: Option<String>,
  #[serde(default)]
  published_at: Option<DateTime<Utc>>,
  #[serde(default)]
  prerelease: bool,
  #[serde(default)]
  draft: bool,
  #[serde(default)]
  body: Option<String>,
  #[serde(default)]
  assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
  #[serde(default)]
  name: Option<String>,
  #[serde(default)]
  size: u64,
  #[serde(default)]
  browser_download_url: Option<String>,
  /// `sha256:<hex>` on assets uploaded since mid-2025.
  #[serde(default)]
  digest: Option<String>,
}

/// Reads a release, keeping only downloads that really belong to `repo`.
pub fn parse_release(body: &[u8], repo: &str) -> Result<CraftAppRelease, String> {
  let raw: GithubRelease = serde_json::from_slice(body).map_err(|e| format!("unreadable release: {e}"))?;
  if raw.draft {
    return Err("the latest release is a draft".into());
  }
  let tag = raw.tag_name.filter(|t| !t.is_empty() && t.len() <= 64).ok_or("the release has no tag")?;
  let releases_page = format!("https://github.com/{repo}/releases/");
  let html_url = raw.html_url.filter(|u| u.starts_with(&releases_page)).unwrap_or_else(|| format!("{releases_page}tag/{tag}"));
  let download_prefix = format!("https://github.com/{repo}/releases/download/");
  let assets = raw
      .assets
      .into_iter()
      .filter_map(|a| {
        let name = a.name.filter(|n| valid_file_name(n))?;
        let download_url = a.browser_download_url.filter(|u| u.starts_with(&download_prefix))?;
        let sha256 = a.digest.as_deref().and_then(|d| d.strip_prefix("sha256:")).filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())).map(str::to_ascii_lowercase);
        let platform = parse_platform(&name);
        Some(CraftAppReleaseAsset { name, size_bytes: a.size, download_url, sha256, platform })
      })
      .take(MAX_ASSETS)
      .collect();
  Ok(CraftAppRelease {
    version: tag.strip_prefix(['v', 'V']).unwrap_or(&tag).to_string(),
    name: raw.name.filter(|n| !n.is_empty()).map(|n| truncate(&n, 200)).unwrap_or_else(|| tag.clone()),
    tag,
    published_at: raw.published_at,
    html_url,
    prerelease: raw.prerelease,
    notes_markdown: raw.body.map(|b| truncate(&b, MAX_NOTES_BYTES)).unwrap_or_default(),
    assets,
  })
}

/// Reads `<app>-<version>-<os>-<arch>[-portable].<ext>`, `<app>-cli-<version>-macos-universal.zip`
/// and `<app>-web-<version>.zip`. `None` for anything else (checksums, unknown files).
pub fn parse_platform(name: &str) -> Option<CraftAppAssetPlatform> {
  if name.contains("-web-") && name.ends_with(".zip") {
    return Some(CraftAppAssetPlatform { os: CraftAppOs::Web, arch: None, package_type: CraftAppPackageType::WebZip });
  }
  const KINDS: [(&str, CraftAppPackageType); 10] = [
    ("-portable.zip", CraftAppPackageType::PortableZip),
    (".AppImage.zsync", CraftAppPackageType::AppImageZsync),
    (".AppImage", CraftAppPackageType::AppImage),
    (".tar.gz", CraftAppPackageType::TarGz),
    (".msi", CraftAppPackageType::Msi),
    (".dmg", CraftAppPackageType::Dmg),
    (".deb", CraftAppPackageType::Deb),
    (".rpm", CraftAppPackageType::Rpm),
    (".flatpak", CraftAppPackageType::Flatpak),
    (".zip", CraftAppPackageType::CliZip),
  ];
  let (base, package_type) = KINDS.iter().find_map(|(suffix, kind)| name.strip_suffix(suffix).map(|b| (b, *kind)))?;
  let mut parts = base.rsplitn(3, '-');
  let (arch, os) = (parts.next()?, parts.next()?);
  let os = match os {
    "windows" => CraftAppOs::Windows,
    "macos" => CraftAppOs::Macos,
    "linux" => CraftAppOs::Linux,
    "freebsd" => CraftAppOs::Freebsd,
    _ => return None,
  };
  let arch = match arch {
    "x64" | "x86_64" => CraftAppArch::X64,
    "x86" => CraftAppArch::X86,
    "arm64" | "aarch64" => CraftAppArch::Arm64,
    "universal" => CraftAppArch::Universal,
    _ => return None,
  };
  // A bare `.zip` is only the macOS command-line tools.
  if package_type == CraftAppPackageType::CliZip && !(os == CraftAppOs::Macos && base.contains("-cli-")) {
    return None;
  }
  Some(CraftAppAssetPlatform { os, arch: Some(arch), package_type })
}

fn valid_file_name(name: &str) -> bool {
  !name.is_empty()
      && name.len() <= 200
      && !name.starts_with('.')
      && !name.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*'))
}

fn truncate(s: &str, max: usize) -> String {
  if s.len() <= max {
    return s.to_string();
  }
  let mut end = max;
  while !s.is_char_boundary(end) {
    end -= 1;
  }
  format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
  use super::*;

  const REPO: &str = "storytold/photocraft";
  const HASH: &str = "8a5edab282632443219e051e4ade2d1d5bbc671c781051bf1437897cbdfea0f1";

  fn release_json(download_url: &str) -> String {
    format!(
      r#"{{"tag_name":"v0.5.0","name":"PhotoCraft v0.5.0","draft":false,"prerelease":false,
          "html_url":"https://github.com/storytold/photocraft/releases/tag/v0.5.0",
          "published_at":"2026-10-08T14:52:50Z","body":"Notes","unknown_field":[1,2,3],
          "assets":[{{"name":"photocraft-0.5.0-windows-x64.msi","size":7,"browser_download_url":"{download_url}","digest":"sha256:{HASH}"}}]}}"#
    )
  }

  #[test]
  fn sends_the_token_as_a_bearer_header() {
    let client = build_client().unwrap();
    let request = build_request(&client, REPO, Some("\"etag\""), Some("ghp_x")).build().unwrap();
    assert_eq!(request.headers().get(AUTHORIZATION).unwrap(), "Bearer ghp_x");
    assert_eq!(request.headers().get(IF_NONE_MATCH).unwrap(), "\"etag\"");
  }

  #[test]
  fn without_a_token_the_request_is_unauthenticated() {
    let client = build_client().unwrap();
    for token in [None, Some(""), Some("  ")] {
      let request = build_request(&client, REPO, None, token).build().unwrap();
      assert!(request.headers().get(AUTHORIZATION).is_none(), "{:?}", token);
      assert!(request.headers().get(IF_NONE_MATCH).is_none());
      assert_eq!(request.uri().to_string(), "https://api.github.com/repos/storytold/photocraft/releases/latest");
    }
  }

  #[test]
  fn reads_a_release() {
    let r = parse_release(release_json("https://github.com/storytold/photocraft/releases/download/v0.5.0/photocraft-0.5.0-windows-x64.msi").as_bytes(), REPO).unwrap();
    assert_eq!((r.tag.as_str(), r.version.as_str(), r.name.as_str()), ("v0.5.0", "0.5.0", "PhotoCraft v0.5.0"));
    assert_eq!(r.assets.len(), 1);
    assert_eq!(r.assets[0].sha256.as_deref(), Some(HASH));
    let p = r.assets[0].platform.as_ref().unwrap();
    assert_eq!((p.os, p.arch, p.package_type), (CraftAppOs::Windows, Some(CraftAppArch::X64), CraftAppPackageType::Msi));
  }

  #[test]
  fn drops_downloads_that_point_elsewhere_and_rejects_garbage() {
    for url in ["https://evil.example/x.msi", "https://github.com/storytold/other/releases/download/v1/x.msi"] {
      assert!(parse_release(release_json(url).as_bytes(), REPO).unwrap().assets.is_empty(), "{url}");
    }
    assert!(parse_release(b"<html>", REPO).is_err());
    assert!(parse_release(br#"{"assets":[]}"#, REPO).is_err());
    assert!(parse_release(br#"{"tag_name":"v1.0.0","draft":true}"#, REPO).is_err());
  }

  #[test]
  fn reads_every_asset_name_the_suite_publishes() {
    let cases = [
      ("photocraft-0.5.0-macos-universal.dmg", Some((CraftAppOs::Macos, Some(CraftAppArch::Universal), CraftAppPackageType::Dmg))),
      ("photocraft-cli-0.5.0-macos-universal.zip", Some((CraftAppOs::Macos, Some(CraftAppArch::Universal), CraftAppPackageType::CliZip))),
      ("photocraft-0.5.0-windows-arm64-portable.zip", Some((CraftAppOs::Windows, Some(CraftAppArch::Arm64), CraftAppPackageType::PortableZip))),
      ("photocraft-0.5.0-windows-x86.msi", Some((CraftAppOs::Windows, Some(CraftAppArch::X86), CraftAppPackageType::Msi))),
      ("photocraft-0.5.0-linux-aarch64.AppImage", Some((CraftAppOs::Linux, Some(CraftAppArch::Arm64), CraftAppPackageType::AppImage))),
      ("photocraft-0.5.0-linux-x86_64.AppImage.zsync", Some((CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::AppImageZsync))),
      ("photocraft-0.5.0-linux-x86_64.deb", Some((CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::Deb))),
      ("photocraft-0.5.0-linux-x86_64.rpm", Some((CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::Rpm))),
      ("photocraft-0.5.0-linux-x86_64.flatpak", Some((CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::Flatpak))),
      ("photocraft-0.5.0-freebsd-x86_64.tar.gz", Some((CraftAppOs::Freebsd, Some(CraftAppArch::X64), CraftAppPackageType::TarGz))),
      ("photocraft-0.2.0-rc.1-linux-x86_64.tar.gz", Some((CraftAppOs::Linux, Some(CraftAppArch::X64), CraftAppPackageType::TarGz))),
      ("photocraft-web-0.5.0.zip", Some((CraftAppOs::Web, None, CraftAppPackageType::WebZip))),
      ("SHA256SUMS.txt", None),
      ("photocraft-0.5.0-plan9-x64.msi", None),
    ];
    for (name, want) in cases {
      let got = parse_platform(name).map(|p| (p.os, p.arch, p.package_type));
      assert_eq!(got, want, "{name}");
    }
  }
}
