//! Keeps [`CraftAppsReleaseCache`] holding the latest release of every Craft App, which
//! `GET /v1/craft_apps/release_info` serves to ArtCraft Launcher (so thousands of launchers don't
//! each poll GitHub).
//!
//! Every instance runs this loop, waking every [`TICK`] (30 s, with jitter). With coordination
//! (the default) the fleet shares its work through Redis (`shared_entry` documents the keys):
//!
//! 1. Read every app's shared entry and the fleet's backoff; take anything newer into memory.
//!    The endpoint always serves memory, so it keeps answering whatever Redis does.
//! 2. If the fleet is backing off (GitHub rate limited us, or a whole round failed), stop.
//! 3. For each app that is due (its shared fetch is `interval - CLOCK_SKEW_MARGIN` old, and any
//!    failure retry has waited), read its entry again (another instance may just have fetched
//!    it), claim it so instances ticking at the same moment don't all ask, then ask GitHub with
//!    the shared ETag and share the answer. Shared writes never go back in time.
//! 4. After a round with requests, update the fleet's backoff (a good round clears it).
//!
//! Each app is all-or-nothing: either its latest release is stored with a fresh
//! `app_last_fetched`, or nothing about it changes.
//!
//! If Redis is unreachable (or coordination is off) the instance does the same from its own
//! memory, as every instance did before coordination, and returns to coordinating when Redis
//! answers again. Nothing here panics; trouble is logged and retried.

use std::time::{Duration, Instant};

use chrono::Utc;
use log::{error, info, warn};
use r2d2::Pool;
use redis::Client;

use crate::state::craft_apps_release_cache::CraftAppsReleaseCache;
use crate::threads::poll_craft_app_releases_thread::config::CraftAppsPollConfig;
use crate::threads::poll_craft_app_releases_thread::fallback::{PollMode, RedisTroubleLog};
use crate::threads::poll_craft_app_releases_thread::github;
use crate::threads::poll_craft_app_releases_thread::github::FetchOutcome;
use crate::threads::poll_craft_app_releases_thread::scheduling::{backoff_after, is_due, jittered, Backoff, RoundResult, TICK};
use crate::threads::poll_craft_app_releases_thread::shared_entry::{SharedAppEntry, SharedFailure};
use crate::threads::poll_craft_app_releases_thread::shared_store::SharedStore;

/// Spreads a fleet's first tick over this window, so a deploy doesn't hit GitHub all at once.
const STARTUP_JITTER: Duration = Duration::from_secs(15);
/// Pause between the requests of one round.
const BETWEEN_REQUESTS: Duration = Duration::from_millis(1_000);

pub async fn poll_craft_app_releases(cache: CraftAppsReleaseCache, config: CraftAppsPollConfig, redis_pool: Pool<Client>, hostname: String) {
  cache.register_apps(&config.apps);
  if !config.enabled {
    info!("Craft app release polling is disabled (CRAFT_APPS_RELEASE_POLLING_ENABLED=false).");
    return;
  }
  let client = match github::build_client() {
    Ok(client) => client,
    Err(err) => {
      error!("Craft app release polling can't start: no HTTP client: {:?}", err);
      return;
    }
  };
  info!(
    "Polling GitHub for {} craft app releases, each at most every {:?} (GitHub token: {}; coordinated through Redis: {}).",
    config.apps.len(), config.interval, config.github_token.is_some(), config.coordination_enabled,
  );
  let store = config.coordination_enabled.then(|| SharedStore::new(redis_pool, hostname));
  let mut poller = Poller::new(client, cache, config, store);
  tokio::time::sleep(jittered(STARTUP_JITTER, 1.0)).await;
  loop {
    poller.tick().await;
    tokio::time::sleep(jittered(TICK, 0.1)).await;
  }
}

/// The polling loop's state.
struct Poller {
  client: wreq::Client,
  cache: CraftAppsReleaseCache,
  config: CraftAppsPollConfig,
  /// `None` when coordination is off.
  store: Option<SharedStore>,
  /// The backoff this instance knows: the fleet's while Redis answers, its own otherwise.
  backoff: Backoff,
  redis_log: RedisTroubleLog,
}

/// What one tick did.
#[derive(Debug)]
struct TickReport {
  mode: PollMode,
  round: RoundResult,
}

/// Whether to ask GitHub for an app, after looking at its shared entry once more.
enum SharedCheck {
  Ask,
  /// Fresh after all, or another instance is asking right now.
  Skip,
  /// The fleet started backing off.
  StopRound,
}

impl Poller {
  fn new(client: wreq::Client, cache: CraftAppsReleaseCache, config: CraftAppsPollConfig, store: Option<SharedStore>) -> Self {
    Self { client, cache, config, store, backoff: Backoff::default(), redis_log: RedisTroubleLog::default() }
  }

  async fn tick(&mut self) -> TickReport {
    let shared_state_readable = self.refresh_from_shared().await;
    let mode = PollMode::choose(self.store.is_some(), shared_state_readable);
    let mut report = TickReport { mode, round: RoundResult::default() };
    let now = Utc::now();
    if self.backoff.is_active(now) {
      return report;
    }
    let due: Vec<(String, String)> = self
        .config
        .apps
        .iter()
        .filter(|(key, _, _)| self.cache.app(key).is_some_and(|app| is_due(&app, now, self.config.interval)))
        .map(|(key, _, repo)| (key.clone(), repo.clone()))
        .collect();
    if due.is_empty() {
      return report;
    }
    let (round, mode) = self.poll_round(&due, mode).await;
    report.round = round;
    report.mode = mode;
    if report.round.requests() == 0 {
      return report;
    }
    self.update_backoff(&report.round, mode).await;
    info!(
      "Craft app releases ({:?}): {} fetched, {} unchanged, {} failed{}{}.",
      mode, report.round.fetched, report.round.unchanged, report.round.failed,
      if report.round.rate_limited.is_some() { ", rate limited" } else { "" },
      self.backoff.until.map(|until| format!("; backing off until {until}")).unwrap_or_default(),
    );
    report
  }

  /// Takes the fleet's shared state into memory. False when coordination is off or Redis
  /// couldn't be read (this tick then works alone).
  async fn refresh_from_shared(&mut self) -> bool {
    let Some(store) = self.store.clone() else {
      return false;
    };
    let keys: Vec<String> = self.config.apps.iter().map(|(key, _, _)| key.clone()).collect();
    match store.read(&keys).await {
      Ok((entries, backoff)) => {
        self.redis_log.succeeded(Instant::now());
        for (key, entry) in keys.iter().zip(entries) {
          if let Some(entry) = entry {
            merge_shared_entry(&self.cache, key, entry);
          }
        }
        self.backoff = backoff;
        true
      }
      Err(err) => {
        self.redis_log.failed(&err, Instant::now());
        false
      }
    }
  }

  /// Asks GitHub about each due app. Returns what happened and the mode it ended in (a Redis
  /// error mid-round finishes the round alone).
  async fn poll_round(&mut self, due: &[(String, String)], mut mode: PollMode) -> (RoundResult, PollMode) {
    let mut round = RoundResult::default();
    for (key, repo) in due {
      if let Some(store) = self.coordinated_store(mode) {
        match self.check_shared(&store, key).await {
          Ok(SharedCheck::Ask) => {}
          Ok(SharedCheck::Skip) => continue,
          Ok(SharedCheck::StopRound) => break,
          Err(err) => {
            self.redis_log.failed(&err, Instant::now());
            mode = PollMode::Standalone;
          }
        }
      }
      if round.requests() > 0 {
        tokio::time::sleep(BETWEEN_REQUESTS).await;
      }
      let etag = self.cache.etag(key);
      let outcome = github::fetch_latest_release(&self.client, repo, etag.as_deref(), self.config.github_token.as_deref()).await;
      let now = Utc::now();
      match outcome {
        FetchOutcome::Fetched { release, etag } => {
          round.fetched += 1;
          if let Some(store) = self.coordinated_store(mode) {
            let shared = store.write_fetched(key, release.as_ref(), etag.as_deref(), now).await;
            mode = self.after_shared_write(shared, mode);
          }
          self.cache.store(key, release, etag, now);
        }
        FetchOutcome::NotModified => {
          round.unchanged += 1;
          self.cache.confirm_unchanged(key, now);
          if let (Some(store), Some(etag)) = (self.coordinated_store(mode), etag.as_deref()) {
            let shared = store.write_unchanged(key, etag, now).await;
            mode = self.after_shared_write(shared, mode);
          }
        }
        FetchOutcome::RateLimited { retry_at } => {
          warn!("GitHub rate limit reached polling {repo}; retry after {:?}.", retry_at);
          round.rate_limited = Some(retry_at);
          // The rest of the round would be refused too.
          break;
        }
        FetchOutcome::Failed(why) => {
          warn!("Couldn't fetch the latest release of {repo}: {why}");
          round.failed += 1;
          let consecutive_failures = self.cache.record_failure(key, now);
          if let Some(store) = self.coordinated_store(mode) {
            let failure = SharedFailure { failed_at: now, consecutive_failures };
            let shared = store.write_failure(key, failure).await;
            mode = self.after_shared_write(shared, mode);
          }
        }
      }
    }
    (round, mode)
  }

  /// Right before asking GitHub about `key`: takes its latest shared entry, and claims it if it
  /// is still due and the fleet isn't backing off.
  async fn check_shared(&mut self, store: &SharedStore, key: &str) -> anyhow::Result<SharedCheck> {
    let (mut entries, backoff) = store.read(&[key.to_string()]).await?;
    if let Some(entry) = entries.pop().flatten() {
      merge_shared_entry(&self.cache, key, entry);
    }
    self.backoff = backoff;
    let now = Utc::now();
    if backoff.is_active(now) {
      return Ok(SharedCheck::StopRound);
    }
    if !self.cache.app(key).is_some_and(|app| is_due(&app, now, self.config.interval)) {
      return Ok(SharedCheck::Skip);
    }
    Ok(if store.claim(key, now).await? { SharedCheck::Ask } else { SharedCheck::Skip })
  }

  /// Sets the backoff after a round with requests, and shares it if it changed.
  async fn update_backoff(&mut self, round: &RoundResult, mode: PollMode) {
    let previous = self.backoff;
    self.backoff = backoff_after(&self.config, round, previous, Utc::now());
    if self.backoff == previous {
      return;
    }
    if let Some(store) = self.coordinated_store(mode) {
      if let Err(err) = store.write_backoff(self.backoff).await {
        self.redis_log.failed(&err, Instant::now());
      }
    }
  }

  /// Logs a failed shared write; the rest of the round then works alone.
  fn after_shared_write(&mut self, result: anyhow::Result<bool>, mode: PollMode) -> PollMode {
    match result {
      Ok(_) => mode,
      Err(err) => {
        self.redis_log.failed(&err, Instant::now());
        PollMode::Standalone
      }
    }
  }

  /// The shared store, in [`PollMode::Coordinated`].
  fn coordinated_store(&self, mode: PollMode) -> Option<SharedStore> {
    self.store.clone().filter(|_| mode == PollMode::Coordinated)
  }
}

/// Takes a shared entry into memory (only what is newer than what this instance has).
fn merge_shared_entry(cache: &CraftAppsReleaseCache, key: &str, entry: SharedAppEntry) {
  if let Some(fetched) = entry.fetched {
    cache.merge_fetched(key, fetched.latest_release, fetched.etag, fetched.fetched_at);
  }
  if let Some(failure) = entry.failure {
    cache.merge_failure(key, failure.failed_at, failure.consecutive_failures);
  }
}

#[cfg(test)]
mod tests {
  use chrono::DateTime;

  use redis_schema::keys::craft_apps::craft_app_release_redis_key::CraftAppReleaseRedisKey;

  use crate::threads::poll_craft_app_releases_thread::config::default_apps;
  use crate::threads::poll_craft_app_releases_thread::shared_entry::{decode_app_entry, encode_fetch};

  use super::*;

  /// Live: real rounds against GitHub (two apps), then the endpoint's response, written to
  /// `CRAFT_APPS_SAMPLE_OUT` when set (ArtCraft Launcher's parser reads it back). Uses
  /// `CRAFT_APPS_RELEASE_GITHUB_TOKEN` if set, and works without it.
  /// `cargo test -p storyteller-web --bin storyteller-web live_round -- --ignored`
  #[tokio::test]
  #[ignore = "needs network access"]
  async fn live_round_fetches_real_releases() {
    use artcraft_api_defs::craft_apps::release_info::{CraftAppsImportantNotices, GetCraftAppsReleaseInfoResponse};
    let mut config = config();
    config.github_token = std::env::var("CRAFT_APPS_RELEASE_GITHUB_TOKEN").ok().filter(|t| !t.trim().is_empty());
    config.apps.retain(|(key, _, _)| key == "photocraft" || key == "artcraft-launcher");
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&config.apps);
    let mut poller = Poller::new(github::build_client().unwrap(), cache.clone(), config.clone(), None);
    let due: Vec<(String, String)> = config.apps.iter().map(|(k, _, r)| (k.clone(), r.clone())).collect();
    let (round, _) = poller.poll_round(&due, PollMode::Standalone).await;
    assert_eq!(round.fetched + round.unchanged, 2, "{round:?}");
    // A second round revalidates with ETags.
    let (again, _) = poller.poll_round(&due, PollMode::Standalone).await;
    assert_eq!(again.fetched + again.unchanged, 2, "{again:?}");
    let info = cache.snapshot();
    assert!(info.last_fetched.is_some());
    let photocraft = info.apps["photocraft"].latest_release.as_ref().unwrap();
    assert!(photocraft.assets.iter().any(|a| a.platform.is_some() && a.sha256.is_some()));
    let response = GetCraftAppsReleaseInfoResponse {
      success: true,
      release_info: info,
      app_renames: Vec::new(),
      repo_renames: Vec::new(),
      important_notices: CraftAppsImportantNotices::default(),
    };
    let json = serde_json::to_string_pretty(&response).unwrap();
    if let Ok(path) = std::env::var("CRAFT_APPS_SAMPLE_OUT") {
      std::fs::write(path, &json).unwrap();
    }
  }

  /// Against a real Redis (`CRAFT_APPS_TEST_REDIS_URL`, default `redis://127.0.0.1:6379/`): an
  /// instance takes another's fresh fetches from Redis instead of asking GitHub.
  /// `cargo test -p storyteller-web --bin storyteller-web takes_fresh -- --ignored`
  #[tokio::test]
  #[ignore = "needs a Redis server"]
  async fn takes_fresh_releases_from_redis_instead_of_github() {
    let url = std::env::var("CRAFT_APPS_TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    let pool = Pool::builder().max_size(2).build(Client::open(url).unwrap()).unwrap();
    let mut config = config();
    let prefix = format!("test-craft-{}", rand::random::<u32>());
    config.apps = vec![(format!("{prefix}-a"), "A".into(), "storytold/a".into()), (format!("{prefix}-b"), "B".into(), "storytold/b".into())];
    let pod_a = SharedStore::new(pool.clone(), "pod-a".into());
    let now = Utc::now();
    for (key, _, _) in &config.apps {
      assert!(pod_a.write_fetched(key, None, Some("\"e\""), now).await.unwrap());
    }

    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&config.apps);
    let mut pod_b = Poller::new(github::build_client().unwrap(), cache.clone(), config.clone(), Some(SharedStore::new(pool.clone(), "pod-b".into())));
    let report = pod_b.tick().await;
    assert_eq!(report.mode, PollMode::Coordinated);
    assert_eq!(report.round.requests(), 0, "fresh in Redis, so GitHub isn't asked");
    for (key, _, _) in &config.apps {
      assert_eq!(fetched_at(&cache, key).map(|t| t.timestamp_millis()), Some(now.timestamp_millis()));
      assert_eq!(cache.etag(key).as_deref(), Some("\"e\""));
    }

    let keys: Vec<String> = config.apps.iter().map(|(k, _, _)| CraftAppReleaseRedisKey::new_for_app(k).to_string()).collect();
    let mut conn = pool.get().unwrap();
    redis::cmd("DEL").arg(&keys).query::<()>(&mut *conn).unwrap();
  }

  #[tokio::test]
  async fn an_unreachable_redis_falls_back_to_polling_alone() {
    let pool = Pool::builder().max_size(1).build_unchecked(Client::open("redis://127.0.0.1:1/").unwrap());
    let mut poller = fresh_poller(Some(SharedStore::new(pool, "pod".into())));
    let report = poller.tick().await;
    assert_eq!(report.mode, PollMode::Standalone);
    assert_eq!(report.round.requests(), 0, "everything was fresh");
    assert!(poller.redis_log.is_failing());
  }

  #[tokio::test]
  async fn with_coordination_off_redis_is_never_used() {
    let mut poller = fresh_poller(None);
    let report = poller.tick().await;
    assert_eq!(report.mode, PollMode::Standalone);
    assert!(!poller.redis_log.is_failing());
  }

  #[tokio::test]
  async fn an_active_backoff_skips_github_even_when_apps_are_due() {
    let mut poller = Poller::new(github::build_client().unwrap(), registered_cache(), config(), None);
    poller.backoff = Backoff { until: Some(Utc::now() + chrono::Duration::minutes(5)), trouble: 1 };
    let report = poller.tick().await;
    assert_eq!(report.round.requests(), 0);
    assert_eq!(fetched_at(&poller.cache, "photocraft"), None);
  }

  #[test]
  fn shared_entries_merge_into_memory() {
    let cache = registered_cache();
    let at = Utc::now();
    let fields = encode_fetch(None, Some("\"e\""), at, "pod").unwrap();
    let map = fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    merge_shared_entry(&cache, "photocraft", decode_app_entry(&map).unwrap());
    assert_eq!(fetched_at(&cache, "photocraft").map(|t| t.timestamp_millis()), Some(at.timestamp_millis()));
    assert_eq!(cache.etag("photocraft").as_deref(), Some("\"e\""));
  }

  fn config() -> CraftAppsPollConfig {
    CraftAppsPollConfig {
      enabled: true,
      coordination_enabled: true,
      interval: Duration::from_secs(1800),
      max_backoff: Duration::from_secs(14_400),
      github_token: None,
      apps: default_apps(),
    }
  }

  fn registered_cache() -> CraftAppsReleaseCache {
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&config().apps);
    cache
  }

  /// A poller whose apps were all just fetched, so a tick makes no GitHub requests.
  fn fresh_poller(store: Option<SharedStore>) -> Poller {
    let cache = registered_cache();
    for (key, _, _) in &config().apps {
      cache.store(key, None, None, Utc::now());
    }
    Poller::new(github::build_client().unwrap(), cache, config(), store)
  }

  fn fetched_at(cache: &CraftAppsReleaseCache, app_key: &str) -> Option<DateTime<Utc>> {
    cache.app(app_key).and_then(|app| app.app_last_fetched)
  }
}
