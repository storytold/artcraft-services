//! The latest release of one Craft App, shared by every storyteller-web instance so the fleet
//! polls GitHub once per app instead of once per pod.
//!
//! A Redis hash, written by `poll_craft_app_releases_thread` (storyteller-web), which documents
//! the fields. Expires if nobody refreshes it for a week, which only happens if polling stops.

use chrono::Duration;

pub struct CraftAppReleaseRedisKey(pub String);

impl_string_key!(CraftAppReleaseRedisKey);

const REDIS_KEY_TTL_DURATION: Duration = Duration::milliseconds(1000 * 60 * 60 * 24 * 7);

impl CraftAppReleaseRedisKey {
  /// `app_key` is the app's lowercase machine name (`photocraft`, `artcraft-launcher`).
  pub fn new_for_app(app_key: &str) -> Self {
    let key = format!("craftApps:releases:app:{}", app_key);
    Self(key)
  }

  pub fn get_redis_ttl() -> Duration {
    REDIS_KEY_TTL_DURATION
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_for_app() {
    let key = CraftAppReleaseRedisKey::new_for_app("photocraft");
    assert_eq!(key.as_str(), "craftApps:releases:app:photocraft");
  }

  #[test]
  fn test_duration() {
    assert_eq!(CraftAppReleaseRedisKey::get_redis_ttl().num_days(), 7);
  }
}
