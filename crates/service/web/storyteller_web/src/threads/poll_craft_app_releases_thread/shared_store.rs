//! Reads and writes the fleet's shared Craft App release state in Redis (keys and fields are
//! documented in `shared_entry`).
//!
//! Writes are last-write-wins but guarded by time: a Lua script compares a timestamp field
//! before writing, so a slow instance never replaces a newer fetch with an older one. A `304`
//! moves only `fetched_ms`, and only if the stored ETag is still the one that was revalidated.
//!
//! The pool is r2d2 (blocking), so every call runs on tokio's blocking pool with a timeout:
//! Redis trouble costs the polling loop a few seconds at most, never a stuck thread of the
//! async runtime.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use r2d2::Pool;
use redis::{Client, Connection, RedisResult, Script};

use artcraft_api_defs::craft_apps::release_info::CraftAppRelease;
use redis_schema::keys::craft_apps::craft_app_release_redis_key::CraftAppReleaseRedisKey;
use redis_schema::keys::craft_apps::craft_apps_release_backoff_redis_key::CraftAppsReleaseBackoffRedisKey;

use crate::threads::poll_craft_app_releases_thread::scheduling::Backoff;
use crate::threads::poll_craft_app_releases_thread::shared_entry;
use crate::threads::poll_craft_app_releases_thread::shared_entry::{SharedAppEntry, SharedFailure};

/// Longest wait for a pooled connection.
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(3);
/// Longest wait for a whole call, connection included.
const CALL_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a claim on an app keeps other instances from requesting it too: longer than one
/// request can take (10 s to connect, 30 s for the answer), plus a margin.
pub const CLAIM_WINDOW: Duration = Duration::from_secs(60);

/// Writes `ARGV[6..]` (field/value pairs) to the hash `KEYS[1]` and sets its TTL to `ARGV[4]` ms,
/// unless the hash's time field `ARGV[1]` already holds a value greater than
/// `ARGV[2] - ARGV[3]` (new time minus a minimum gap; a gap of 0 means "never go back in time"),
/// or `ARGV[5]` is non-empty and differs from the hash's `etag`. Returns 1 if written, else 0.
/// Unreadable stored times don't block the write, so garbage gets overwritten.
const GUARDED_HSET_LUA: &str = r#"
local current = tonumber(redis.call('HGET', KEYS[1], ARGV[1]))
if current and current + tonumber(ARGV[3]) > tonumber(ARGV[2]) then
  return 0
end
if ARGV[5] ~= '' and redis.call('HGET', KEYS[1], 'etag') ~= ARGV[5] then
  return 0
end
redis.call('HSET', KEYS[1], unpack(ARGV, 6))
redis.call('PEXPIRE', KEYS[1], ARGV[4])
return 1
"#;

static GUARDED_HSET: LazyLock<Script> = LazyLock::new(|| Script::new(GUARDED_HSET_LUA));

/// Handle on the shared state. Cheap to clone.
#[derive(Clone)]
pub struct SharedStore {
  pool: Pool<Client>,
  /// Written as `by`, for debugging.
  hostname: String,
}

/// A guarded write: which time field guards it, and what to write.
struct GuardedWrite {
  key: String,
  time_field: &'static str,
  time: DateTime<Utc>,
  min_gap: Duration,
  required_etag: Option<String>,
  fields: Vec<(&'static str, String)>,
}

impl SharedStore {
  pub fn new(pool: Pool<Client>, hostname: String) -> Self {
    Self { pool, hostname }
  }

  /// The shared entries of `app_keys` (in order; `None` for absent or unreadable ones) and the
  /// fleet's backoff, in one round trip.
  pub async fn read(&self, app_keys: &[String]) -> anyhow::Result<(Vec<Option<SharedAppEntry>>, Backoff)> {
    let keys: Vec<String> = app_keys.iter().map(|k| CraftAppReleaseRedisKey::new_for_app(k).to_string()).collect();
    let mut hashes: Vec<HashMap<String, String>> = self
        .run(move |conn| {
          let mut pipe = redis::pipe();
          for key in &keys {
            pipe.hgetall(key);
          }
          pipe.hgetall(CraftAppsReleaseBackoffRedisKey::new_for_fleet().as_str());
          pipe.query(conn)
        })
        .await?;
    let backoff = hashes.pop().map(|fields| shared_entry::decode_backoff(&fields)).unwrap_or_default();
    let entries = hashes.iter().map(shared_entry::decode_app_entry).collect();
    Ok((entries, backoff))
  }

  /// Claims the next request for `app_key` at `now`. False when another instance claimed it
  /// within [`CLAIM_WINDOW`]: it is asking GitHub already, so this one doesn't. Not a lock: it
  /// is never released and simply lapses, so a crashed instance can't block anyone.
  pub async fn claim(&self, app_key: &str, now: DateTime<Utc>) -> anyhow::Result<bool> {
    self.guarded_write(GuardedWrite {
      key: CraftAppReleaseRedisKey::new_for_app(app_key).to_string(),
      time_field: shared_entry::FIELD_CLAIMED_MS,
      time: now,
      min_gap: CLAIM_WINDOW,
      required_etag: None,
      fields: vec![
        (shared_entry::FIELD_VERSION, shared_entry::FORMAT_VERSION.to_string()),
        (shared_entry::FIELD_CLAIMED_MS, now.timestamp_millis().to_string()),
        (shared_entry::FIELD_BY, self.hostname.clone()),
      ],
    }).await
  }

  /// Shares a successful fetch, unless a newer one is already shared. Returns whether written.
  pub async fn write_fetched(&self, app_key: &str, latest_release: Option<&CraftAppRelease>, etag: Option<&str>, fetched_at: DateTime<Utc>) -> anyhow::Result<bool> {
    self.guarded_write(GuardedWrite {
      key: CraftAppReleaseRedisKey::new_for_app(app_key).to_string(),
      time_field: shared_entry::FIELD_FETCHED_MS,
      time: fetched_at,
      min_gap: Duration::ZERO,
      required_etag: None,
      fields: shared_entry::encode_fetch(latest_release, etag, fetched_at, &self.hostname)?,
    }).await
  }

  /// Shares a `304` for the release with ETag `etag`: moves only the fetch time, and only if
  /// that release is still the shared one and nothing newer was shared. Returns whether written.
  pub async fn write_unchanged(&self, app_key: &str, etag: &str, fetched_at: DateTime<Utc>) -> anyhow::Result<bool> {
    if etag.is_empty() {
      return Ok(false);
    }
    self.guarded_write(GuardedWrite {
      key: CraftAppReleaseRedisKey::new_for_app(app_key).to_string(),
      time_field: shared_entry::FIELD_FETCHED_MS,
      time: fetched_at,
      min_gap: Duration::ZERO,
      required_etag: Some(etag.to_string()),
      fields: shared_entry::encode_unchanged(fetched_at, &self.hostname),
    }).await
  }

  /// Shares a failed request, unless a newer failure is already shared. Returns whether written.
  pub async fn write_failure(&self, app_key: &str, failure: SharedFailure) -> anyhow::Result<bool> {
    self.guarded_write(GuardedWrite {
      key: CraftAppReleaseRedisKey::new_for_app(app_key).to_string(),
      time_field: shared_entry::FIELD_FAILED_MS,
      time: failure.failed_at,
      min_gap: Duration::ZERO,
      required_etag: None,
      fields: shared_entry::encode_failure(failure, &self.hostname),
    }).await
  }

  /// Shares the fleet's backoff; no trouble deletes it.
  pub async fn write_backoff(&self, backoff: Backoff) -> anyhow::Result<()> {
    let key = CraftAppsReleaseBackoffRedisKey::new_for_fleet().to_string();
    if backoff.trouble == 0 {
      return self.run(move |conn| redis::cmd("DEL").arg(&key).query::<()>(conn)).await;
    }
    let fields = shared_entry::encode_backoff(backoff, &self.hostname);
    let ttl_ms = CraftAppsReleaseBackoffRedisKey::get_redis_ttl().num_milliseconds();
    self
        .run(move |conn| {
          redis::pipe()
              .hset_multiple(&key, &fields).ignore()
              .pexpire(&key, ttl_ms).ignore()
              .query::<()>(conn)
        })
        .await
  }

  async fn guarded_write(&self, write: GuardedWrite) -> anyhow::Result<bool> {
    let ttl_ms = CraftAppReleaseRedisKey::get_redis_ttl().num_milliseconds();
    let written: i64 = self
        .run(move |conn| {
          let mut invocation = GUARDED_HSET.key(&write.key);
          invocation
              .arg(write.time_field)
              .arg(write.time.timestamp_millis())
              .arg(write.min_gap.as_millis() as u64)
              .arg(ttl_ms)
              .arg(write.required_etag.as_deref().unwrap_or_default());
          for (field, value) in &write.fields {
            invocation.arg(*field).arg(value);
          }
          invocation.invoke(conn)
        })
        .await?;
    Ok(written == 1)
  }

  /// Runs `call` on a pooled connection, off the async runtime, within [`CALL_TIMEOUT`].
  async fn run<T, F>(&self, call: F) -> anyhow::Result<T>
  where
    T: Send + 'static,
    F: FnOnce(&mut Connection) -> RedisResult<T> + Send + 'static,
  {
    let pool = self.pool.clone();
    let task = tokio::task::spawn_blocking(move || -> anyhow::Result<T> {
      let mut conn = pool.get_timeout(CONNECTION_TIMEOUT)?;
      Ok(call(&mut conn)?)
    });
    match tokio::time::timeout(CALL_TIMEOUT, task).await {
      Ok(Ok(result)) => result,
      Ok(Err(join_err)) => Err(anyhow!("redis task failed: {join_err}")),
      Err(_) => Err(anyhow!("redis call timed out after {CALL_TIMEOUT:?}")),
    }
  }
}

#[cfg(test)]
mod tests {
  use chrono::TimeZone;

  use super::*;

  /// Against a real Redis (`CRAFT_APPS_TEST_REDIS_URL`, default `redis://127.0.0.1:6379/`). Uses
  /// app keys of its own, deletes them afterwards, and leaves the fleet backoff key alone unless
  /// it was absent.
  /// `cargo test -p storyteller-web --bin storyteller-web shared_store -- --ignored`
  #[tokio::test]
  #[ignore = "needs a Redis server"]
  async fn guarded_writes_against_a_real_redis() {
    let url = std::env::var("CRAFT_APPS_TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    let pool = Pool::builder().max_size(2).build(Client::open(url).unwrap()).unwrap();
    let pod_a = SharedStore::new(pool.clone(), "pod-a".into());
    let pod_b = SharedStore::new(pool.clone(), "pod-b".into());
    let app = format!("test-craft-{}", rand::random::<u32>());
    let other = format!("{app}-other");
    let keys = vec![app.clone(), other.clone()];
    let t = |m: u32| Utc.with_ymd_and_hms(2026, 10, 10, 12, m, 0).unwrap();
    let read_app = |store: SharedStore, keys: Vec<String>| async move { store.read(&keys).await.unwrap().0.remove(0) };

    // Nothing yet.
    let (entries, _) = pod_a.read(&keys).await.unwrap();
    assert!(entries.iter().all(Option::is_none));

    // Two instances tick together: only one gets to ask GitHub; the claim lapses after the window.
    assert!(pod_a.claim(&app, t(0)).await.unwrap());
    assert!(!pod_b.claim(&app, t(0) + chrono::Duration::seconds(5)).await.unwrap());
    assert!(pod_b.claim(&app, t(2)).await.unwrap());

    // A newer fetch is never replaced by an older one.
    assert!(pod_a.write_fetched(&app, None, Some("\"e2\""), t(2)).await.unwrap());
    assert!(!pod_b.write_fetched(&app, None, Some("\"e1\""), t(1)).await.unwrap());
    let fetched = read_app(pod_a.clone(), keys.clone()).await.unwrap().fetched.unwrap();
    assert_eq!((fetched.etag.as_deref(), fetched.fetched_at), (Some("\"e2\""), t(2)));

    // A 304 moves the time only, and only for the release it revalidated.
    assert!(!pod_b.write_unchanged(&app, "\"e1\"", t(3)).await.unwrap(), "stale ETag");
    assert!(!pod_b.write_unchanged(&app, "\"e2\"", t(1)).await.unwrap(), "older than the shared fetch");
    assert!(pod_b.write_unchanged(&app, "\"e2\"", t(3)).await.unwrap());
    let fetched = read_app(pod_a.clone(), keys.clone()).await.unwrap().fetched.unwrap();
    assert_eq!((fetched.etag.as_deref(), fetched.fetched_at), (Some("\"e2\""), t(3)));

    // Failures: newest wins.
    let failure = SharedFailure { failed_at: t(5), consecutive_failures: 2 };
    assert!(pod_a.write_failure(&app, failure).await.unwrap());
    assert!(!pod_b.write_failure(&app, SharedFailure { failed_at: t(4), consecutive_failures: 1 }).await.unwrap());
    assert_eq!(read_app(pod_a.clone(), keys.clone()).await.unwrap().failure, Some(failure));

    // Garbage in an entry is overwritten, not trusted.
    let key = CraftAppReleaseRedisKey::new_for_app(&other).to_string();
    let garbage_key = key.clone();
    pod_a.run(move |conn| redis::cmd("HSET").arg(&garbage_key).arg("v").arg("1").arg("fetched_ms").arg("soon").arg("release").arg("{").query::<()>(conn)).await.unwrap();
    assert!(pod_a.read(&keys).await.unwrap().0[1].as_ref().unwrap().fetched.is_none());
    assert!(pod_a.write_fetched(&other, None, None, t(1)).await.unwrap());
    assert!(pod_a.read(&keys).await.unwrap().0[1].as_ref().unwrap().fetched.is_some());

    // Entries expire if nobody refreshes them.
    let ttl_key = key.clone();
    let ttl: i64 = pod_a.run(move |conn| redis::cmd("PTTL").arg(&ttl_key).query(conn)).await.unwrap();
    assert!(ttl > 6 * 24 * 3600 * 1000, "{}", ttl);

    // The fleet backoff round-trips (only touched when absent, to leave a real one alone).
    let (_, existing) = pod_a.read(&keys).await.unwrap();
    if existing == Backoff::default() {
      let backoff = Backoff { until: Some(t(30)), trouble: 2 };
      pod_a.write_backoff(backoff).await.unwrap();
      assert_eq!(pod_b.read(&keys).await.unwrap().1, backoff);
      pod_b.write_backoff(Backoff::default()).await.unwrap();
      assert_eq!(pod_a.read(&keys).await.unwrap().1, Backoff::default());
    }

    let cleanup: Vec<String> = keys.iter().map(|k| CraftAppReleaseRedisKey::new_for_app(k).to_string()).collect();
    pod_a.run(move |conn| redis::cmd("DEL").arg(&cleanup).query::<()>(conn)).await.unwrap();
  }

  /// An unreachable Redis fails fast, so the loop can fall back to polling on its own.
  #[tokio::test]
  async fn an_unreachable_redis_fails_fast() {
    let pool = Pool::builder().max_size(1).build_unchecked(Client::open("redis://127.0.0.1:1/").unwrap());
    let store = SharedStore::new(pool, "pod".into());
    let started = std::time::Instant::now();
    assert!(store.read(&["photocraft".to_string()]).await.is_err());
    assert!(started.elapsed() < CALL_TIMEOUT + Duration::from_secs(1));
  }
}
