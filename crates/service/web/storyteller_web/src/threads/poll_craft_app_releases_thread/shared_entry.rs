//! What the fleet shares in Redis about Craft App releases, and how it is encoded. Pure: the
//! Redis I/O is in `shared_store`.
//!
//! ## `craftApps:releases:app:<app key>` (hash, 7 day TTL, refreshed on every write)
//!
//! | Field        | Meaning                                                                        |
//! |--------------|--------------------------------------------------------------------------------|
//! | `v`          | Format version, `1`. Entries with another version are ignored (and replaced)   |
//! | `fetched_ms` | When the release was last fetched or confirmed by GitHub (Unix ms)             |
//! | `etag`       | GitHub's ETag for that answer; empty for none                                  |
//! | `release`    | The latest release as JSON (`CraftAppRelease`), `null` if the repo has none    |
//! | `failed_ms`  | When a request for the app last failed (Unix ms); stale once `fetched_ms` newer |
//! | `failures`   | Failed requests in a row as of `failed_ms`                                     |
//! | `claimed_ms` | When an instance last started a request for the app (Unix ms), to skip doubles |
//! | `by`         | Hostname of the instance that wrote last, for debugging                        |
//!
//! ## `craftApps:releases:backoff` (hash, 1 day TTL; deleted after a good round)
//!
//! | Field      | Meaning                                                         |
//! |------------|-----------------------------------------------------------------|
//! | `v`        | Format version, `1`                                             |
//! | `until_ms` | No instance asks GitHub before this time (Unix ms)              |
//! | `trouble`  | Rounds in a row that were rate limited or failed entirely       |
//! | `by`       | Hostname of the instance that wrote last, for debugging         |
//!
//! Anything unreadable (wrong version, bad numbers, bad JSON) reads as absent, so it is
//! refetched and overwritten rather than trusted.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use artcraft_api_defs::craft_apps::release_info::CraftAppRelease;

use crate::threads::poll_craft_app_releases_thread::scheduling::Backoff;

pub const FORMAT_VERSION: &str = "1";

pub const FIELD_VERSION: &str = "v";
pub const FIELD_FETCHED_MS: &str = "fetched_ms";
pub const FIELD_ETAG: &str = "etag";
pub const FIELD_RELEASE: &str = "release";
pub const FIELD_FAILED_MS: &str = "failed_ms";
pub const FIELD_FAILURES: &str = "failures";
pub const FIELD_CLAIMED_MS: &str = "claimed_ms";
pub const FIELD_BY: &str = "by";
pub const FIELD_UNTIL_MS: &str = "until_ms";
pub const FIELD_TROUBLE: &str = "trouble";

/// One app's shared entry, as read from Redis.
#[derive(Clone, Debug, Default)]
pub struct SharedAppEntry {
  pub fetched: Option<SharedFetch>,
  pub failure: Option<SharedFailure>,
}

/// A successful fetch (or `304` confirmation) by some instance.
#[derive(Clone, Debug)]
pub struct SharedFetch {
  pub latest_release: Option<CraftAppRelease>,
  pub etag: Option<String>,
  pub fetched_at: DateTime<Utc>,
}

/// The last failed request by some instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedFailure {
  pub failed_at: DateTime<Utc>,
  pub consecutive_failures: u32,
}

/// Reads an app's hash. `None` for an empty hash or another format version.
pub fn decode_app_entry(fields: &HashMap<String, String>) -> Option<SharedAppEntry> {
  if fields.get(FIELD_VERSION).map(String::as_str) != Some(FORMAT_VERSION) {
    return None;
  }
  let fetched = decode_fetch(fields);
  let failure = time_field(fields, FIELD_FAILED_MS).map(|failed_at| SharedFailure {
    failed_at,
    consecutive_failures: fields.get(FIELD_FAILURES).and_then(|f| f.parse().ok()).unwrap_or(1).max(1),
  });
  Some(SharedAppEntry { fetched, failure })
}

/// The fields written for a successful fetch.
pub fn encode_fetch(latest_release: Option<&CraftAppRelease>, etag: Option<&str>, fetched_at: DateTime<Utc>, by: &str) -> Result<Vec<(&'static str, String)>, serde_json::Error> {
  Ok(vec![
    (FIELD_VERSION, FORMAT_VERSION.to_string()),
    (FIELD_FETCHED_MS, fetched_at.timestamp_millis().to_string()),
    (FIELD_ETAG, etag.unwrap_or_default().to_string()),
    (FIELD_RELEASE, serde_json::to_string(&latest_release)?),
    (FIELD_BY, by.to_string()),
  ])
}

/// The fields written for a `304`: only the time moves.
pub fn encode_unchanged(fetched_at: DateTime<Utc>, by: &str) -> Vec<(&'static str, String)> {
  vec![
    (FIELD_VERSION, FORMAT_VERSION.to_string()),
    (FIELD_FETCHED_MS, fetched_at.timestamp_millis().to_string()),
    (FIELD_BY, by.to_string()),
  ]
}

/// The fields written for a failed request.
pub fn encode_failure(failure: SharedFailure, by: &str) -> Vec<(&'static str, String)> {
  vec![
    (FIELD_VERSION, FORMAT_VERSION.to_string()),
    (FIELD_FAILED_MS, failure.failed_at.timestamp_millis().to_string()),
    (FIELD_FAILURES, failure.consecutive_failures.to_string()),
    (FIELD_BY, by.to_string()),
  ]
}

/// Reads the backoff hash; anything absent or unreadable is no backoff.
pub fn decode_backoff(fields: &HashMap<String, String>) -> Backoff {
  if fields.get(FIELD_VERSION).map(String::as_str) != Some(FORMAT_VERSION) {
    return Backoff::default();
  }
  Backoff {
    until: time_field(fields, FIELD_UNTIL_MS),
    trouble: fields.get(FIELD_TROUBLE).and_then(|t| t.parse().ok()).unwrap_or(0),
  }
}

pub fn encode_backoff(backoff: Backoff, by: &str) -> Vec<(&'static str, String)> {
  vec![
    (FIELD_VERSION, FORMAT_VERSION.to_string()),
    (FIELD_UNTIL_MS, backoff.until.map(|u| u.timestamp_millis()).unwrap_or(0).to_string()),
    (FIELD_TROUBLE, backoff.trouble.to_string()),
    (FIELD_BY, by.to_string()),
  ]
}

/// The fetch part of an app's hash; `None` unless the time, the ETag and the release all read.
fn decode_fetch(fields: &HashMap<String, String>) -> Option<SharedFetch> {
  let fetched_at = time_field(fields, FIELD_FETCHED_MS)?;
  let latest_release: Option<CraftAppRelease> = serde_json::from_str(fields.get(FIELD_RELEASE)?).ok()?;
  let etag = fields.get(FIELD_ETAG)?;
  Some(SharedFetch {
    latest_release,
    etag: (!etag.is_empty()).then(|| etag.clone()),
    fetched_at,
  })
}

/// A Unix-milliseconds field; `None` when absent, unreadable or zero.
fn time_field(fields: &HashMap<String, String>, name: &str) -> Option<DateTime<Utc>> {
  let ms = fields.get(name)?.parse::<i64>().ok().filter(|ms| *ms > 0)?;
  DateTime::from_timestamp_millis(ms)
}

#[cfg(test)]
mod tests {
  use chrono::TimeZone;

  use super::*;

  const BY: &str = "storyteller-web-abc";

  mod app_entries {
    use super::*;

    #[test]
    fn a_fetch_round_trips() {
      let release = release();
      let at = time();
      let fields = to_map(encode_fetch(Some(&release), Some("\"e1\""), at, BY).unwrap());
      let entry = decode_app_entry(&fields).unwrap();
      let fetched = entry.fetched.unwrap();
      assert_eq!((fetched.etag.as_deref(), fetched.fetched_at), (Some("\"e1\""), at));
      assert_eq!(json(&fetched.latest_release), json(&Some(release)));
      assert_eq!(entry.failure, None);
    }

    #[test]
    fn a_repo_without_release_or_etag_round_trips() {
      let fields = to_map(encode_fetch(None, None, time(), BY).unwrap());
      let fetched = decode_app_entry(&fields).unwrap().fetched.unwrap();
      assert!(fetched.latest_release.is_none());
      assert_eq!(fetched.etag, None);
    }

    #[test]
    fn a_304_moves_only_the_time() {
      let mut fields = to_map(encode_fetch(Some(&release()), Some("\"e1\""), time(), BY).unwrap());
      let later = time() + chrono::Duration::minutes(2);
      fields.extend(to_map(encode_unchanged(later, BY)));
      let fetched = decode_app_entry(&fields).unwrap().fetched.unwrap();
      assert_eq!((fetched.fetched_at, fetched.etag.as_deref()), (later, Some("\"e1\"")));
      assert_eq!(json(&fetched.latest_release), json(&Some(release())));
    }

    #[test]
    fn a_failure_round_trips_alongside_a_fetch() {
      let mut fields = to_map(encode_fetch(None, None, time(), BY).unwrap());
      let failure = SharedFailure { failed_at: time() + chrono::Duration::minutes(1), consecutive_failures: 3 };
      fields.extend(to_map(encode_failure(failure, BY)));
      let entry = decode_app_entry(&fields).unwrap();
      assert_eq!(entry.failure, Some(failure));
      assert!(entry.fetched.is_some());
    }

    #[test]
    fn garbage_reads_as_absent() {
      assert!(decode_app_entry(&HashMap::new()).is_none());
      let good = to_map(encode_fetch(Some(&release()), Some("\"e1\""), time(), BY).unwrap());
      let with = |field: &str, value: &str| {
        let mut fields = good.clone();
        fields.insert(field.to_string(), value.to_string());
        fields
      };
      assert!(decode_app_entry(&with(FIELD_VERSION, "2")).is_none(), "unknown version");
      assert!(decode_app_entry(&with(FIELD_RELEASE, "{not json")).unwrap().fetched.is_none());
      assert!(decode_app_entry(&with(FIELD_RELEASE, "{\"tag\":1}")).unwrap().fetched.is_none());
      assert!(decode_app_entry(&with(FIELD_FETCHED_MS, "yesterday")).unwrap().fetched.is_none());
      assert!(decode_app_entry(&with(FIELD_FAILED_MS, "-5")).unwrap().failure.is_none());
    }
  }

  mod backoffs {
    use super::*;

    #[test]
    fn a_backoff_round_trips() {
      let backoff = Backoff { until: Some(time()), trouble: 4 };
      assert_eq!(decode_backoff(&to_map(encode_backoff(backoff, BY))), backoff);
    }

    #[test]
    fn garbage_reads_as_no_backoff() {
      assert_eq!(decode_backoff(&HashMap::new()), Backoff::default());
      let mut fields = to_map(encode_backoff(Backoff { until: Some(time()), trouble: 4 }, BY));
      fields.insert(FIELD_VERSION.into(), "zz".into());
      assert_eq!(decode_backoff(&fields), Backoff::default());
      fields.insert(FIELD_VERSION.into(), FORMAT_VERSION.into());
      fields.insert(FIELD_UNTIL_MS.into(), "soon".into());
      fields.insert(FIELD_TROUBLE.into(), "-1".into());
      assert_eq!(decode_backoff(&fields), Backoff::default());
    }
  }

  fn time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 10, 12, 0, 0).unwrap()
  }

  fn release() -> CraftAppRelease {
    CraftAppRelease {
      tag: "v0.5.0".into(),
      version: "0.5.0".into(),
      name: "PhotoCraft v0.5.0".into(),
      published_at: Some(time()),
      html_url: "https://github.com/storytold/photocraft/releases/tag/v0.5.0".into(),
      prerelease: false,
      notes_markdown: "Notes".into(),
      assets: Vec::new(),
    }
  }

  fn json(release: &Option<CraftAppRelease>) -> String {
    serde_json::to_string(release).unwrap()
  }

  fn to_map(fields: Vec<(&'static str, String)>) -> HashMap<String, String> {
    fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
  }
}
