//! Reads and writes the fleet's shared Craft App release state in Redis (keys and fields are
//! documented in `shared_entry`).
//!
//! Writes are last-write-wins but guarded by time: a Lua script compares a timestamp field
//! before writing, so a slow instance never replaces a newer fetch with an older one. A `304`
//! moves only `fetched_ms`, and only if the stored ETag is still the one that was revalidated.
//!
//! Some Redis setups refuse scripts (scripting disabled, an ACL without `EVAL`, a proxy). The
//! first time Redis refuses the script, this store switches to applying the same rule itself:
//! it `WATCH`es the hash, reads and decodes the guarding fields, decides, and writes in a
//! `MULTI`/`EXEC` that Redis drops if another instance wrote in between (then it decides again).
//!
//! The pool is r2d2 (blocking), so every call runs on tokio's blocking pool with a timeout:
//! Redis trouble costs the polling loop a few seconds at most, never a stuck thread of the
//! async runtime. The poller gets its own small pool ([`build_poller_redis_pool`]) whose
//! connections time out reads and writes, so a hung Redis can neither strand blocking threads
//! nor take connections from the request handlers' pool.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use log::warn;
use r2d2::{ManageConnection, Pool};
use redis::{Client, Connection, ConnectionLike, ErrorKind, RedisError, RedisResult, Script, ServerErrorKind, Value};

use redis_schema::keys::craft_apps::craft_app_release_history_redis_key::CraftAppReleaseHistoryRedisKey;
use redis_schema::keys::craft_apps::craft_app_release_redis_key::CraftAppReleaseRedisKey;
use redis_schema::keys::craft_apps::craft_apps_release_backoff_redis_key::CraftAppsReleaseBackoffRedisKey;

use crate::state::craft_apps_release_cache::{CraftAppHistory, CraftAppReleases};
use crate::threads::poll_craft_app_releases_thread::scheduling::Backoff;
use crate::threads::poll_craft_app_releases_thread::shared_entry;
use crate::threads::poll_craft_app_releases_thread::shared_entry::{SharedAppEntry, SharedFailure};

/// Connections in the poller's pool: one per instance is all the loop uses at a time.
const POOL_MAX_SIZE: u32 = 2;
/// Longest wait for a pooled connection.
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(3);
/// Longest wait for one read or write on a connection; after it, the connection gives up
/// instead of holding its blocking thread until the operating system notices a dead peer.
const IO_TIMEOUT: Duration = Duration::from_secs(5);
/// Longest wait for a whole call, connection included.
const CALL_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a claim on an app keeps other instances from requesting it too: longer than one fetch
/// can take (up to 10 pages, each at most 10 s to connect and 30 s for the answer), plus a margin.
pub const CLAIM_WINDOW: Duration = Duration::from_secs(7 * 60);
/// Without scripts: how many times a guarded write decides again after another instance wrote
/// between its read and its write, before giving up (as if it lost the race).
const WATCH_ATTEMPTS: usize = 3;

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

/// [`GUARDED_HSET_LUA`] with a companion: by the same rule and in the same step, also sets the
/// string `KEYS[2]` to `ARGV[6]` with the same TTL (the hash's fields then start at `ARGV[7]`).
/// The history of a fetch is written only together with the fetch it belongs to.
const GUARDED_HSET_WITH_COMPANION_LUA: &str = r#"
local current = tonumber(redis.call('HGET', KEYS[1], ARGV[1]))
if current and current + tonumber(ARGV[3]) > tonumber(ARGV[2]) then
  return 0
end
if ARGV[5] ~= '' and redis.call('HGET', KEYS[1], 'etag') ~= ARGV[5] then
  return 0
end
redis.call('HSET', KEYS[1], unpack(ARGV, 7))
redis.call('PEXPIRE', KEYS[1], ARGV[4])
redis.call('SET', KEYS[2], ARGV[6], 'PX', ARGV[4])
return 1
"#;

static GUARDED_HSET_WITH_COMPANION: LazyLock<Script> = LazyLock::new(|| Script::new(GUARDED_HSET_WITH_COMPANION_LUA));

/// Handle on the shared state. Cheap to clone.
#[derive(Clone)]
pub struct SharedStore {
  pool: PollerRedisPool,
  /// Written as `by`, for debugging.
  hostname: String,
  /// Set once Redis refuses the guarded-write script; from then on guarded writes use `WATCH`.
  scripts_refused: Arc<AtomicBool>,
}

/// The poller's Redis pool; see [`build_poller_redis_pool`].
pub type PollerRedisPool = Pool<PollerRedisConnections>;

/// Makes the poller's Redis connections, every one of them with I/O timeouts.
pub struct PollerRedisConnections {
  client: Client,
}

/// A guarded write: which time field guards it, and what to write.
struct GuardedWrite {
  key: String,
  time_field: &'static str,
  time: DateTime<Utc>,
  min_gap: Duration,
  required_etag: Option<String>,
  fields: Vec<(&'static str, String)>,
  /// A string key set to a value only when, and in the same step as, the hash is written.
  companion: Option<(String, String)>,
}

impl SharedStore {
  pub fn new(pool: PollerRedisPool, hostname: String) -> Self {
    Self { pool, hostname, scripts_refused: Arc::new(AtomicBool::new(false)) }
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
      companion: None,
    }).await
  }

  /// Shares a successful fetch and its history, in one step, unless a newer fetch is already
  /// shared. Returns whether written.
  pub async fn write_fetched(&self, app_key: &str, releases: &CraftAppReleases, history: &CraftAppHistory, etag: Option<&str>, fetched_at: DateTime<Utc>) -> anyhow::Result<bool> {
    self.guarded_write(GuardedWrite {
      key: CraftAppReleaseRedisKey::new_for_app(app_key).to_string(),
      time_field: shared_entry::FIELD_FETCHED_MS,
      time: fetched_at,
      min_gap: Duration::ZERO,
      required_etag: None,
      fields: shared_entry::encode_fetch(releases, etag, fetched_at, history.fetched_at, &self.hostname)?,
      companion: Some((CraftAppReleaseHistoryRedisKey::new_for_app(app_key).to_string(), shared_entry::encode_history(history)?)),
    }).await
  }

  /// Extends the shared history's TTL to the entry's.
  async fn touch_history(&self, app_key: &str) -> anyhow::Result<()> {
    let key = CraftAppReleaseHistoryRedisKey::new_for_app(app_key).to_string();
    let ttl_ms = CraftAppReleaseHistoryRedisKey::get_redis_ttl().num_milliseconds();
    self.run(move |conn| redis::cmd("PEXPIRE").arg(&key).arg(ttl_ms).query::<()>(conn)).await
  }

  /// The shared history of `app_key`, `None` when absent or unreadable.
  pub async fn read_history(&self, app_key: &str) -> anyhow::Result<Option<CraftAppHistory>> {
    let key = CraftAppReleaseHistoryRedisKey::new_for_app(app_key).to_string();
    let value: Option<String> = self.run(move |conn| redis::cmd("GET").arg(&key).query(conn)).await?;
    Ok(value.as_deref().and_then(shared_entry::decode_history))
  }

  /// Shares a `304` for the release with ETag `etag`: moves only the fetch time, and only if
  /// that release is still the shared one and nothing newer was shared. Returns whether written.
  pub async fn write_unchanged(&self, app_key: &str, etag: &str, fetched_at: DateTime<Utc>) -> anyhow::Result<bool> {
    if etag.is_empty() {
      return Ok(false);
    }
    let written = self.guarded_write(GuardedWrite {
      key: CraftAppReleaseRedisKey::new_for_app(app_key).to_string(),
      time_field: shared_entry::FIELD_FETCHED_MS,
      time: fetched_at,
      min_gap: Duration::ZERO,
      required_etag: Some(etag.to_string()),
      fields: shared_entry::encode_unchanged(fetched_at, &self.hostname),
      companion: None,
    }).await?;
    if written {
      // The history lives as long as the hash it belongs with.
      let key = CraftAppReleaseHistoryRedisKey::new_for_app(app_key).to_string();
      let ttl_ms = CraftAppReleaseHistoryRedisKey::get_redis_ttl().num_milliseconds();
      self.run(move |conn| redis::cmd("PEXPIRE").arg(&key).arg(ttl_ms).query::<()>(conn)).await?;
    }
    Ok(written)
  }

  /// Shares a failed request, unless a newer failure is already shared. Returns whether written.
  pub async fn write_failure(&self, app_key: &str, failure: SharedFailure) -> anyhow::Result<bool> {
    let written = self.guarded_write(GuardedWrite {
      key: CraftAppReleaseRedisKey::new_for_app(app_key).to_string(),
      time_field: shared_entry::FIELD_FAILED_MS,
      time: failure.failed_at,
      min_gap: Duration::ZERO,
      required_etag: None,
      fields: shared_entry::encode_failure(failure, &self.hostname),
      companion: None,
    }).await?;
    // Failures keep the entry alive; the history lives as long, so it isn't the one to expire. Not
    // extending it loses nothing yet, so it doesn't make the failure unshared.
    if written {
      if let Err(err) = self.touch_history(app_key).await {
        warn!("Craft app releases: couldn't extend the shared history of {}: {}", app_key, err);
      }
    }
    Ok(written)
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

  /// Applies `write` with the script, or (once Redis has refused the script) with `WATCH`.
  async fn guarded_write(&self, write: GuardedWrite) -> anyhow::Result<bool> {
    let ttl_ms = CraftAppReleaseRedisKey::get_redis_ttl().num_milliseconds();
    let scripts_refused = self.scripts_refused.clone();
    self
        .run(move |conn| {
          if !scripts_refused.load(Ordering::Relaxed) {
            match guarded_write_with_script(conn, &write, ttl_ms) {
              Err(err) if is_script_refusal(&err) => {
                if !scripts_refused.swap(true, Ordering::Relaxed) {
                  warn!("Craft app releases: Redis refused the guarded-write script ({err}); writing with WATCH/MULTI/EXEC instead.");
                }
              }
              result => return result,
            }
          }
          guarded_write_with_watch(conn, &write, ttl_ms)
        })
        .await
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

/// The poller's own Redis pool, apart from the one the request handlers use. Built without
/// connecting, so an unreachable Redis at startup doesn't stop the server: the poller then
/// works alone until Redis answers.
pub fn build_poller_redis_pool(redis_url: &str) -> RedisResult<PollerRedisPool> {
  let manager = PollerRedisConnections { client: Client::open(redis_url)? };
  Ok(Pool::builder()
      .max_size(POOL_MAX_SIZE)
      .min_idle(Some(0))
      .connection_timeout(CONNECTION_TIMEOUT)
      .build_unchecked(manager))
}

impl ManageConnection for PollerRedisConnections {
  type Connection = Connection;
  type Error = RedisError;

  /// Connects (handshake included) within [`CONNECTION_TIMEOUT`], then makes every later read
  /// and write give up after [`IO_TIMEOUT`], before the connection is ever used.
  fn connect(&self) -> RedisResult<Connection> {
    let conn = self.client.get_connection_with_timeout(CONNECTION_TIMEOUT)?;
    conn.set_read_timeout(Some(IO_TIMEOUT))?;
    conn.set_write_timeout(Some(IO_TIMEOUT))?;
    Ok(conn)
  }

  /// The pool's check before lending a connection; bounded by [`IO_TIMEOUT`] like any command.
  fn is_valid(&self, conn: &mut Connection) -> RedisResult<()> {
    redis::cmd("PING").query::<()>(conn)
  }

  fn has_broken(&self, conn: &mut Connection) -> bool {
    !conn.is_open()
  }
}

/// [`GUARDED_HSET_LUA`] decides and writes on the server, in one step.
fn guarded_write_with_script(conn: &mut Connection, write: &GuardedWrite, ttl_ms: i64) -> RedisResult<bool> {
  let mut invocation = match &write.companion {
    Some((key, _)) => {
      let mut invocation = GUARDED_HSET_WITH_COMPANION.key(&write.key);
      invocation.key(key);
      invocation
    }
    None => GUARDED_HSET.key(&write.key),
  };
  invocation
      .arg(write.time_field)
      .arg(write.time.timestamp_millis())
      .arg(write.min_gap.as_millis() as u64)
      .arg(ttl_ms)
      .arg(write.required_etag.as_deref().unwrap_or_default());
  if let Some((_, value)) = &write.companion {
    invocation.arg(value);
  }
  for (field, value) in &write.fields {
    invocation.arg(*field).arg(value);
  }
  let written: i64 = invocation.invoke(conn)?;
  Ok(written == 1)
}

/// The same rule without scripts: `WATCH` the hash, read and decode its guarding fields, decide
/// with [`write_wins`], then write in a transaction. If another instance wrote the hash in
/// between, Redis drops the transaction and this decides again with what it wrote.
fn guarded_write_with_watch(conn: &mut Connection, write: &GuardedWrite, ttl_ms: i64) -> RedisResult<bool> {
  for _ in 0..WATCH_ATTEMPTS {
    redis::cmd("WATCH").arg(&write.key).query::<()>(conn)?;
    let stored: RedisResult<(Option<String>, Option<String>)> = redis::cmd("HMGET")
        .arg(&write.key)
        .arg(write.time_field)
        .arg(shared_entry::FIELD_ETAG)
        .query(conn);
    let (stored_time, stored_etag) = match stored {
      Ok(stored) => stored,
      Err(err) => {
        // The connection goes back to the shared pool; don't leave it watching.
        let _ = redis::cmd("UNWATCH").query::<()>(conn);
        return Err(err);
      }
    };
    if !write_wins(write, stored_time.as_deref(), stored_etag.as_deref()) {
      redis::cmd("UNWATCH").query::<()>(conn)?;
      return Ok(false);
    }
    // `EXEC` ends the watch either way; `None` means another write got in first.
    let mut pipe = redis::pipe();
    pipe.atomic().hset_multiple(&write.key, &write.fields).pexpire(&write.key, ttl_ms);
    if let Some((key, value)) = &write.companion {
      pipe.cmd("SET").arg(key).arg(value).arg("PX").arg(ttl_ms);
    }
    let committed: Option<Value> = pipe.query(conn)?;
    if committed.is_some() {
      return Ok(true);
    }
  }
  Ok(false)
}

/// Whether `write` may replace what the hash holds, by the script's rule: not if the stored time
/// plus the minimum gap is later than the new time, nor if an ETag is required and the stored
/// one differs. An unreadable stored time doesn't block the write, so garbage gets overwritten.
fn write_wins(write: &GuardedWrite, stored_time: Option<&str>, stored_etag: Option<&str>) -> bool {
  let new_ms = write.time.timestamp_millis();
  let gap_ms = write.min_gap.as_millis().min(i64::MAX as u128) as i64;
  if let Some(current_ms) = stored_time.and_then(|t| t.parse::<i64>().ok()) {
    if current_ms.saturating_add(gap_ms) > new_ms {
      return false;
    }
  }
  match write.required_etag.as_deref().filter(|etag| !etag.is_empty()) {
    Some(required) => stored_etag == Some(required),
    None => true,
  }
}

/// Whether Redis answered that it won't run the script (as opposed to being unreachable, busy,
/// out of memory or read-only, which the `WATCH` path wouldn't get past either): an ACL
/// refusal, an unknown `EVALSHA`/`EVAL`/`SCRIPT` command, or scripting switched off.
fn is_script_refusal(err: &RedisError) -> bool {
  if err.is_io_error() || err.is_timeout() || err.is_connection_dropped() || err.is_connection_refusal() {
    return false;
  }
  match err.kind() {
    ErrorKind::Server(ServerErrorKind::NoPerm) => true,
    ErrorKind::Server(ServerErrorKind::ResponseError) | ErrorKind::Extension => {
      if err.code() == Some("BUSY") {
        return false;
      }
      let detail = err.detail().unwrap_or_default().to_ascii_lowercase();
      detail.contains("unknown command") || detail.contains("eval") || detail.contains("script")
    }
    _ => false,
  }
}


#[cfg(test)]
mod tests {
  use std::io::{Read, Write};
  use std::net::{TcpListener, TcpStream};

  use chrono::TimeZone;
  use url::Url;

  use super::*;

  const DEFAULT_TEST_REDIS_URL: &str = "redis://127.0.0.1:6379/";

  /// Against a real Redis (`CRAFT_APPS_TEST_REDIS_URL`, default `redis://127.0.0.1:6379/`). Each
  /// test uses app keys of its own, deletes them afterwards, and leaves the fleet backoff key
  /// alone unless it was absent.
  /// `cargo test -p storyteller-web --bin storyteller-web shared_store -- --ignored`
  mod live_redis_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "needs a Redis server"]
    async fn guarded_writes_with_the_script() {
      let pool = test_pool(&test_redis_url());
      let (pod_a, pod_b) = (SharedStore::new(pool.clone(), "pod-a".into()), SharedStore::new(pool, "pod-b".into()));
      exercise_guarded_writes(&pod_a, &pod_b).await;
      assert!(!pod_a.scripts_refused.load(Ordering::Relaxed));
    }

    #[tokio::test]
    #[ignore = "needs a Redis server"]
    async fn guarded_writes_with_watch() {
      let pool = test_pool(&test_redis_url());
      let (pod_a, pod_b) = (SharedStore::new(pool.clone(), "pod-a".into()), SharedStore::new(pool, "pod-b".into()));
      pod_a.scripts_refused.store(true, Ordering::Relaxed);
      pod_b.scripts_refused.store(true, Ordering::Relaxed);
      exercise_guarded_writes(&pod_a, &pod_b).await;
    }

    /// A Redis user without scripting permissions: the first guarded write is refused, and the
    /// store switches to `WATCH` and still applies every rule.
    #[tokio::test]
    #[ignore = "needs a Redis server that supports ACLs"]
    async fn a_refused_script_falls_back_to_watch() {
      let admin_url = test_redis_url();
      let admin = test_pool(&admin_url);
      let user = format!("craft-test-{}", rand::random::<u32>());
      let password = format!("pw-{}", rand::random::<u64>());
      let mut conn = admin.get().unwrap();
      redis::cmd("ACL").arg("SETUSER").arg(&user).arg("reset").arg("on").arg(format!(">{password}")).arg("~*").arg("&*").arg("+@all").arg("-@scripting")
          .query::<()>(&mut *conn).unwrap();

      let mut url = Url::parse(&admin_url).unwrap();
      url.set_username(&user).unwrap();
      url.set_password(Some(&password)).unwrap();
      let pool = test_pool(url.as_str());
      let (pod_a, pod_b) = (SharedStore::new(pool.clone(), "pod-a".into()), SharedStore::new(pool, "pod-b".into()));
      exercise_guarded_writes(&pod_a, &pod_b).await;

      redis::cmd("ACL").arg("DELUSER").arg(&user).query::<()>(&mut *conn).unwrap();
      assert!(pod_a.scripts_refused.load(Ordering::Relaxed), "the script should have been refused");
      assert!(pod_b.scripts_refused.load(Ordering::Relaxed), "the script should have been refused");
    }
  }

  /// An unreachable Redis fails fast, so the loop can fall back to polling on its own.
  #[tokio::test]
  async fn an_unreachable_redis_fails_fast() {
    let pool = build_poller_redis_pool("redis://127.0.0.1:1/").unwrap();
    let store = SharedStore::new(pool, "pod".into());
    let started = std::time::Instant::now();
    assert!(store.read(&["photocraft".to_string()]).await.is_err());
    assert!(started.elapsed() < CALL_TIMEOUT + Duration::from_secs(1));
  }

  /// A Redis that completes the connection handshake and then stops answering (a wedged server,
  /// a half-open connection): the call fails by itself, before [`CALL_TIMEOUT`] gives up on it,
  /// so its blocking thread and connection are released rather than stranded until the
  /// operating system notices. Without I/O timeouts the pool's `PING` would hang here forever.
  #[tokio::test]
  async fn a_redis_that_stops_answering_releases_the_blocking_thread() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("redis://{}/", listener.local_addr().unwrap());
    std::thread::spawn(move || {
      for stream in listener.incoming().flatten() {
        std::thread::spawn(move || answer_handshake_then_go_silent(stream));
      }
    });
    let store = SharedStore::new(build_poller_redis_pool(&url).unwrap(), "pod".into());
    let started = std::time::Instant::now();
    let err = store.read(&["photocraft".to_string()]).await.unwrap_err();
    assert!(started.elapsed() < CALL_TIMEOUT, "{:?} after {:?}", err, started.elapsed());
  }

  /// [`write_wins`] must decide exactly as [`GUARDED_HSET_LUA`] does.
  mod write_rule_tests {
    use super::*;

    #[test]
    fn a_write_never_goes_back_in_time() {
      let write = guarded_write_at(t(2), Duration::ZERO, None);
      assert!(write_wins(&write, None, None), "nothing stored");
      assert!(write_wins(&write, Some(&ms(t(1))), None), "newer than stored");
      assert!(write_wins(&write, Some(&ms(t(2))), None), "same time, as the script allows");
      assert!(!write_wins(&write, Some(&ms(t(3))), None), "older than stored");
    }

    #[test]
    fn a_claim_waits_for_the_window() {
      let window = chrono::Duration::from_std(CLAIM_WINDOW).unwrap();
      let claim = guarded_write_at(t(0) + window, CLAIM_WINDOW, None);
      assert!(!write_wins(&claim, Some(&ms(t(0) + chrono::Duration::seconds(30))), None));
      assert!(write_wins(&claim, Some(&ms(t(0))), None), "exactly one window later");
    }

    #[test]
    fn a_required_etag_must_match() {
      let write = guarded_write_at(t(2), Duration::ZERO, Some("\"e2\""));
      assert!(write_wins(&write, Some(&ms(t(1))), Some("\"e2\"")));
      assert!(!write_wins(&write, Some(&ms(t(1))), Some("\"e1\"")), "another release is shared");
      assert!(!write_wins(&write, Some(&ms(t(1))), None), "no release is shared");
      assert!(write_wins(&guarded_write_at(t(2), Duration::ZERO, Some("")), None, None), "empty means none required");
    }

    #[test]
    fn an_unreadable_stored_time_is_overwritten() {
      let write = guarded_write_at(t(2), Duration::ZERO, None);
      assert!(write_wins(&write, Some("soon"), None));
      assert!(write_wins(&write, Some(""), None));
    }
  }

  mod script_refusal_tests {
    use redis::make_extension_error;

    use super::*;

    #[test]
    fn redis_refusing_scripts_is_a_refusal() {
      let refusals = [
        server_error(ServerErrorKind::NoPerm, "this user has no permissions to run the 'evalsha' command"),
        server_error(ServerErrorKind::ResponseError, "unknown command 'EVALSHA', with args beginning with: "),
        server_error(ServerErrorKind::ResponseError, "This Redis instance has scripting disabled"),
        make_extension_error("ERR".to_string(), Some("unknown command 'SCRIPT'".to_string())),
      ];
      for err in refusals {
        assert!(is_script_refusal(&err), "{:?}", err);
      }
    }

    #[test]
    fn redis_being_unavailable_is_not_a_refusal() {
      let unavailable = [
        RedisError::from(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused")),
        RedisError::from(std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out")),
        make_extension_error("OOM".to_string(), Some("command not allowed when used memory > 'maxmemory'.".to_string())),
        make_extension_error("BUSY".to_string(), Some("Redis is busy running a script.".to_string())),
        server_error(ServerErrorKind::ReadOnly, "You can't write against a read only replica."),
        server_error(ServerErrorKind::ResponseError, "wrong number of arguments for 'hset' command"),
      ];
      for err in unavailable {
        assert!(!is_script_refusal(&err), "{:?}", err);
      }
    }

    fn server_error(kind: ServerErrorKind, detail: &str) -> RedisError {
      RedisError::from((ErrorKind::Server(kind), "server error", detail.to_string()))
    }
  }

  /// Every guarded-write rule, as two instances sharing one Redis see it.
  async fn exercise_guarded_writes(pod_a: &SharedStore, pod_b: &SharedStore) {
    let app = format!("test-craft-{}", rand::random::<u32>());
    let other = format!("{app}-other");
    let keys = vec![app.clone(), other.clone()];
    let read_app = |store: SharedStore, keys: Vec<String>| async move { store.read(&keys).await.unwrap().0.remove(0) };

    // Nothing yet.
    let (entries, _) = pod_a.read(&keys).await.unwrap();
    assert!(entries.iter().all(Option::is_none));

    // Two instances tick together: only one gets to ask GitHub; the claim lapses after the window.
    assert!(pod_a.claim(&app, t(0)).await.unwrap());
    assert!(!pod_b.claim(&app, t(0) + chrono::Duration::seconds(5)).await.unwrap());
    assert!(pod_b.claim(&app, t(0) + chrono::Duration::from_std(CLAIM_WINDOW).unwrap()).await.unwrap());

    // A newer fetch is never replaced by an older one.
    let history = |etag: &str| CraftAppHistory { etag: Some(etag.into()), complete: true, releases: Vec::new(), fetched_at: None };
    assert!(pod_a.write_fetched(&app, &CraftAppReleases::default(), &history("\"e2\""), Some("\"e2\""), t(2)).await.unwrap());
    assert!(pod_a.claim(&app, t(2)).await.unwrap(), "a shared result releases the claim");
    assert!(!pod_b.write_fetched(&app, &CraftAppReleases::default(), &history("\"e1\""), Some("\"e1\""), t(1)).await.unwrap());
    assert_eq!(pod_b.read_history(&app).await.unwrap().unwrap().etag.as_deref(), Some("\"e2\""), "the losing write's history isn't shared");
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
    pod_a.run(move |conn| redis::cmd("HSET").arg(&garbage_key).arg("v").arg("1").arg("fetched_ms").arg("soon").arg("releases").arg("{").query::<()>(conn)).await.unwrap();
    assert!(pod_a.read(&keys).await.unwrap().0[1].as_ref().unwrap().fetched.is_none());
    assert!(pod_a.write_fetched(&other, &CraftAppReleases::default(), &CraftAppHistory::default(), None, t(1)).await.unwrap());
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

    let mut cleanup: Vec<String> = keys.iter().map(|k| CraftAppReleaseRedisKey::new_for_app(k).to_string()).collect();
    cleanup.extend(keys.iter().map(|k| CraftAppReleaseHistoryRedisKey::new_for_app(k).to_string()));
    pod_a.run(move |conn| redis::cmd("DEL").arg(&cleanup).query::<()>(conn)).await.unwrap();
  }

  fn guarded_write_at(time: DateTime<Utc>, min_gap: Duration, required_etag: Option<&str>) -> GuardedWrite {
    GuardedWrite {
      key: "craftApps:releases:app:test".to_string(),
      time_field: shared_entry::FIELD_FETCHED_MS,
      time,
      min_gap,
      required_etag: required_etag.map(str::to_string),
      fields: Vec::new(),
      companion: None,
    }
  }

  /// Answers `+OK` to connection setup (`CLIENT SETINFO`, `SELECT`, `HELLO`), then keeps the
  /// connection open without another word once a real command (`PING`, `HGETALL`, ...) arrives.
  fn answer_handshake_then_go_silent(mut stream: TcpStream) {
    let mut buf = [0u8; 4096];
    loop {
      let n = match stream.read(&mut buf) {
        Ok(0) | Err(_) => return,
        Ok(n) => n,
      };
      let chunk = String::from_utf8_lossy(&buf[..n]).to_ascii_uppercase();
      if !(chunk.contains("CLIENT") || chunk.contains("SELECT") || chunk.contains("HELLO")) {
        std::thread::sleep(Duration::from_secs(60));
        return;
      }
      let commands = chunk.lines().filter(|line| line.starts_with('*')).count();
      if stream.write_all("+OK\r\n".repeat(commands).as_bytes()).is_err() {
        return;
      }
    }
  }

  fn test_redis_url() -> String {
    std::env::var("CRAFT_APPS_TEST_REDIS_URL").unwrap_or_else(|_| DEFAULT_TEST_REDIS_URL.to_string())
  }

  fn test_pool(url: &str) -> PollerRedisPool {
    build_poller_redis_pool(url).unwrap()
  }

  fn t(minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 10, 12, minute, 0).unwrap()
  }

  fn ms(time: DateTime<Utc>) -> String {
    time.timestamp_millis().to_string()
  }
}
