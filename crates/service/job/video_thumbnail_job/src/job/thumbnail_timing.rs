use std::time::Instant;

use chrono::{DateTime, TimeDelta, Utc};
use log::info;
use serde_json::json;

use mysql_queries::queries::media_files::thumbnails::list_video_media_files_without_thumbnails_for_job::VideoMediaFileWithoutThumbnail;

/// Measures the complete thumbnail attempt: download through uploads and DB update.
pub struct ThumbnailTiming<'a> {
  media_file: &'a VideoMediaFileWithoutThumbnail,
  clock: DatabaseClock,
  started_at: Instant,
}

struct DatabaseClock {
  database_read_at: DateTime<Utc>,
  query_started_at: Instant,
  query_finished_at: Instant,
}

struct LatencySnapshot {
  estimated_database_at: DateTime<Utc>,
  maybe_since_generation_completed_ms: Option<i64>,
  since_media_created_ms: i64,
}

impl<'a> ThumbnailTiming<'a> {
  pub fn start(
    media_file: &'a VideoMediaFileWithoutThumbnail,
    query_started_at: Instant,
    query_finished_at: Instant,
  ) -> Self {
    let timing = Self {
      media_file,
      clock: DatabaseClock { database_read_at: media_file.database_read_at, query_started_at, query_finished_at },
      started_at: Instant::now(),
    };

    timing.log_event("started", timing.started_at);
    timing
  }

  pub fn finish(&self, success: bool) {
    self.log_event(if success { "completed" } else { "failed" }, Instant::now());
  }

  pub fn log_stage(&self, stage: &str, started_at: Instant, success: bool) {
    info!(
      "thumbnail_stage {}",
      json!({
        "media_file_token": self.media_file.token.as_str(),
        "stage": stage,
        "success": success,
        "duration_ms": started_at.elapsed().as_millis(),
      })
    );
  }

  fn log_event(&self, event: &str, now: Instant) {
    let snapshot =
      self.clock.snapshot_at(now, self.media_file.created_at, self.media_file.maybe_generation_completed_at);
    let query_duration = self.clock.query_finished_at.duration_since(self.clock.query_started_at);

    info!(
      "thumbnail_timing {}",
      json!({
        "event": event,
        "media_file_token": self.media_file.token.as_str(),
        "media_created_at": self.media_file.created_at,
        "generation_completed_at": self.media_file.maybe_generation_completed_at,
        "database_read_at": self.clock.database_read_at,
        "estimated_database_at": snapshot.estimated_database_at,
        "since_generation_completed_ms": snapshot.maybe_since_generation_completed_ms,
        "since_media_created_ms": snapshot.since_media_created_ms,
        "processing_elapsed_ms": now.duration_since(self.started_at).as_millis(),
        "page_wait_ms": self.started_at.duration_since(self.clock.query_finished_at).as_millis(),
        "database_query_ms": query_duration.as_millis(),
        "clock_uncertainty_ms": query_duration.as_nanos().div_ceil(2_000_000),
        "source_timestamp_precision_ms": 1000,
      })
    );
  }
}

impl DatabaseClock {
  fn snapshot_at(
    &self,
    now: Instant,
    media_created_at: DateTime<Utc>,
    maybe_generation_completed_at: Option<DateTime<Utc>>,
  ) -> LatencySnapshot {
    // NOW(6) is sampled at statement start, somewhere between these two Instants.
    // Use the midpoint as an estimate with +/- half the query duration uncertainty.
    // This includes pool wait, execution, and network time; anchoring NOW() at the
    // response instead would systematically omit query time from every latency.
    // The completion/creation columns themselves have only whole-second precision.
    let query_duration = self.query_finished_at.duration_since(self.query_started_at);
    let midpoint = self.query_started_at + query_duration / 2;
    let elapsed = TimeDelta::from_std(now.duration_since(midpoint))
      .expect("thumbnail measurement duration fits in chrono::TimeDelta");
    let estimated_database_at = self.database_read_at + elapsed;

    LatencySnapshot {
      estimated_database_at,
      // Preserve missing and negative values instead of fabricating a zero delay.
      maybe_since_generation_completed_ms: maybe_generation_completed_at
        .map(|completed_at| (estimated_database_at - completed_at).num_milliseconds()),
      since_media_created_ms: (estimated_database_at - media_created_at).num_milliseconds(),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;

  mod database_clock {
    use super::*;

    #[test]
    fn includes_query_time_and_time_spent_waiting_behind_other_files() {
      let clock = clock_with_two_second_query();
      let created_at = clock.database_read_at - TimeDelta::seconds(12);
      let completed_at = clock.database_read_at - TimeDelta::seconds(10);
      let started_at = clock.query_finished_at + Duration::from_secs(60);

      let start = clock.snapshot_at(started_at, created_at, Some(completed_at));
      let end = clock.snapshot_at(started_at + Duration::from_secs(8), created_at, Some(completed_at));

      assert_eq!(start.maybe_since_generation_completed_ms, Some(71_000));
      assert_eq!(end.maybe_since_generation_completed_ms, Some(79_000));
      assert_eq!(start.since_media_created_ms, 73_000);
      assert_eq!(end.estimated_database_at - start.estimated_database_at, TimeDelta::seconds(8));
    }

    #[test]
    fn missing_job_completion_does_not_become_media_creation_or_zero() {
      let clock = clock_with_two_second_query();
      let created_at = clock.database_read_at - TimeDelta::seconds(30);

      let snapshot = clock.snapshot_at(clock.query_finished_at, created_at, None);

      assert_eq!(snapshot.maybe_since_generation_completed_ms, None);
      assert_eq!(snapshot.since_media_created_ms, 31_000);
    }

    #[test]
    fn negative_latency_is_preserved_for_diagnosing_timestamp_anomalies() {
      let clock = clock_with_two_second_query();
      let completed_at = clock.database_read_at + TimeDelta::seconds(2);

      let snapshot = clock.snapshot_at(clock.query_finished_at, clock.database_read_at, Some(completed_at));

      assert_eq!(snapshot.maybe_since_generation_completed_ms, Some(-1_000));
    }
  }

  fn clock_with_two_second_query() -> DatabaseClock {
    let query_started_at = Instant::now();
    DatabaseClock {
      // Intentionally unrelated to the worker's wall clock.
      database_read_at: DateTime::from_timestamp(1_000_000, 0).unwrap(),
      query_started_at,
      query_finished_at: query_started_at + Duration::from_secs(2),
    }
  }
}
