use std::future::Future;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use futures::{stream, StreamExt};
use log::{error, info, warn};

use mysql_queries::queries::media_files::thumbnails::list_video_media_files_without_thumbnails_for_job::{
  list_video_media_files_without_thumbnails_for_job,
  ListVideoMediaFilesWithoutThumbnailsArgs,
  VideoMediaFileWithoutThumbnail,
};

use crate::job::alert_on_error::alert_pager_and_return_err;
use crate::job_dependencies::JobDependencies;
use crate::job::process_single_media_file::process_single_media_file;
use crate::job::thumbnail_timing::ThumbnailTiming;
use crate::job::memory_monitor::monitor_memory;
use crate::job::retry_backoff::RetryBackoff;

pub async fn main_loop(deps: JobDependencies) {
  let config = &deps.worker_config;
  let mut retry_cache = RetryBackoff::new(config.retry_cache_capacity, config.retry_initial_delay, config.retry_max_delay);
  let memory_monitor = tokio::spawn(monitor_memory(config.concurrency.get()));

  while !deps.application_shutdown.get() {
    let processed_any = match run_batch_cycle(&deps, &mut retry_cache).await {
      Ok(count) => {
        if count > 0 {
          info!("Processed {} video thumbnail(s) this cycle.", count);
        }
        count > 0
      }
      Err(err) => {
        error!("Error in video thumbnail batch cycle: {:?}", err);
        let _ = alert_pager_and_return_err::<()>(&deps.pager, "Video thumbnail batch cycle error", err);
        let _ = deps.job_stats.increment_failure_count();

        // Wait before retrying after a failure.
        tokio::time::sleep(Duration::from_millis(deps.query_failure_retry_delay_millis)).await;
        false
      }
    };

    // If we didn't process anything, sleep for the poll interval before checking again.
    if !processed_any {
      tokio::time::sleep(Duration::from_millis(deps.poll_interval_millis)).await;
    }
  }

  memory_monitor.abort();
  warn!("Video thumbnail job main loop is shut down.");
}

/// Run one full pagination cycle: keep querying pages of media files without thumbnails
/// until there are no more results. Returns the total number of items processed.
async fn run_batch_cycle(deps: &JobDependencies, retry_cache: &mut RetryBackoff) -> anyhow::Result<u64> {
  let mut maybe_cursor: Option<i64> = None;
  let mut total_processed: u64 = 0;

  loop {
    if deps.application_shutdown.get() {
      break;
    }

    let query_started_at = Instant::now();
    let result = list_video_media_files_without_thumbnails_for_job(
      ListVideoMediaFilesWithoutThumbnailsArgs {
        custom_max_lookback_hours: deps.custom_max_lookback_hours,
        custom_page_size: deps.custom_page_size,
        maybe_id_cursor: maybe_cursor,
        executor: &deps.mysql_pool,
      },
    ).await?;
    let query_finished_at = Instant::now();

    info!(
      "thumbnail_page fetched_count={} database_query_ms={}",
      result.media_files.len(),
      query_finished_at.duration_since(query_started_at).as_millis(),
    );

    if result.media_files.is_empty() {
      break;
    }

    // Advance using the database page, even when every row is in backoff or
    // belongs to another shard. Otherwise failed files could hide older good files.
    maybe_cursor = result.next_cursor;
    let media_files = result.media_files.into_iter().filter(|media_file| {
      deps.shard_info.as_ref().is_none_or(|shard| {
        (media_file.id as u64) % u64::from(shard.number_of_shards) == u64::from(shard.shard_index)
      })
    }).collect();

    total_processed += process_page(media_files, deps.worker_config.concurrency, retry_cache, |media_file| async move {
      // Already-running attempts are drained on shutdown; no new attempts start.
      if deps.application_shutdown.get() {
        return None;
      }
      let timing = ThumbnailTiming::start(&media_file, query_started_at, query_finished_at);
      let result = process_single_media_file(deps, &media_file, &timing).await;
      timing.finish(result.is_ok());
      if result.is_ok() {
        let _ = deps.job_stats.increment_success_count();
      } else {
        let _ = deps.job_stats.increment_failure_count();
      }
      Some(result)
    }).await;

    if maybe_cursor.is_none() {
      break;
    }

    // Throttle between pages.
    tokio::time::sleep(Duration::from_millis(deps.query_delay_millis)).await;
  }

  Ok(total_processed)
}

/// Only keep `concurrency` attempts in flight, including their download, FFmpeg,
/// and upload work. Complete one before admitting another; never spawn a page of
/// tasks that merely wait for a semaphore while retaining downloaded videos.
async fn process_page<F, Fut>(
  mut media_files: Vec<VideoMediaFileWithoutThumbnail>,
  concurrency: NonZeroUsize,
  retry_cache: &mut RetryBackoff,
  process: F,
) -> u64
where
  F: Fn(VideoMediaFileWithoutThumbnail) -> Fut,
  Fut: Future<Output = Option<anyhow::Result<()>>>,
{
  let fetched_count = media_files.len();
  let now = Instant::now();
  media_files.retain(|file| retry_cache.is_ready(file.id, now));
  // Keep database ordering within each group, but let untried files precede retries.
  media_files.sort_by_key(|file| retry_cache.has_failed(file.id));
  info!(
    "thumbnail_dispatch eligible_count={} backoff_skipped_count={} concurrency={}",
    media_files.len(), fetched_count - media_files.len(), concurrency,
  );

  let mut pending = stream::iter(media_files).map(|file| {
    let id = file.id;
    let token = file.token.clone();
    let future = process(file);
    async move { (id, token, future.await) }
  }).buffer_unordered(concurrency.get());
  let mut completed = 0;
  while let Some((id, token, maybe_result)) = pending.next().await {
    match maybe_result {
      Some(Ok(())) => {
        retry_cache.record_success(id);
        completed += 1;
      }
      Some(Err(err)) => {
        let retry = retry_cache.record_failure(id, Instant::now());
        warn!(
          "thumbnail_retry media_file_token={} failed_attempts={} retry_delay_ms={} error={:?}",
          token.as_str(), retry.attempts, retry.delay.as_millis(), err,
        );
      }
      None => {}, // Shutdown: this file was not attempted.
    }
  }
  completed
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::sync::Mutex;

  use chrono::DateTime;
  use serde_json::json;

  use super::*;

  mod dispatch {
    use super::*;

    #[tokio::test]
    async fn fills_available_slots_without_exceeding_configured_concurrency() {
      for concurrency in 1..=4 {
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let mut cache = retry_cache();
        let completed = process_page((1..=20).map(media_file).collect(), limit(concurrency), &mut cache, |_| async {
          let count = active.fetch_add(1, Ordering::SeqCst) + 1;
          peak.fetch_max(count, Ordering::SeqCst);
          tokio::task::yield_now().await;
          active.fetch_sub(1, Ordering::SeqCst);
          Some(Ok(()))
        }).await;
        assert_eq!(completed, 20);
        assert_eq!(peak.load(Ordering::SeqCst), concurrency);
        assert_eq!(active.load(Ordering::SeqCst), 0);
      }
    }

    #[tokio::test]
    async fn cached_failure_page_does_not_prevent_processing_a_later_good_page() {
      let mut cache = retry_cache();
      for id in 1..=100 {
        cache.record_failure(id, Instant::now());
      }
      let completed = process_page((1..=100).map(media_file).collect(), limit(2), &mut cache, |_| async {
        panic!("A file in backoff must not run");
      }).await;
      assert_eq!(completed, 0);
      let completed = process_page(vec![media_file(101)], limit(2), &mut cache, |_| async { Some(Ok(())) }).await;
      assert_eq!(completed, 1);
    }

    #[tokio::test]
    async fn untried_files_precede_due_retries_and_failure_does_not_stop_progress() {
      let mut cache = retry_cache();
      cache.record_failure(1, Instant::now() - Duration::from_secs(60));
      let order = Mutex::new(Vec::new());
      let completed = process_page(vec![media_file(1), media_file(2)], limit(1), &mut cache, |file| {
        let order = &order;
        async move {
          order.lock().unwrap().push(file.id);
          Some(if file.id == 1 { Err(anyhow::anyhow!("invalid video")) } else { Ok(()) })
        }
      }).await;
      assert_eq!(completed, 1);
      assert_eq!(*order.lock().unwrap(), vec![2, 1]);
      assert!(!cache.is_ready(1, Instant::now()));
      assert!(!cache.has_failed(2));
    }
  }

  fn retry_cache() -> RetryBackoff {
    RetryBackoff::new(limit(1000), Duration::from_secs(30), Duration::from_secs(1800))
  }

  fn limit(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
  }

  fn media_file(id: i64) -> VideoMediaFileWithoutThumbnail {
    let now = DateTime::from_timestamp(1_000_000, 0).unwrap();
    VideoMediaFileWithoutThumbnail {
      id,
      token: serde_json::from_value(json!(format!("m_test_{id}"))).unwrap(),
      created_at: now,
      database_read_at: now,
      maybe_generation_completed_at: Some(now),
      maybe_thumbnail_version: None,
      public_bucket_directory_hash: String::new(),
      maybe_public_bucket_prefix: None,
      maybe_public_bucket_extension: None,
    }
  }
}
