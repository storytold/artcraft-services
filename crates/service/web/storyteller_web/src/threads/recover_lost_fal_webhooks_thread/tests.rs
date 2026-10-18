//! Tests of lost-webhook recovery: the queries that find and claim FAL requests, and recovery
//! passes against a stand-in for FAL, applying results through the real webhook processing.
//!
//! These are DATABASE TESTS (see `mysql_testing`). Skip them on machines without MySQL with
//! `--features skip_database_tests`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use enums::common::job_status_plus::JobStatusPlus;
use enums::common::visibility::Visibility;
use fal_client::webhook_payload::raw::raw_webhook_payload::{RawWebhookPayload, RawWebhookStatus};
use mysql_queries::queries::fal_queue_requests::claim_fal_queue_request_for_recovery::{claim_fal_queue_request_for_recovery, ClaimFalQueueRequestForRecoveryArgs};
use mysql_queries::queries::fal_queue_requests::insert_fal_queue_request::{insert_fal_queue_request, InsertFalQueueRequestArgs};
use mysql_queries::queries::fal_queue_requests::list_fal_queue_requests_awaiting_recovery::{list_fal_queue_requests_awaiting_recovery, ListFalQueueRequestsAwaitingRecoveryArgs};
use mysql_queries::queries::generic_inference::api_providers::fal::insert_generic_inference_job_for_fal_queue::FalCategory;
use mysql_queries::queries::generic_inference::api_providers::fal::insert_generic_inference_job_for_fal_queue_with_apriori_job_token::{insert_generic_inference_job_for_fal_queue_with_apriori_job_token, InsertGenericInferenceForFalWithAprioriJobTokenArgs};
use rand::Rng;
use serde_json::json;
use sqlx::MySqlPool;
use tokens::tokens::generic_inference_jobs::InferenceJobToken;

use crate::http_server::endpoints::omni_gen::generate::video::tests::support::build_test_server_state;
use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::webhooks::fal::process_fal_webhook_payload::{process_fal_webhook_payload, result_lock_name};
use crate::state::server_state::ServerState;
use crate::threads::recover_lost_fal_webhooks_thread::config::FalWebhookRecoveryConfig;
use crate::threads::recover_lost_fal_webhooks_thread::fal_queue_poller::{FalQueuePoller, FalQueueResult, FalQueueStatus};
use crate::threads::recover_lost_fal_webhooks_thread::recover_lost_fal_webhooks::run_recovery_pass;

/// Jobs in these tests are "submitted" this long ago, past `RECOVER_AFTER`.
const OVERDUE_MINUTES: u32 = 20;
const RECOVER_AFTER: Duration = Duration::from_secs(15 * 60);
const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const CLAIM_LEASE: Duration = Duration::from_secs(300);

mod finding_and_claiming {
  use super::*;

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn an_overdue_pending_job_is_found() {
    let pool = pool().await;
    let request = submit_fal_job(&pool, OVERDUE_MINUTES).await;

    assert!(is_awaiting_recovery(&pool, &request).await);
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_job_still_within_its_webhook_window_is_not_found() {
    let pool = pool().await;
    let request = submit_fal_job(&pool, 5).await;

    assert!(!is_awaiting_recovery(&pool, &request).await);
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_job_older_than_the_max_age_is_not_found() {
    let pool = pool().await;
    let request = submit_fal_job(&pool, 25 * 60).await;

    assert!(!is_awaiting_recovery(&pool, &request).await);
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_finished_job_is_not_found() {
    let pool = pool().await;

    for status in [JobStatusPlus::CompleteSuccess, JobStatusPlus::CompleteFailure, JobStatusPlus::CancelledByUser] {
      let request = submit_fal_job(&pool, OVERDUE_MINUTES).await;
      set_job_status(&pool, &request, status).await;

      assert!(!is_awaiting_recovery(&pool, &request).await, "{:?}", status);
    }
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn only_one_server_claims_a_request_within_the_lease() {
    let pool = pool().await;
    let request = submit_fal_job(&pool, OVERDUE_MINUTES).await;

    assert!(claim(&pool, &request).await);
    assert!(!claim(&pool, &request).await);
    assert!(!is_awaiting_recovery(&pool, &request).await);
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_request_can_be_claimed_again_once_the_lease_expires() {
    let pool = pool().await;
    let request = submit_fal_job(&pool, OVERDUE_MINUTES).await;

    assert!(claim(&pool, &request).await);
    expire_claim(&pool, &request).await;

    assert!(is_awaiting_recovery(&pool, &request).await);
    assert!(claim(&pool, &request).await);
    assert_eq!(attempt_count(&pool, &request).await, 2);
  }
}

mod recovery_passes {
  use super::*;

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_failed_request_marks_its_job_failed() {
    let harness = Harness::new().await;
    let request = submit_fal_job(&harness.pool, 110).await;
    harness.fal.finish_with(&request, FalQueueResult::Failed {
      http_status: 422,
      raw_body: json!({"detail": [{"msg": "Prompt was rejected", "type": "content_policy_violation"}]}).to_string(),
    });

    harness.recover_jobs_aged(110).await;

    assert_eq!(job_status(&harness.pool, &request).await, JobStatusPlus::CompleteFailure);
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_request_fal_is_still_running_is_left_for_later() {
    let harness = Harness::new().await;
    let request = submit_fal_job(&harness.pool, 130).await;

    harness.recover_jobs_aged(130).await;

    assert_eq!(job_status(&harness.pool, &request).await, JobStatusPlus::Pending);
    assert_eq!(attempt_count(&harness.pool, &request).await, 1);
    assert_eq!(harness.fal.result_fetches(&request), 0);
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_request_fal_cant_be_asked_about_is_left_for_later() {
    let harness = Harness::new().await;
    let request = submit_fal_job(&harness.pool, 150).await;
    harness.fal.finish_with(&request, FalQueueResult::Unavailable("HTTP 503".to_string()));

    harness.recover_jobs_aged(150).await;

    assert_eq!(job_status(&harness.pool, &request).await, JobStatusPlus::Pending);
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_claimed_request_is_not_polled_again_within_the_lease() {
    let harness = Harness::new().await;
    let request = submit_fal_job(&harness.pool, 170).await;

    harness.recover_jobs_aged(170).await;
    harness.recover_jobs_aged(170).await;

    assert_eq!(harness.fal.status_checks(&request), 1);
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_successful_result_goes_through_the_webhook_processing() {
    // An output with nothing the processing recognises is the one success that stores no files,
    // so it can run here without a media CDN; the job's outputs are what the webhook path tests.
    let harness = Harness::new().await;
    let request = submit_fal_job(&harness.pool, 190).await;
    harness.fal.finish_with(&request, FalQueueResult::Output { payload: json!({"seed": 1}), raw_body: r#"{"seed":1}"#.to_string() });

    harness.recover_jobs_aged(190).await;

    assert_eq!(harness.fal.result_fetches(&request), 1);
  }
}

mod repeated_results {
  use super::*;

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_result_already_being_applied_is_answered_with_retry_later() {
    let harness = Harness::new().await;
    let request = submit_fal_job(&harness.pool, OVERDUE_MINUTES).await;
    let holder = hold_result_lock(&harness.pool, &request).await;

    let result = process_fal_webhook_payload(&harness.server_state, &error_webhook(&request), "{}").await;

    assert!(matches!(result, Err(CommonWebError::TooManyRequests)), "{:?}", result.as_ref().err());
    assert_eq!(job_status(&harness.pool, &request).await, JobStatusPlus::Pending);

    release_result_lock(holder, &request).await;
    process_fal_webhook_payload(&harness.server_state, &error_webhook(&request), "{}").await.unwrap();

    assert_eq!(job_status(&harness.pool, &request).await, JobStatusPlus::CompleteFailure);
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn recovery_leaves_a_result_that_is_being_applied_for_later() {
    let harness = Harness::new().await;
    let request = submit_fal_job(&harness.pool, 210).await;
    harness.fal.finish_with(&request, FalQueueResult::Failed { http_status: 500, raw_body: "{}".to_string() });
    let holder = hold_result_lock(&harness.pool, &request).await;

    harness.recover_jobs_aged(210).await;

    assert_eq!(job_status(&harness.pool, &request).await, JobStatusPlus::Pending);
    release_result_lock(holder, &request).await;
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_failure_report_does_not_undo_a_success() {
    let harness = Harness::new().await;
    let request = submit_fal_job(&harness.pool, OVERDUE_MINUTES).await;
    set_job_status(&harness.pool, &request, JobStatusPlus::CompleteSuccess).await;

    let result = process_fal_webhook_payload(&harness.server_state, &error_webhook(&request), "{}").await;

    assert!(result.is_ok());
    assert_eq!(job_status(&harness.pool, &request).await, JobStatusPlus::CompleteSuccess);
  }

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_repeated_failure_report_is_ignored() {
    let harness = Harness::new().await;
    let request = submit_fal_job(&harness.pool, OVERDUE_MINUTES).await;

    process_fal_webhook_payload(&harness.server_state, &error_webhook(&request), "{}").await.unwrap();
    let first_reason = failure_reason(&harness.pool, &request).await;
    let mut second = error_webhook(&request);
    second.error = Some("Invalid status code: 500".to_string());
    process_fal_webhook_payload(&harness.server_state, &second, "{}").await.unwrap();

    assert_eq!(failure_reason(&harness.pool, &request).await, first_reason);
  }
}

/// A submitted FAL request: its request ID and the row recording its queue URLs.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Request {
  fal_request_id: String,
  status_url: String,
}

struct Harness {
  pool: MySqlPool,
  server_state: ServerState,
  fal: Arc<FakeFal>,
}

impl Harness {
  async fn new() -> Self {
    let pool = pool().await;
    let server_state = build_test_server_state(pool.clone(), "http://localhost".to_string());
    Self { pool, server_state, fal: Arc::new(FakeFal::default()) }
  }

  /// One recovery pass over the jobs submitted about `minutes` ago.
  ///
  /// Tests run in parallel on one database, and a pass claims every request it finds. Each test
  /// that runs passes submits its job at its own age and only looks at that age, so it never
  /// claims (and leaves stuck) another test's request.
  async fn recover_jobs_aged(&self, minutes: u64) {
    let config = FalWebhookRecoveryConfig {
      recover_after: Duration::from_secs((minutes - 2) * 60),
      max_age: Duration::from_secs((minutes + 2) * 60),
      ..config()
    };
    run_recovery_pass(&self.server_state, self.fal.as_ref(), &config).await.expect("recovery pass");
  }
}

/// Stands in for FAL's queue: every request is still running unless the test finished it.
#[derive(Default)]
struct FakeFal {
  results: Mutex<HashMap<String, FalQueueResult>>,
  status_checks: Mutex<HashMap<String, usize>>,
  result_fetches: Mutex<HashMap<String, usize>>,
}

impl FakeFal {
  fn finish_with(&self, request: &Request, result: FalQueueResult) {
    self.results.lock().unwrap().insert(request.status_url.clone(), result);
  }

  fn status_checks(&self, request: &Request) -> usize {
    *self.status_checks.lock().unwrap().get(&request.status_url).unwrap_or(&0)
  }

  fn result_fetches(&self, request: &Request) -> usize {
    *self.result_fetches.lock().unwrap().get(&response_url_for(&request.status_url)).unwrap_or(&0)
  }
}

#[async_trait]
impl FalQueuePoller for FakeFal {
  async fn status(&self, status_url: &str) -> Result<FalQueueStatus, String> {
    *self.status_checks.lock().unwrap().entry(status_url.to_string()).or_default() += 1;
    Ok(if self.results.lock().unwrap().contains_key(status_url) { FalQueueStatus::Done } else { FalQueueStatus::NotDone })
  }

  async fn result(&self, response_url: &str) -> FalQueueResult {
    *self.result_fetches.lock().unwrap().entry(response_url.to_string()).or_default() += 1;
    let status_url = format!("{}/status", response_url);
    self.results.lock().unwrap().get(&status_url).cloned().unwrap_or(FalQueueResult::NotDone)
  }
}

fn config() -> FalWebhookRecoveryConfig {
  FalWebhookRecoveryConfig {
    enabled: true,
    recover_after: RECOVER_AFTER,
    interval: Duration::from_secs(60),
    max_age: MAX_AGE,
    claim_lease: CLAIM_LEASE,
    batch_size: 1000,
  }
}

async fn pool() -> MySqlPool {
  let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
  mysql_testing::pool::create_test_pool().await
}

/// Insert a FAL job and its queue URLs the way the generation endpoints do, as if submitted
/// `minutes_ago`.
async fn submit_fal_job(pool: &MySqlPool, minutes_ago: u32) -> Request {
  let fal_request_id = format!("test-{:032x}", rand::rng().random::<u128>());
  let status_url = format!("https://queue.fal.run/fal-ai/test-model/requests/{fal_request_id}/status");
  let job_token = InferenceJobToken::generate();
  let idempotency_token = format!("{:032x}", rand::rng().random::<u128>());

  let mut connection = pool.acquire().await.expect("connection");

  insert_generic_inference_job_for_fal_queue_with_apriori_job_token(InsertGenericInferenceForFalWithAprioriJobTokenArgs {
    apriori_job_token: &job_token,
    uuid_idempotency_token: &idempotency_token,
    maybe_external_third_party_id: &fal_request_id,
    fal_category: FalCategory::ImageGeneration,
    maybe_model_type: None,
    maybe_prompt_token: None,
    maybe_creator_user_token: None,
    maybe_avt_token: None,
    creator_ip_address: "127.0.0.1",
    creator_set_visibility: Visibility::Public,
    maybe_platform_type: None,
    maybe_cost_estimates: None,
    maybe_debug_log_event_token: None,
    mysql_executor: &mut *connection,
    maybe_inference_args: None,
    starting_job_status_override: None,
    maybe_frontend_failure_category: None,
    maybe_failure_reason: None,
    phantom: Default::default(),
  }).await.expect("insert job");

  insert_fal_queue_request(InsertFalQueueRequestArgs {
    fal_request_id: &fal_request_id,
    status_url: &status_url,
    response_url: &response_url_for(&status_url),
    mysql_executor: &mut *connection,
  }).await.expect("insert queue request");

  sqlx::query("UPDATE generic_inference_jobs SET created_at = NOW() - INTERVAL ? MINUTE WHERE maybe_external_third_party_id = ?")
    .bind(minutes_ago)
    .bind(&fal_request_id)
    .execute(&mut *connection)
    .await
    .expect("age the job");

  sqlx::query("UPDATE fal_queue_requests SET created_at = NOW() - INTERVAL ? MINUTE WHERE fal_request_id = ?")
    .bind(minutes_ago)
    .bind(&fal_request_id)
    .execute(&mut *connection)
    .await
    .expect("age the queue request");

  Request { fal_request_id, status_url }
}

/// Take the lock that applying `request`'s result takes, on a connection of its own, as if a
/// webhook for it were being processed right now.
async fn hold_result_lock(pool: &MySqlPool, request: &Request) -> sqlx::pool::PoolConnection<sqlx::MySql> {
  let mut connection = pool.acquire().await.expect("connection");
  let locked: Option<i64> = sqlx::query_scalar("SELECT GET_LOCK(?, 0)")
    .bind(result_lock_name(&request.fal_request_id))
    .fetch_one(&mut *connection)
    .await
    .expect("take the lock");
  assert_eq!(locked, Some(1));
  connection
}

async fn release_result_lock(mut connection: sqlx::pool::PoolConnection<sqlx::MySql>, request: &Request) {
  sqlx::query("SELECT RELEASE_LOCK(?)")
    .bind(result_lock_name(&request.fal_request_id))
    .execute(&mut *connection)
    .await
    .expect("release the lock");
}

fn response_url_for(status_url: &str) -> String {
  status_url.trim_end_matches("/status").to_string()
}

fn error_webhook(request: &Request) -> RawWebhookPayload {
  RawWebhookPayload {
    request_id: request.fal_request_id.clone(),
    gateway_request_id: request.fal_request_id.clone(),
    status: RawWebhookStatus::Error,
    error: Some("Invalid status code: 422".to_string()),
    payload: Some(json!({"detail": [{"msg": "Prompt was rejected", "type": "content_policy_violation"}]})),
    payload_error: None,
  }
}

async fn is_awaiting_recovery(pool: &MySqlPool, request: &Request) -> bool {
  let waiting = list_fal_queue_requests_awaiting_recovery(ListFalQueueRequestsAwaitingRecoveryArgs {
    min_age_seconds: RECOVER_AFTER.as_secs() as u32,
    max_age_seconds: MAX_AGE.as_secs() as u32,
    claim_lease_seconds: CLAIM_LEASE.as_secs() as u32,
    limit: 10_000,
    mysql_executor: pool,
  }).await.expect("list requests awaiting recovery");

  waiting.iter().any(|waiting| waiting.fal_request_id == request.fal_request_id)
}

async fn claim(pool: &MySqlPool, request: &Request) -> bool {
  claim_fal_queue_request_for_recovery(ClaimFalQueueRequestForRecoveryArgs {
    fal_queue_request_id: queue_request_id(pool, request).await,
    claim_lease_seconds: CLAIM_LEASE.as_secs() as u32,
    mysql_executor: pool,
  }).await.expect("claim")
}

async fn queue_request_id(pool: &MySqlPool, request: &Request) -> u64 {
  sqlx::query_scalar("SELECT id FROM fal_queue_requests WHERE fal_request_id = ?")
    .bind(&request.fal_request_id)
    .fetch_one(pool)
    .await
    .expect("queue request id")
}

async fn expire_claim(pool: &MySqlPool, request: &Request) {
  sqlx::query("UPDATE fal_queue_requests SET maybe_recovery_claimed_at = NOW() - INTERVAL 1 HOUR WHERE fal_request_id = ?")
    .bind(&request.fal_request_id)
    .execute(pool)
    .await
    .expect("expire the claim");
}

async fn attempt_count(pool: &MySqlPool, request: &Request) -> u16 {
  sqlx::query_scalar("SELECT recovery_attempt_count FROM fal_queue_requests WHERE fal_request_id = ?")
    .bind(&request.fal_request_id)
    .fetch_one(pool)
    .await
    .expect("attempt count")
}

async fn set_job_status(pool: &MySqlPool, request: &Request, status: JobStatusPlus) {
  sqlx::query("UPDATE generic_inference_jobs SET status = ? WHERE maybe_external_third_party_id = ?")
    .bind(status.to_str())
    .bind(&request.fal_request_id)
    .execute(pool)
    .await
    .expect("set job status");
}

async fn job_status(pool: &MySqlPool, request: &Request) -> JobStatusPlus {
  let status: String = sqlx::query_scalar("SELECT status FROM generic_inference_jobs WHERE maybe_external_third_party_id = ?")
    .bind(&request.fal_request_id)
    .fetch_one(pool)
    .await
    .expect("job status");
  JobStatusPlus::from_str(&status).expect("known status")
}

async fn failure_reason(pool: &MySqlPool, request: &Request) -> Option<String> {
  sqlx::query_scalar("SELECT internal_debugging_failure_reason FROM generic_inference_jobs WHERE maybe_external_third_party_id = ?")
    .bind(&request.fal_request_id)
    .fetch_one(pool)
    .await
    .expect("failure reason")
}
