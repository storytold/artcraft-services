//! Polls GitHub for the latest release of every Craft App and keeps it in
//! [`CraftAppsReleaseCache`], which `GET /v1/craft_apps/release_info` serves to ArtCraft Launcher
//! (so thousands of launchers don't each poll GitHub).
//!
//! Every instance of storyteller-web polls on its own, so it is gentle: one round at startup,
//! then one per `CRAFT_APPS_RELEASE_POLL_INTERVAL_SECS` (30 minutes by default) with jitter, one
//! request per app with ETag revalidation, and growing waits after rate limits or failures.
//! Each app is all-or-nothing: either its latest release is stored with a fresh
//! `app_last_fetched`, or nothing about it changes.

pub mod config;
pub mod github;

use std::time::Duration;

use chrono::{DateTime, Utc};
use log::{error, info, warn};

use crate::state::craft_apps_release_cache::CraftAppsReleaseCache;
use config::CraftAppsPollConfig;
use github::FetchOutcome;

/// Spreads a fleet's first round over this window, so a deploy doesn't hit GitHub all at once.
const STARTUP_JITTER: Duration = Duration::from_secs(15);
/// Pause between the requests of one round.
const BETWEEN_REQUESTS: Duration = Duration::from_millis(1_000);
/// First wait after a round where every request failed (network trouble); doubles up to the
/// normal interval.
const FIRST_FAILURE_RETRY: Duration = Duration::from_secs(60);
/// Extra margin after GitHub's announced rate-limit reset.
const RESET_MARGIN: Duration = Duration::from_secs(60);

pub async fn poll_craft_app_releases(cache: CraftAppsReleaseCache, config: CraftAppsPollConfig) {
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
    "Polling GitHub for {} craft app releases every {:?} (token: {}).",
    config.apps.len(), config.interval, config.github_token.is_some(),
  );
  tokio::time::sleep(jittered(STARTUP_JITTER, 1.0)).await;

  let mut consecutive_trouble = 0u32;
  loop {
    let round = poll_round(&client, &cache, &config).await;
    let wait = next_wait(&config, &round, &mut consecutive_trouble, Utc::now());
    info!(
      "Craft app releases: {} fetched, {} unchanged, {} failed{}; next round in {:?}.",
      round.fetched, round.unchanged, round.failed,
      if round.rate_limited.is_some() { ", rate limited" } else { "" },
      wait,
    );
    tokio::time::sleep(wait).await;
  }
}

/// What one round did.
#[derive(Debug, Default)]
struct RoundResult {
  fetched: usize,
  unchanged: usize,
  failed: usize,
  /// GitHub refused; `Some(Some(t))` when it said when to come back.
  rate_limited: Option<Option<DateTime<Utc>>>,
}

async fn poll_round(client: &wreq::Client, cache: &CraftAppsReleaseCache, config: &CraftAppsPollConfig) -> RoundResult {
  let mut round = RoundResult::default();
  for (i, (key, _, repo)) in config.apps.iter().enumerate() {
    if i > 0 {
      tokio::time::sleep(BETWEEN_REQUESTS).await;
    }
    let etag = cache.etag(key);
    match github::fetch_latest_release(client, repo, etag.as_deref(), config.github_token.as_deref()).await {
      FetchOutcome::Fetched { release, etag } => {
        cache.store(key, release, etag, Utc::now());
        round.fetched += 1;
      }
      FetchOutcome::NotModified => {
        cache.confirm_unchanged(key, Utc::now());
        round.unchanged += 1;
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
      }
    }
  }
  round
}

/// How long to wait before the next round. Rate limits wait for GitHub's reset and back off
/// further each time; rounds where everything failed retry sooner, but never faster than a
/// minute, doubling up to the normal interval.
fn next_wait(config: &CraftAppsPollConfig, round: &RoundResult, consecutive_trouble: &mut u32, now: DateTime<Utc>) -> Duration {
  let base = if let Some(retry_at) = round.rate_limited {
    *consecutive_trouble = consecutive_trouble.saturating_add(1);
    let until_reset = retry_at
        .and_then(|t| (t - now).to_std().ok())
        .map(|d| d + RESET_MARGIN)
        .unwrap_or(Duration::ZERO);
    let backoff = scaled(config.interval, *consecutive_trouble);
    until_reset.max(backoff).min(config.max_backoff)
  } else if round.fetched == 0 && round.unchanged == 0 && round.failed > 0 {
    *consecutive_trouble = consecutive_trouble.saturating_add(1);
    scaled(FIRST_FAILURE_RETRY, *consecutive_trouble).min(config.interval)
  } else {
    *consecutive_trouble = 0;
    config.interval
  };
  jittered(base, 0.1)
}

/// `base × 2^(n-1)`, without overflow.
fn scaled(base: Duration, n: u32) -> Duration {
  base.saturating_mul(1u32 << n.saturating_sub(1).min(16))
}

/// `base` ± `fraction`, so a fleet doesn't poll in lockstep. With `fraction == 1.0`, anywhere in
/// `0..=2×base`; used for the startup spread.
fn jittered(base: Duration, fraction: f64) -> Duration {
  let factor = 1.0 + fraction * (rand::random::<f64>() * 2.0 - 1.0);
  base.mul_f64(factor.max(0.0))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn config() -> CraftAppsPollConfig {
    CraftAppsPollConfig {
      enabled: true,
      interval: Duration::from_secs(1800),
      max_backoff: Duration::from_secs(14_400),
      github_token: None,
      apps: config::default_apps(),
    }
  }

  fn within(d: Duration, around: Duration) -> bool {
    d >= around.mul_f64(0.89) && d <= around.mul_f64(1.11)
  }

  /// Live: one real round against GitHub (two apps), then the endpoint's response, written to
  /// `CRAFT_APPS_SAMPLE_OUT` when set (ArtCraft Launcher's parser reads it back).
  /// `cargo test -p storyteller-web --bin storyteller-web live_round -- --ignored`
  #[tokio::test]
  #[ignore = "needs network access"]
  async fn live_round_fetches_real_releases() {
    use artcraft_api_defs::craft_apps::release_info::{CraftAppsImportantNotices, GetCraftAppsReleaseInfoResponse};
    let mut config = config();
    config.apps.retain(|(key, _, _)| key == "photocraft" || key == "artcraft-launcher");
    let cache = CraftAppsReleaseCache::new();
    cache.register_apps(&config.apps);
    let client = github::build_client().unwrap();
    let round = poll_round(&client, &cache, &config).await;
    assert_eq!(round.fetched + round.unchanged, 2, "{round:?}");
    // A second round revalidates with ETags.
    let again = poll_round(&client, &cache, &config).await;
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

  #[test]
  fn a_good_round_waits_the_interval_and_resets_trouble() {
    let mut trouble = 3;
    let round = RoundResult { fetched: 5, unchanged: 8, ..RoundResult::default() };
    assert!(within(next_wait(&config(), &round, &mut trouble, Utc::now()), Duration::from_secs(1800)));
    assert_eq!(trouble, 0);
  }

  #[test]
  fn rate_limits_wait_for_the_reset_and_back_off_further_each_time() {
    let now = Utc::now();
    let mut trouble = 0;
    // GitHub says come back in 50 minutes: wait that (plus margin), more than the interval.
    let round = RoundResult { rate_limited: Some(Some(now + chrono::Duration::minutes(50))), ..RoundResult::default() };
    assert!(within(next_wait(&config(), &round, &mut trouble, now), Duration::from_secs(51 * 60)));
    // Again and again, with no reset time: 2×, 4×, … the interval, capped at the maximum.
    let round = RoundResult { rate_limited: Some(None), ..RoundResult::default() };
    assert!(within(next_wait(&config(), &round, &mut trouble, now), Duration::from_secs(3600)));
    assert!(within(next_wait(&config(), &round, &mut trouble, now), Duration::from_secs(7200)));
    for _ in 0..10 {
      assert!(next_wait(&config(), &round, &mut trouble, now) <= Duration::from_secs(14_400).mul_f64(1.11));
    }
  }

  #[test]
  fn a_round_where_everything_failed_retries_sooner_but_not_in_a_tight_loop() {
    let now = Utc::now();
    let mut trouble = 0;
    let round = RoundResult { failed: 13, ..RoundResult::default() };
    assert!(within(next_wait(&config(), &round, &mut trouble, now), Duration::from_secs(60)));
    assert!(within(next_wait(&config(), &round, &mut trouble, now), Duration::from_secs(120)));
    for _ in 0..20 {
      let w = next_wait(&config(), &round, &mut trouble, now);
      assert!(w >= Duration::from_secs(53) && w <= Duration::from_secs(1800).mul_f64(1.11), "{w:?}");
    }
  }
}
