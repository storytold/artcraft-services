//! The fleet-wide GitHub backoff for Craft App release polling: after GitHub rate-limits us (or
//! every request of a round fails), no storyteller-web instance asks GitHub again until it ends.
//!
//! A Redis hash, written by `poll_craft_app_releases_thread` (storyteller-web), which documents
//! the fields. It outlives the backoff itself so repeated trouble keeps growing the wait; it is
//! deleted after a good round.

use chrono::Duration;

pub struct CraftAppsReleaseBackoffRedisKey(pub String);

impl_string_key!(CraftAppsReleaseBackoffRedisKey);

const REDIS_KEY_TTL_DURATION: Duration = Duration::milliseconds(1000 * 60 * 60 * 24);

impl CraftAppsReleaseBackoffRedisKey {
  pub fn new_for_fleet() -> Self {
    Self("craftApps:releases:backoff".to_string())
  }

  pub fn get_redis_ttl() -> Duration {
    REDIS_KEY_TTL_DURATION
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_for_fleet() {
    assert_eq!(CraftAppsReleaseBackoffRedisKey::new_for_fleet().as_str(), "craftApps:releases:backoff");
  }

  #[test]
  fn test_duration() {
    assert_eq!(CraftAppsReleaseBackoffRedisKey::get_redis_ttl().num_hours(), 24);
  }
}
