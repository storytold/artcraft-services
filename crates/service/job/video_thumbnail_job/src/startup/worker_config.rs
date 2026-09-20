use std::num::NonZeroUsize;
use std::time::Duration;

use anyhow::{ensure, Result};

pub struct WorkerConfig {
  pub concurrency: NonZeroUsize,
  pub retry_cache_capacity: NonZeroUsize,
  pub retry_initial_delay: Duration,
  pub retry_max_delay: Duration,
}

impl WorkerConfig {
  pub fn from_env() -> Result<Self> {
    Self::new(
      easyenv::get_env_num("VIDEO_THUMBNAIL_CONCURRENCY", 1usize)?,
      easyenv::get_env_num("VIDEO_THUMBNAIL_RETRY_CACHE_CAPACITY", 10_000usize)?,
      easyenv::get_env_num("VIDEO_THUMBNAIL_RETRY_INITIAL_DELAY_MILLIS", 30_000u64)?,
      easyenv::get_env_num("VIDEO_THUMBNAIL_RETRY_MAX_DELAY_MILLIS", 1_800_000u64)?,
    )
  }

  fn new(concurrency: usize, capacity: usize, initial_ms: u64, max_ms: u64) -> Result<Self> {
    ensure!(concurrency > 0, "VIDEO_THUMBNAIL_CONCURRENCY must be positive");
    ensure!(capacity > 0, "VIDEO_THUMBNAIL_RETRY_CACHE_CAPACITY must be positive");
    ensure!(initial_ms > 0, "VIDEO_THUMBNAIL_RETRY_INITIAL_DELAY_MILLIS must be positive");
    ensure!(max_ms >= initial_ms, "VIDEO_THUMBNAIL_RETRY_MAX_DELAY_MILLIS must be >= the initial delay");
    ensure!(max_ms <= 86_400_000, "VIDEO_THUMBNAIL_RETRY_MAX_DELAY_MILLIS must be <= one day");
    Ok(Self {
      concurrency: NonZeroUsize::new(concurrency).unwrap(),
      retry_cache_capacity: NonZeroUsize::new(capacity).unwrap(),
      retry_initial_delay: Duration::from_millis(initial_ms),
      retry_max_delay: Duration::from_millis(max_ms),
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_settings_that_disable_processing_or_retry_protection() {
    for (concurrency, capacity, initial, max) in [(0, 10, 30, 60), (1, 0, 30, 60), (1, 10, 0, 60), (1, 10, 60, 30)] {
      assert!(WorkerConfig::new(concurrency, capacity, initial, max).is_err());
    }
  }

  #[test]
  fn accepts_configurable_concurrency() {
    for concurrency in 1..=4 {
      assert_eq!(WorkerConfig::new(concurrency, 10_000, 30_000, 1_800_000).unwrap().concurrency.get(), concurrency);
    }
  }
}
