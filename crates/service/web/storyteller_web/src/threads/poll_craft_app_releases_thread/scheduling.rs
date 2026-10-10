//! When to ask GitHub: per-app freshness, retries after failures, and the backoff after rate
//! limits. Pure functions of what's known, so the same rules apply whether this instance works
//! from the fleet's shared state in Redis or (as a fallback) only from its own memory.

use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::state::craft_apps_release_cache::CachedCraftApp;
use crate::threads::poll_craft_app_releases_thread::config::CraftAppsPollConfig;

/// How often every instance wakes up to look at the shared state (with jitter).
pub const TICK: Duration = Duration::from_secs(30);
/// Allowance for clocks that differ between instances: an app fetched by another instance
/// counts as fresh until `interval - CLOCK_SKEW_MARGIN` has passed.
pub const CLOCK_SKEW_MARGIN: Duration = Duration::from_secs(30);
/// First wait after a failed request (or a round where every request failed); doubles up to the
/// normal interval.
const FIRST_FAILURE_RETRY: Duration = Duration::from_secs(60);
/// Extra margin after GitHub's announced rate-limit reset.
const RESET_MARGIN: Duration = Duration::from_secs(60);

/// What one round did.
#[derive(Debug, Default)]
pub struct RoundResult {
  pub fetched: usize,
  pub unchanged: usize,
  pub failed: usize,
  /// GitHub refused; `Some(Some(t))` when it said when to come back.
  pub rate_limited: Option<Option<DateTime<Utc>>>,
}

/// Don't ask GitHub before `until`. `trouble` counts the rounds in a row that were rate limited
/// or failed entirely; each one makes the next wait longer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Backoff {
  pub until: Option<DateTime<Utc>>,
  pub trouble: u32,
}

impl RoundResult {
  /// How many requests reached a conclusion (a rate limit counts as one).
  pub fn requests(&self) -> usize {
    self.fetched + self.unchanged + self.failed + usize::from(self.rate_limited.is_some())
  }
}

impl Backoff {
  pub fn is_active(&self, now: DateTime<Utc>) -> bool {
    self.until.is_some_and(|until| until > now)
  }
}

/// Whether `app` should be requested from GitHub now. An app is due once its last fetch (by any
/// instance) is `interval - CLOCK_SKEW_MARGIN` old. After a failure it waits a minute, then two,
/// four, … up to the interval, so one broken repository is never retried every tick.
pub fn is_due(app: &CachedCraftApp, now: DateTime<Utc>, interval: Duration) -> bool {
  if let Some(fetched) = app.app_last_fetched {
    if age(fetched, now) < interval.saturating_sub(CLOCK_SKEW_MARGIN) {
      return false;
    }
  }
  if app.is_failing() {
    if let Some(failed) = app.last_failed_at {
      let retry = scaled(FIRST_FAILURE_RETRY, app.consecutive_failures).min(interval);
      if age(failed, now) < retry.saturating_sub(CLOCK_SKEW_MARGIN) {
        return false;
      }
    }
  }
  true
}

/// The backoff after a round that made at least one request, starting from `previous`. A good
/// round clears it.
pub fn backoff_after(config: &CraftAppsPollConfig, round: &RoundResult, previous: Backoff, now: DateTime<Utc>) -> Backoff {
  let mut trouble = previous.trouble;
  let wait = next_wait(config, round, &mut trouble, now);
  if trouble == 0 {
    return Backoff::default();
  }
  let wait = chrono::Duration::from_std(wait).unwrap_or(chrono::Duration::MAX);
  Backoff { until: now.checked_add_signed(wait), trouble }
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

/// `base` ± `fraction`, so a fleet doesn't poll in lockstep. With `fraction == 1.0`, anywhere in
/// `0..=2×base`; used for the startup spread.
pub fn jittered(base: Duration, fraction: f64) -> Duration {
  let factor = 1.0 + fraction * (rand::random::<f64>() * 2.0 - 1.0);
  base.mul_f64(factor.max(0.0))
}

/// `base × 2^(n-1)`, without overflow.
fn scaled(base: Duration, n: u32) -> Duration {
  base.saturating_mul(1u32 << n.saturating_sub(1).min(16))
}

/// How long ago `then` was; zero if it is in the future (another instance's clock runs ahead).
fn age(then: DateTime<Utc>, now: DateTime<Utc>) -> Duration {
  (now - then).to_std().unwrap_or(Duration::ZERO)
}

#[cfg(test)]
mod tests {
  use crate::threads::poll_craft_app_releases_thread::config::default_apps;

  use super::*;

  mod freshness {
    use super::*;

    #[test]
    fn an_app_never_fetched_is_due() {
      assert!(is_due(&app(None, None, 0), Utc::now(), Duration::from_secs(120)));
    }

    #[test]
    fn a_fetch_counts_as_fresh_until_the_interval_minus_the_skew_margin() {
      let now = Utc::now();
      let interval = Duration::from_secs(120);
      assert!(!is_due(&app(Some(now - secs(89)), None, 0), now, interval));
      assert!(is_due(&app(Some(now - secs(90)), None, 0), now, interval));
      assert!(is_due(&app(Some(now - secs(600)), None, 0), now, interval));
    }

    #[test]
    fn a_fetch_from_a_clock_ahead_of_ours_is_fresh() {
      let now = Utc::now();
      assert!(!is_due(&app(Some(now + secs(20)), None, 0), now, Duration::from_secs(120)));
    }

    #[test]
    fn a_failing_app_waits_longer_after_each_failure_up_to_the_interval() {
      let now = Utc::now();
      let interval = Duration::from_secs(1800);
      // 1st failure: retry after 60 s (less the margin); 3rd: 240 s.
      assert!(!is_due(&app(None, Some(now - secs(29)), 1), now, interval));
      assert!(is_due(&app(None, Some(now - secs(30)), 1), now, interval));
      assert!(!is_due(&app(None, Some(now - secs(200)), 3), now, interval));
      assert!(is_due(&app(None, Some(now - secs(210)), 3), now, interval));
      // Many failures: never longer than the interval.
      assert!(is_due(&app(None, Some(now - secs(1770)), 30), now, interval));
    }

    #[test]
    fn a_failure_older_than_the_last_fetch_is_ignored() {
      let now = Utc::now();
      let app = app(Some(now - secs(200)), Some(now - secs(300)), 5);
      assert!(is_due(&app, now, Duration::from_secs(120)));
    }
  }

  mod backoff {
    use super::*;

    #[test]
    fn a_good_round_clears_the_backoff() {
      let now = Utc::now();
      let round = RoundResult { fetched: 1, unchanged: 3, failed: 1, ..RoundResult::default() };
      let previous = Backoff { until: Some(now), trouble: 3 };
      assert_eq!(backoff_after(&config(), &round, previous, now), Backoff::default());
    }

    #[test]
    fn a_rate_limit_backs_off_the_fleet_until_after_the_reset() {
      let now = Utc::now();
      let round = RoundResult { rate_limited: Some(Some(now + chrono::Duration::minutes(50))), ..RoundResult::default() };
      let b = backoff_after(&config(), &round, Backoff::default(), now);
      assert_eq!(b.trouble, 1);
      assert!(b.is_active(now + chrono::Duration::minutes(45)));
      assert!(!b.is_active(now + chrono::Duration::minutes(57)));
    }

    #[test]
    fn trouble_carries_over_between_rounds_and_instances() {
      let now = Utc::now();
      let round = RoundResult { failed: 2, ..RoundResult::default() };
      let first = backoff_after(&config(), &round, Backoff::default(), now);
      let second = backoff_after(&config(), &round, first, now);
      assert_eq!((first.trouble, second.trouble), (1, 2));
      assert!(second.until.unwrap() - now >= chrono::Duration::seconds(107), "{:?}", second);
    }

    #[test]
    fn an_expired_backoff_is_not_active() {
      let now = Utc::now();
      assert!(!Backoff::default().is_active(now));
      assert!(!Backoff { until: Some(now - secs(1)), trouble: 4 }.is_active(now));
      assert!(Backoff { until: Some(now + secs(1)), trouble: 4 }.is_active(now));
    }
  }

  mod waits {
    use super::*;

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
        assert!(w >= Duration::from_secs(53) && w <= Duration::from_secs(1800).mul_f64(1.11), "{:?}", w);
      }
    }
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

  fn app(fetched: Option<DateTime<Utc>>, failed: Option<DateTime<Utc>>, failures: u32) -> CachedCraftApp {
    CachedCraftApp {
      name: "PhotoCraft".into(),
      repo: "storytold/photocraft".into(),
      latest_release: None,
      app_last_fetched: fetched,
      etag: None,
      last_failed_at: failed,
      consecutive_failures: failures,
    }
  }

  fn secs(n: i64) -> chrono::Duration {
    chrono::Duration::seconds(n)
  }

  fn within(d: Duration, around: Duration) -> bool {
    d >= around.mul_f64(0.89) && d <= around.mul_f64(1.11)
  }
}
