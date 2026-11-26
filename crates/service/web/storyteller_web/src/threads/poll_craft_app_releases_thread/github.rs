//! The requests to GitHub for an app's releases, and reading the answers.
//!
//! `GET /repos/{repo}/releases?per_page=100`, revalidated with the ETag: one request per app and
//! interval while nothing changed. When the first page changed (a `200`), the next pages are read
//! too while each is full, up to [`MAX_PAGES`]: the app's whole release history, which
//! `GET /v1/craft_apps/{app}/releases` serves (choosing a version, going back to one, nightly
//! discovery). [`sort_releases`] picks from it the stable release and the beta or release
//! candidate with the highest version, the newest nightly and the recent version list that
//! `release_info` serves.
//! Not `/releases/latest`: it never returns a pre-release, and GitHub lets a nightly without the
//! pre-release flag be the latest release.

use std::time::Duration;

use chrono::{DateTime, Utc};
use wreq::header::{ACCEPT, AUTHORIZATION, ETAG, IF_NONE_MATCH, RETRY_AFTER, USER_AGENT};
use wreq::StatusCode;
use serde_derive::Deserialize;

use artcraft_api_defs::craft_apps::release_info::{
  CraftAppArch, CraftAppAssetPlatform, CraftAppOs, CraftAppPackageType, CraftAppRelease, CraftAppReleaseAsset,
};

use crate::state::craft_apps_release_cache::{CraftAppHistory, CraftAppReleases};

const API_BASE: &str = "https://api.github.com";
/// Releases asked for in one request, GitHub's most. A page is up to 10 MB from GitHub today
/// (each release carries its notes and ~50 files), for an app with daily nightlies.
pub const RELEASES_PER_PAGE: usize = 100;
const MAX_NOTES_BYTES: usize = 64 * 1024;
const MAX_ASSETS: usize = 200;
/// The most pages read: the history stops (and is marked incomplete) after this many releases.
pub const MAX_PAGES: usize = 10;
/// The most entries in the version list `release_info` serves; the whole history is per app.
pub const MAX_VERSION_LIST: usize = 30;
/// A page of releases.
const MAX_LIST_BODY_BYTES: usize = 32 * 1024 * 1024;

/// What one fetch produced.
#[derive(Debug)]
pub enum FetchOutcome {
  /// The app's releases (empty: the repository has no published release) and its history.
  Fetched { releases: CraftAppReleases, history: CraftAppHistory, etag: Option<String> },
  /// `If-None-Match` matched: what we have is current.
  NotModified,
  /// GitHub refused: out of quota until this time, if it said.
  RateLimited { retry_at: Option<DateTime<Utc>> },
  /// Network error, unexpected status or unreadable answer. Nothing is stored.
  Failed(String),
}

/// What one request to GitHub produced, before reading the body.
enum Response {
  Body { body: Vec<u8>, etag: Option<String> },
  NotFound,
  NotModified,
  RateLimited { retry_at: Option<DateTime<Utc>> },
  Failed(String),
}

pub fn build_client() -> wreq::Result<wreq::Client> {
  wreq::Client::builder()
      .connect_timeout(Duration::from_secs(10))
      .timeout(Duration::from_secs(30))
      .build()
}

/// `repo`'s releases: the first page, revalidated with `etag`, then the next pages while
/// [`read_more`] says so. `token` is optional: without one the requests are unauthenticated. All
/// or nothing: if a later page fails, so does the fetch, and [`settle`] fails one that found no
/// stable release in an incomplete history, rather than storing "no release" for an app that has
/// one further back.
pub async fn fetch_releases(client: &wreq::Client, repo: &str, etag: Option<&str>, token: Option<&str>) -> FetchOutcome {
  let (body, etag) = match send(client, &list_url(repo, 1), etag, token).await {
    Response::Body { body, etag } => (body, etag),
    Response::NotFound => {
      let history = CraftAppHistory { etag: None, complete: true, releases: Vec::new(), fetched_at: Some(Utc::now()) };
      return FetchOutcome::Fetched { releases: CraftAppReleases::default(), history, etag: None };
    }
    Response::NotModified => return FetchOutcome::NotModified,
    Response::RateLimited { retry_at } => return FetchOutcome::RateLimited { retry_at },
    Response::Failed(why) => return FetchOutcome::Failed(why),
  };
  let (mut all, mut listed) = match parse_release_list(&body, repo) {
    Ok(read) => read,
    Err(why) => return FetchOutcome::Failed(why),
  };
  let mut page = 1;
  while read_more(listed, page) {
    page += 1;
    let body = match send(client, &list_url(repo, page), None, token).await {
      Response::Body { body, .. } => body,
      // An empty page ends the list; a missing one leaves it unknown, so nothing is stored.
      Response::NotFound => return FetchOutcome::Failed(format!("page {page} of the release list is missing")),
      Response::NotModified => return FetchOutcome::Failed("unexpected 304 for a later page".into()),
      Response::RateLimited { retry_at } => return FetchOutcome::RateLimited { retry_at },
      Response::Failed(why) => return FetchOutcome::Failed(why),
    };
    match parse_release_list(&body, repo) {
      Ok((more, n)) => {
        all.extend(more);
        listed = n;
      }
      Err(why) => return FetchOutcome::Failed(why),
    }
  }
  match settle(all, listed) {
    Ok((releases, history)) => {
      let history = CraftAppHistory { etag: etag.clone(), fetched_at: Some(Utc::now()), ..history };
      FetchOutcome::Fetched { releases, history, etag }
    }
    Err(why) => FetchOutcome::Failed(why),
  }
}

/// Whether to read the page after `page`: the last one read held `listed` releases, so a full one
/// may have more behind it, up to [`MAX_PAGES`].
pub fn read_more(listed: usize, page: usize) -> bool {
  listed >= RELEASES_PER_PAGE && page < MAX_PAGES
}

/// What the pages read say, newest first, the last of them holding `listed` releases: the
/// channels and the history (complete when the last page wasn't full). Fails when the history is
/// incomplete and has no stable release, which is then further back.
pub fn settle(mut all: Vec<CraftAppRelease>, listed: usize) -> Result<(CraftAppReleases, CraftAppHistory), String> {
  // A release published between two page reads moves the last one of a page onto the next: keep
  // each tag once, where it first appeared.
  let mut seen = std::collections::HashSet::new();
  all.retain(|release| seen.insert(release.tag.clone()));
  let complete = listed < RELEASES_PER_PAGE;
  let releases = sort_releases(&all);
  if releases.latest_release.is_none() && !complete {
    return Err(format!("no stable release among the newest {} releases", MAX_PAGES * RELEASES_PER_PAGE));
  }
  Ok((releases, CraftAppHistory { etag: None, complete, releases: all, fetched_at: None }))
}

/// Sends one GET to GitHub and sorts out the answer. A body larger than a page may be fails.
async fn send(client: &wreq::Client, url: &str, etag: Option<&str>, token: Option<&str>) -> Response {
  let request = build_request(client, url, etag, token);
  let mut response = match request.send().await {
    Ok(r) => r,
    Err(err) => return Response::Failed(format!("request failed: {err}")),
  };
  let status = response.status();
  let header = |name: &str| response.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
  match status {
    StatusCode::NOT_MODIFIED => Response::NotModified,
    StatusCode::NOT_FOUND => Response::NotFound,
    StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => {
      let remaining = header("x-ratelimit-remaining");
      let reset = header("x-ratelimit-reset").and_then(|r| r.parse::<i64>().ok()).and_then(|s| DateTime::from_timestamp(s, 0));
      let retry_after = header(RETRY_AFTER.as_str())
          .and_then(|r| r.parse::<i64>().ok())
          .map(|s| Utc::now() + chrono::Duration::seconds(s.clamp(0, 86_400)));
      if status == StatusCode::TOO_MANY_REQUESTS || remaining.as_deref() == Some("0") || retry_after.is_some() {
        Response::RateLimited { retry_at: retry_after.or(reset) }
      } else {
        Response::Failed(format!("HTTP {status}"))
      }
    }
    StatusCode::OK => {
      let etag = header(ETAG.as_str());
      // Read in pieces and stopped at the cap: a compressed answer announces no decoded length.
      let mut body = Vec::new();
      loop {
        match response.chunk().await {
          Ok(Some(piece)) if body.len().saturating_add(piece.len()) <= MAX_LIST_BODY_BYTES => body.extend_from_slice(&piece),
          Ok(Some(_)) => return Response::Failed(format!("answer larger than {MAX_LIST_BODY_BYTES} bytes")),
          Ok(None) => return Response::Body { body, etag },
          Err(err) => return Response::Failed(format!("unreadable answer: {err}")),
        }
      }
    }
    other => Response::Failed(format!("HTTP {other}")),
  }
}

/// Page `page` (from 1) of `repo`'s releases. The first page's URL is the one its ETag is for.
fn list_url(repo: &str, page: usize) -> String {
  if page <= 1 {
    format!("{API_BASE}/repos/{repo}/releases?per_page={RELEASES_PER_PAGE}")
  } else {
    format!("{API_BASE}/repos/{repo}/releases?per_page={RELEASES_PER_PAGE}&page={page}")
  }
}

/// The request for `url`. Sends `Authorization: Bearer` only for a non-blank token (an
/// authenticated `304` doesn't count against the rate limit); never an empty header.
fn build_request(client: &wreq::Client, url: &str, etag: Option<&str>, token: Option<&str>) -> wreq::RequestBuilder {
  let mut request = client
      .get(url)
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

/// Reads a page of releases (newest first), and says how many the page listed. A release that
/// can't be read (a draft, no tag, the wrong shape) is skipped; only an answer that isn't a list
/// fails.
pub fn parse_release_list(body: &[u8], repo: &str) -> Result<(Vec<CraftAppRelease>, usize), String> {
  let raw: Vec<serde_json::Value> = serde_json::from_slice(body).map_err(|e| format!("unreadable release list: {e}"))?;
  let listed = raw.len();
  let releases = raw
      .into_iter()
      .take(RELEASES_PER_PAGE)
      .filter_map(|value| serde_json::from_value::<GithubRelease>(value).ok().and_then(|r| read_release(r, repo).ok()))
      .collect();
  Ok((releases, listed))
}

/// Sorts an app's releases (newest first, as GitHub lists them) into what `release_info` serves:
/// the stable release and beta with the highest version (as ArtCraft Launcher ranks them, so a
/// backport published after a newer release doesn't become the latest), the most recently
/// published nightly, and the first [`MAX_VERSION_LIST`] ordinary releases.
pub fn sort_releases(all: &[CraftAppRelease]) -> CraftAppReleases {
  let mut releases = CraftAppReleases::default();
  for release in all {
    if is_nightly_tag(&release.tag) {
      releases.latest_nightly.get_or_insert_with(|| release.clone());
      continue;
    }
    if releases.releases.len() < MAX_VERSION_LIST {
      releases.releases.push(release.into());
    }
    let slot = if release.prerelease { &mut releases.latest_prerelease } else { &mut releases.latest_release };
    if slot.as_ref().is_none_or(|kept| is_higher(release, kept)) {
      *slot = Some(release.clone());
    }
  }
  releases
}

/// Whether `a`'s version is higher than `b`'s, by semver precedence (build metadata doesn't count).
/// A tag that isn't a version never outranks one that is, and between two that rank the same the
/// one listed first stays.
fn is_higher(a: &CraftAppRelease, b: &CraftAppRelease) -> bool {
  match (version_of(&a.tag), version_of(&b.tag)) {
    (Some(a), Some(b)) => a.cmp_precedence(&b).is_gt(),
    (Some(_), None) => true,
    _ => false,
  }
}

fn version_of(tag: &str) -> Option<semver::Version> {
  semver::Version::parse(tag.strip_prefix(['v', 'V']).unwrap_or(tag)).ok()
}

/// Whether the tag's pre-release part has a `nightly` identifier, as ArtCraft Launcher reads it
/// (`v0.50.0-nightly.20261126.166.1`, `v1.0.0-dev.nightly.1`). Not `v1.0.0-nightlyish`, and not
/// build metadata (`v1.0.0+nightly`).
pub fn is_nightly_tag(tag: &str) -> bool {
  let version = tag.strip_prefix(['v', 'V']).unwrap_or(tag);
  let version = version.split_once('+').map_or(version, |(v, _)| v);
  version.split_once('-').is_some_and(|(_, pre)| pre.split('.').any(|part| part.eq_ignore_ascii_case("nightly")))
}

/// Reads a release, keeping only downloads that really belong to `repo`.
#[cfg(test)]
fn parse_release(body: &[u8], repo: &str) -> Result<CraftAppRelease, String> {
  let raw: GithubRelease = serde_json::from_slice(body).map_err(|e| format!("unreadable release: {e}"))?;
  read_release(raw, repo)
}

/// A release GitHub described, keeping only downloads that really belong to `repo`.
fn read_release(raw: GithubRelease, repo: &str) -> Result<CraftAppRelease, String> {
  if raw.draft {
    return Err("the release is a draft".into());
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
  // A pre-release tag (`v0.7.0-rc.1`) is a pre-release even when GitHub's flag wasn't set.
  let prerelease = raw.prerelease || version_of(&tag).is_some_and(|v| !v.pre.is_empty());
  Ok(CraftAppRelease {
    version: tag.strip_prefix(['v', 'V']).unwrap_or(&tag).to_string(),
    name: raw.name.filter(|n| !n.is_empty()).map(|n| truncate(&n, 200)).unwrap_or_else(|| tag.clone()),
    tag,
    published_at: raw.published_at,
    html_url,
    prerelease,
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
    let request = build_request(&client, &list_url(REPO, 1), Some("\"etag\""), Some("ghp_x")).build().unwrap();
    assert_eq!(request.headers().get(AUTHORIZATION).unwrap(), "Bearer ghp_x");
    assert_eq!(request.headers().get(IF_NONE_MATCH).unwrap(), "\"etag\"");
  }

  #[test]
  fn without_a_token_the_request_is_unauthenticated() {
    let client = build_client().unwrap();
    for token in [None, Some(""), Some("  ")] {
      let request = build_request(&client, &list_url(REPO, 1), None, token).build().unwrap();
      assert!(request.headers().get(AUTHORIZATION).is_none(), "{:?}", token);
      assert!(request.headers().get(IF_NONE_MATCH).is_none());
    }
  }

  #[test]
  fn asks_for_a_page_of_releases_and_later_pages_by_number() {
    assert_eq!(list_url(REPO, 1), "https://api.github.com/repos/storytold/photocraft/releases?per_page=100");
    assert_eq!(list_url(REPO, 2), "https://api.github.com/repos/storytold/photocraft/releases?per_page=100&page=2");
  }

  mod release_lists {
    use super::*;

    #[test]
    fn sorts_a_page_into_stable_beta_nightly_and_the_version_list() {
      let page = list(&[
        entry("v0.6.0-nightly.20261126.2.1", true),
        entry("v0.6.0-rc.2", true),
        entry("v0.6.0-nightly.20261125.1.1", true),
        entry("v0.5.1", false),
        entry("v0.6.0-rc.1", true),
        entry("v0.5.0", false),
      ]);
      let (all, listed) = parse_release_list(page.as_bytes(), REPO).unwrap();
      assert_eq!(listed, 6);
      assert_eq!(all.len(), 6, "the history keeps every release, nightlies too");
      let releases = sort_releases(&all);
      assert_eq!(tag(&releases.latest_release), Some("v0.5.1"));
      assert_eq!(tag(&releases.latest_prerelease), Some("v0.6.0-rc.2"));
      assert_eq!(tag(&releases.latest_nightly), Some("v0.6.0-nightly.20261126.2.1"));
      let versions: Vec<&str> = releases.releases.iter().map(|r| r.tag.as_str()).collect();
      assert_eq!(versions, ["v0.6.0-rc.2", "v0.5.1", "v0.6.0-rc.1", "v0.5.0"], "newest first, no nightlies");
      assert!(releases.releases[0].prerelease && !releases.releases[1].prerelease);
      assert_eq!(releases.latest_release.as_ref().unwrap().assets.len(), 1, "the latest ones keep their files");
    }

    #[test]
    fn a_release_candidate_without_the_prerelease_flag_is_a_beta_never_the_stable_release() {
      let page = list(&[entry("v0.7.0-rc.1", false), entry("v0.6.0", false)]);
      let (all, _) = parse_release_list(page.as_bytes(), REPO).unwrap();
      assert!(all[0].prerelease, "the history says so too");
      let releases = sort_releases(&all);
      assert_eq!(tag(&releases.latest_release), Some("v0.6.0"));
      assert_eq!(tag(&releases.latest_prerelease), Some("v0.7.0-rc.1"));
    }

    #[test]
    fn build_metadata_does_not_rank_a_release() {
      let page = list(&[entry("v0.6.0+a", false), entry("v0.6.0+z", false), entry("v0.5.0", false)]);
      let releases = sort_releases(&parse_release_list(page.as_bytes(), REPO).unwrap().0);
      assert_eq!(tag(&releases.latest_release), Some("v0.6.0+a"), "the same version: the one listed first stays");
    }

    #[test]
    fn a_nightly_without_the_prerelease_flag_is_still_a_nightly_never_the_stable_release() {
      let page = list(&[entry("v0.6.0-nightly.20261126.2.1", false), entry("v0.5.0", false)]);
      let releases = sort_releases(&parse_release_list(page.as_bytes(), REPO).unwrap().0);
      assert_eq!(tag(&releases.latest_release), Some("v0.5.0"));
      assert_eq!(tag(&releases.latest_nightly), Some("v0.6.0-nightly.20261126.2.1"));
      assert_eq!(releases.releases.len(), 1);
    }

    #[test]
    fn drafts_and_unreadable_entries_are_skipped_not_fatal() {
      let page = format!(
        "[{}, {}, {}, {}]",
        r#"{"tag_name":"v0.7.0","draft":true,"prerelease":false}"#,
        r#"{"tag_name":7}"#,
        r#""not a release""#,
        entry("v0.5.0", false),
      );
      let (all, listed) = parse_release_list(page.as_bytes(), REPO).unwrap();
      assert_eq!(listed, 4, "a skipped entry still counts toward a full page");
      assert_eq!(all.len(), 1);
      assert_eq!(tag(&sort_releases(&all).latest_release), Some("v0.5.0"));
    }

    #[test]
    fn an_answer_that_is_not_a_list_fails() {
      assert!(parse_release_list(b"<html>", REPO).is_err());
      assert!(parse_release_list(entry("v0.5.0", false).as_bytes(), REPO).is_err());
    }

    #[test]
    fn an_empty_page_is_a_complete_history_without_releases() {
      let (all, listed) = parse_release_list(b"[]", REPO).unwrap();
      assert!(!read_more(listed, 1));
      let (releases, history) = settle(all, listed).unwrap();
      assert!(releases.latest_release.is_none() && releases.releases.is_empty());
      assert!(history.complete && history.releases.is_empty());
    }

    #[test]
    fn later_pages_are_read_while_each_is_full_up_to_the_cap() {
      assert!(read_more(RELEASES_PER_PAGE, 1));
      assert!(!read_more(RELEASES_PER_PAGE - 1, 1), "a page that isn't full is the last one");
      assert!(read_more(RELEASES_PER_PAGE, MAX_PAGES - 1));
      assert!(!read_more(RELEASES_PER_PAGE, MAX_PAGES), "the cap");
    }

    /// Nightlies without the pre-release flag (one of which GitHub may call the latest release)
    /// don't count as stable: with the history cut off and no stable release in it, the fetch
    /// fails instead of storing "no release"; with the whole history read, there truly is none.
    #[test]
    fn no_stable_release_fails_only_when_the_history_is_cut_off() {
      let nightlies: Vec<String> = (0..RELEASES_PER_PAGE).map(|i| entry(&format!("v0.6.0-nightly.202611{:02}.{i}.1", i % 28 + 1), false)).collect();
      let (all, listed) = parse_release_list(list(&nightlies).as_bytes(), REPO).unwrap();
      assert!(settle(all.clone(), listed).is_err());
      let (releases, history) = settle(all, 3).unwrap();
      assert!(releases.latest_release.is_none() && history.complete);
    }

    /// A backport published after a newer release doesn't replace it as the latest, and the same
    /// holds for betas; the version list keeps GitHub's order.
    #[test]
    fn the_latest_stable_release_and_beta_are_the_highest_versions() {
      let page = list(&[entry("v0.4.3", false), entry("v0.6.0-rc.1", true), entry("v0.5.0", false), entry("v0.6.0-rc.2", true), entry("latest", false)]);
      let releases = sort_releases(&parse_release_list(page.as_bytes(), REPO).unwrap().0);
      assert_eq!(tag(&releases.latest_release), Some("v0.5.0"));
      assert_eq!(tag(&releases.latest_prerelease), Some("v0.6.0-rc.2"));
      assert_eq!(releases.releases[0].tag, "v0.4.3");
    }

    #[test]
    fn a_release_seen_on_two_pages_is_kept_once() {
      let (mut all, _) = parse_release_list(list(&[entry("v0.6.0", false), entry("v0.5.0", false)]).as_bytes(), REPO).unwrap();
      let (next, listed) = parse_release_list(list(&[entry("v0.5.0", false), entry("v0.4.0", false)]).as_bytes(), REPO).unwrap();
      all.extend(next);
      let (releases, history) = settle(all, listed).unwrap();
      let tags: Vec<&str> = history.releases.iter().map(|r| r.tag.as_str()).collect();
      assert_eq!(tags, ["v0.6.0", "v0.5.0", "v0.4.0"]);
      assert_eq!(releases.releases.len(), 3);
    }

    #[test]
    fn the_version_list_is_capped_but_the_history_is_not() {
      let many: Vec<String> = (0..MAX_VERSION_LIST + 5).map(|i| entry(&format!("v0.{i}.0"), false)).collect();
      let (all, listed) = parse_release_list(list(&many).as_bytes(), REPO).unwrap();
      let (releases, history) = settle(all, listed).unwrap();
      assert_eq!(releases.releases.len(), MAX_VERSION_LIST);
      assert_eq!(history.releases.len(), MAX_VERSION_LIST + 5);
    }

    #[test]
    fn reads_at_most_one_page_however_long_the_answer() {
      let many: Vec<String> = (0..RELEASES_PER_PAGE + 5).map(|i| entry(&format!("v0.{i}.0"), false)).collect();
      let (all, listed) = parse_release_list(list(&many).as_bytes(), REPO).unwrap();
      assert_eq!(listed, RELEASES_PER_PAGE + 5);
      assert_eq!(all.len(), RELEASES_PER_PAGE);
    }

    fn entry(tag: &str, prerelease: bool) -> String {
      format!(
        r#"{{"tag_name":"{tag}","name":"PhotoCraft {tag}","draft":false,"prerelease":{prerelease},
            "html_url":"https://github.com/storytold/photocraft/releases/tag/{tag}","body":"Notes",
            "assets":[{{"name":"photocraft-x-windows-x64.msi","size":7,"browser_download_url":"https://github.com/storytold/photocraft/releases/download/{tag}/photocraft-x-windows-x64.msi"}}]}}"#
      )
    }

    fn list(entries: &[String]) -> String {
      format!("[{}]", entries.join(","))
    }

    fn tag(release: &Option<CraftAppRelease>) -> Option<&str> {
      release.as_ref().map(|r| r.tag.as_str())
    }
  }

  #[test]
  fn reads_nightly_tags_as_the_launcher_does() {
    for tag in ["v0.50.0-nightly.20261126.166.1", "v1.0.0-dev.nightly.1", "1.0.0-NIGHTLY.3", "v1.0.0-nightly+build.5"] {
      assert!(is_nightly_tag(tag), "{}", tag);
    }
    for tag in ["v0.5.0", "v0.6.0-rc.1", "v1.0.0-nightlyish.1", "v1.0.0+nightly", "nightly", "artcraft-v0.41.1"] {
      assert!(!is_nightly_tag(tag), "{}", tag);
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
