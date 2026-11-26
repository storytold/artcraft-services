//! The release history of one Craft App (every published release, with its files), shared by
//! every storyteller-web instance next to `CraftAppReleaseRedisKey`, and kept apart from it so
//! the fleet's every-tick read of the latest releases stays small.
//!
//! A Redis string (JSON), written by `poll_craft_app_releases_thread` (storyteller-web), which
//! documents it. Expires if nobody refreshes it for a week, which only happens if polling stops.

use chrono::Duration;

pub struct CraftAppReleaseHistoryRedisKey(pub String);

impl_string_key!(CraftAppReleaseHistoryRedisKey);

const REDIS_KEY_TTL_DURATION: Duration = Duration::milliseconds(1000 * 60 * 60 * 24 * 7);

impl CraftAppReleaseHistoryRedisKey {
  /// `app_key` is the app's lowercase machine name (`photocraft`, `artcraft-launcher`).
  pub fn new_for_app(app_key: &str) -> Self {
    let key = format!("craftApps:releases:v2:history:{}", app_key);
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
    let key = CraftAppReleaseHistoryRedisKey::new_for_app("photocraft");
    assert_eq!(key.as_str(), "craftApps:releases:v2:history:photocraft");
  }
}
