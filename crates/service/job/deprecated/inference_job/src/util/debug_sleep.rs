use std::time::Duration;

/// Upper bound for the optional debug sleep. The value arrives in an enqueue request header,
/// so it must not be able to park a worker indefinitely.
const MAX_DEBUG_SLEEP: Duration = Duration::from_secs(60);

pub fn debug_sleep_duration(millis: u64) -> Duration {
  Duration::from_millis(millis).min(MAX_DEBUG_SLEEP)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clamps_hostile_values() {
    assert_eq!(debug_sleep_duration(250), Duration::from_millis(250));
    assert_eq!(debug_sleep_duration(u64::MAX), MAX_DEBUG_SLEEP);
  }
}
