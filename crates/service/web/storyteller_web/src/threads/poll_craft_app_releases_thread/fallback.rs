//! Falling back to polling alone when Redis is unreachable, and saying so without flooding the
//! log every tick.

use std::time::{Duration, Instant};

use log::{info, warn};

/// While Redis keeps failing, repeat the warning at most this often.
const WARN_EVERY: Duration = Duration::from_secs(10 * 60);

/// How this tick works.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollMode {
  /// With the fleet: shared releases, claims and backoff in Redis.
  Coordinated,
  /// Alone, from this instance's memory: coordination is off or Redis is unreachable. Same
  /// interval and backoff rules, so a Redis outage never stops releases from refreshing.
  Standalone,
}

/// Remembers Redis trouble so it is logged when it starts, every [`WARN_EVERY`] while it lasts,
/// and when it ends.
#[derive(Debug, Default)]
pub struct RedisTroubleLog {
  failing_since: Option<Instant>,
  last_warned: Option<Instant>,
  suppressed: u32,
}

impl PollMode {
  /// Coordinated only when coordination is on and the shared state could be read.
  pub fn choose(coordination_enabled: bool, shared_state_readable: bool) -> Self {
    if coordination_enabled && shared_state_readable { Self::Coordinated } else { Self::Standalone }
  }
}

impl RedisTroubleLog {
  /// Notes a Redis error; returns whether it was logged.
  pub fn failed(&mut self, err: &anyhow::Error, now: Instant) -> bool {
    self.failing_since.get_or_insert(now);
    if self.last_warned.is_some_and(|last| now.duration_since(last) < WARN_EVERY) {
      self.suppressed = self.suppressed.saturating_add(1);
      return false;
    }
    warn!(
      "Craft app releases: Redis unavailable ({:#}); polling GitHub on this instance alone until it is back ({} similar errors not logged).",
      err, self.suppressed,
    );
    self.last_warned = Some(now);
    self.suppressed = 0;
    true
  }

  /// Notes that Redis works; logs the recovery once.
  pub fn succeeded(&mut self, now: Instant) {
    if let Some(since) = self.failing_since.take() {
      info!("Craft app releases: Redis is back after {:?}; coordinating with the fleet again.", now.duration_since(since));
      self.last_warned = None;
      self.suppressed = 0;
    }
  }

  pub fn is_failing(&self) -> bool {
    self.failing_since.is_some()
  }
}

#[cfg(test)]
mod tests {
  use anyhow::anyhow;

  use super::*;

  #[test]
  fn coordinates_only_when_enabled_and_redis_answers() {
    assert_eq!(PollMode::choose(true, true), PollMode::Coordinated);
    assert_eq!(PollMode::choose(true, false), PollMode::Standalone);
    assert_eq!(PollMode::choose(false, true), PollMode::Standalone);
    assert_eq!(PollMode::choose(false, false), PollMode::Standalone);
  }

  #[test]
  fn warns_once_per_window_while_redis_keeps_failing() {
    let mut log = RedisTroubleLog::default();
    let start = Instant::now();
    let err = anyhow!("connection refused");
    assert!(log.failed(&err, start));
    assert!(!log.failed(&err, start + Duration::from_secs(30)));
    assert!(!log.failed(&err, start + Duration::from_secs(599)));
    assert!(log.failed(&err, start + Duration::from_secs(600)));
    assert!(log.is_failing());

    log.succeeded(start + Duration::from_secs(700));
    assert!(!log.is_failing());
    assert!(log.failed(&err, start + Duration::from_secs(730)), "a new outage warns right away");
  }
}
