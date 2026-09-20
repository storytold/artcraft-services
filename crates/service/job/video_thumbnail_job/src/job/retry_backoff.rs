use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

/// Process-local failure history. Memory is bounded; restarts/eviction forget failures.
pub struct RetryBackoff {
  failures: HashMap<i64, Failure>,
  capacity: NonZeroUsize,
  initial_delay: Duration,
  max_delay: Duration,
}

struct Failure {
  attempts: u32,
  last_failed_at: Instant,
  retry_at: Instant,
}

pub struct RetrySchedule {
  pub attempts: u32,
  pub delay: Duration,
}

impl RetryBackoff {
  pub fn new(capacity: NonZeroUsize, initial_delay: Duration, max_delay: Duration) -> Self {
    Self { failures: HashMap::new(), capacity, initial_delay, max_delay }
  }

  pub fn is_ready(&self, id: i64, now: Instant) -> bool {
    self.failures.get(&id).is_none_or(|failure| now >= failure.retry_at)
  }

  pub fn has_failed(&self, id: i64) -> bool {
    self.failures.contains_key(&id)
  }

  pub fn record_success(&mut self, id: i64) {
    self.failures.remove(&id);
  }

  pub fn record_failure(&mut self, id: i64, now: Instant) -> RetrySchedule {
    if !self.failures.contains_key(&id) && self.failures.len() == self.capacity.get() {
      // Evict the least recently failed entry. Skipping a cached failure must not
      // refresh its position or allocate an unbounded auxiliary eviction queue.
      if let Some(oldest_id) = self.failures.iter()
        .min_by_key(|(_, failure)| failure.last_failed_at)
        .map(|(id, _)| *id)
      {
        self.failures.remove(&oldest_id);
      }
    }

    let attempts = self.failures.get(&id)
      .map(|failure| failure.attempts.saturating_add(1))
      .unwrap_or(1);
    let multiplier = 1u32 << attempts.saturating_sub(1).min(31);
    let delay = self.initial_delay.saturating_mul(multiplier).min(self.max_delay);
    self.failures.insert(id, Failure { attempts, last_failed_at: now, retry_at: now + delay });

    RetrySchedule { attempts, delay }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn retries_at_the_deadline_with_exponential_delay_and_a_cap() {
    let mut cache = cache(10);
    let mut now = Instant::now();
    for (index, seconds) in [30, 60, 120, 120, 120].into_iter().enumerate() {
      let retry = cache.record_failure(1, now);
      assert_eq!(retry.attempts, index as u32 + 1);
      assert_eq!(retry.delay, Duration::from_secs(seconds));
      assert!(!cache.is_ready(1, now + retry.delay - Duration::from_millis(1)));
      now += retry.delay;
      assert!(cache.is_ready(1, now));
    }
  }

  #[test]
  fn success_forgets_failures_and_resets_the_backoff() {
    let mut cache = cache(10);
    let now = Instant::now();
    cache.record_failure(1, now);
    cache.record_failure(1, now);
    cache.record_success(1);
    assert!(cache.is_ready(1, now));
    assert!(!cache.has_failed(1));
    assert_eq!(cache.record_failure(1, now).attempts, 1);
  }

  #[test]
  fn capacity_is_bounded_and_evicts_the_least_recent_failure() {
    let mut cache = cache(2);
    let now = Instant::now();
    cache.record_failure(1, now);
    cache.record_failure(2, now + Duration::from_secs(1));
    cache.record_failure(1, now + Duration::from_secs(2));
    cache.record_failure(3, now + Duration::from_secs(3));
    assert_eq!(cache.failures.len(), 2);
    assert!(cache.has_failed(1));
    assert!(!cache.has_failed(2));
    assert!(cache.has_failed(3));
    for id in 4..1000 {
      cache.record_failure(id, now + Duration::from_secs(id as u64));
      assert_eq!(cache.failures.len(), 2);
    }
  }

  fn cache(capacity: usize) -> RetryBackoff {
    RetryBackoff::new(NonZeroUsize::new(capacity).unwrap(), Duration::from_secs(30), Duration::from_secs(120))
  }
}
