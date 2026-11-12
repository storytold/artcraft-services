-- noinspection SqlDialectInspectionForFile
-- noinspection SqlNoDataSourceInspectionForFile
-- noinspection SqlResolveForFile

-- Materialized from 2026-10-15-120000-0000_create_fal_queue_requests.
-- See _database/sql/fal_queue_requests.md.
CREATE TABLE fal_queue_requests (
  id BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT,

  -- FAL's queue request ID. Matches the job's
  -- generic_inference_jobs.maybe_external_third_party_id (with maybe_external_third_party = 'fal').
  fal_request_id VARCHAR(64) NOT NULL,

  -- From FAL's queue submit response. Both point at queue.fal.run.
  status_url VARCHAR(512) NOT NULL,
  response_url VARCHAR(512) NOT NULL,

  -- Lost-webhook recovery: how many times the recovery thread has claimed this request
  -- to poll FAL. Application inserts explicitly bind 0.
  recovery_attempt_count SMALLINT UNSIGNED NOT NULL,

  -- When the recovery thread last claimed this request. NULL until the first claim. A
  -- claim is a lease: another server only claims the request again once it has expired.
  maybe_recovery_claimed_at TIMESTAMP NULL DEFAULT NULL,

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  PRIMARY KEY (id),
  UNIQUE KEY (fal_request_id),
  KEY index_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
