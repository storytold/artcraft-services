# Video thumbnail timing

The worker emits `thumbnail_timing` followed by a JSON object at the start and
end of every attempt. Filter by `media_file_token` to correlate events.

- `event=started`: before downloading the source video.
- `event=completed`: JPG and GIF uploaded and the thumbnail version updated in
  the database, so the thumbnails are available to API readers.
- `event=failed`: an attempt ended unsuccessfully; exclude this from successful
  thumbnail latency percentiles.

`since_generation_completed_ms` measures time from
`generic_inference_jobs.successfully_completed_at` to the event. The source job
is found through `media_files.maybe_source_job_token`. This timestamp records
our backend finalizing the generation job after ingesting its output. It does
not measure when the external provider originally finished generating video,
nor is it an exact transaction commit timestamp.

For uploads, older unlinked media, or a job whose completion has not yet been
written when the page is fetched, `generation_completed_at` and
`since_generation_completed_ms` are JSON `null`. They are never replaced with
media creation time. `since_media_created_ms` is reported separately for all
files. `media_files.updated_at` is unsuitable because subsequent edits and the
thumbnail update change it.

Additional measurements:

- `processing_elapsed_ms`: monotonic time spent on this attempt; zero at start.
- `page_wait_ms`: time from fetching the page until this file starts, including
  waiting for earlier files in the same page.
- `database_query_ms`: total time spent fetching the page, including pool wait.
- `thumbnail_stage`: JSON events with `stage`, `success`, and `duration_ms` for
  download, JPG generation/upload, GIF generation/upload, and database update.
- `thumbnail_page`: fetched row count and query duration, including empty pages.

## Clock handling

The listing query returns `NOW(6)` as `database_read_at` with each row. Both that
timestamp and the generation completion timestamp come from the database.
Worker wall-clock time is not used to calculate latency.

MySQL evaluates `NOW(6)` at statement start. We bracket the query with two Rust
`Instant`s and use their midpoint as the estimated local instant corresponding
to `database_read_at`. Every event advances this database time using monotonic
elapsed time, including time queued behind other files in the page.

This estimate has up to `clock_uncertainty_ms` of error in either direction
(half the query duration, rounded up), plus the source timestamps' existing
one-second precision. For example, a 20 ms query contributes up to 10 ms of
clock-mapping uncertainty. A slow query reports a larger uncertainty; do not
interpret millisecond-valued fields as millisecond accuracy. Negative values
are preserved to expose timestamp anomalies. No extra per-file clock query is
needed.

These measurements end at backend thumbnail availability. Client polling,
caching, and UI refresh delays happen afterward and need separate measurement.
