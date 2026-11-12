//! Tests that a FAL result is applied once: a result already being applied is answered with
//! "retry later", and repeated results don't change a finished job.
//!
//! These are DATABASE TESTS (see `mysql_testing`). Skip them on machines without MySQL with
//! `--features skip_database_tests`.

use enums::common::job_status_plus::JobStatusPlus;
use enums::common::visibility::Visibility;
use fal_client::webhook_payload::raw::raw_webhook_payload::{RawWebhookPayload, RawWebhookStatus};
use mysql_queries::queries::generic_inference::api_providers::fal::insert_generic_inference_job_for_fal_queue::FalCategory;
use mysql_queries::queries::generic_inference::api_providers::fal::insert_generic_inference_job_for_fal_queue_with_apriori_job_token::{insert_generic_inference_job_for_fal_queue_with_apriori_job_token, InsertGenericInferenceForFalWithAprioriJobTokenArgs};
use rand::Rng;
use serde_json::json;
use sqlx::MySqlPool;
use tokens::tokens::generic_inference_jobs::InferenceJobToken;

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::omni_gen::generate::video::tests::support::build_test_server_state;
use crate::http_server::endpoints::webhooks::fal::process_fal_webhook_payload::{process_fal_webhook_payload, result_lock_name};
use crate::state::server_state::ServerState;

#[tokio::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn a_result_already_being_applied_is_answered_with_retry_later() {
  let harness = Harness::new().await;
  let request_id = submit_fal_job(&harness.pool).await;
  let holder = hold_result_lock(&harness.pool, &request_id).await;

  let result = process_fal_webhook_payload(&harness.server_state, &error_webhook(&request_id), "{}").await;

  assert!(matches!(result, Err(CommonWebError::TooManyRequests)), "{:?}", result.as_ref().err());
  assert_eq!(job_status(&harness.pool, &request_id).await, JobStatusPlus::Pending);

  release_result_lock(holder, &request_id).await;
  process_fal_webhook_payload(&harness.server_state, &error_webhook(&request_id), "{}").await.unwrap();

  assert_eq!(job_status(&harness.pool, &request_id).await, JobStatusPlus::CompleteFailure);
}

#[tokio::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn a_failure_report_does_not_undo_a_success() {
  let harness = Harness::new().await;
  let request_id = submit_fal_job(&harness.pool).await;
  set_job_status(&harness.pool, &request_id, JobStatusPlus::CompleteSuccess).await;

  let result = process_fal_webhook_payload(&harness.server_state, &error_webhook(&request_id), "{}").await;

  assert!(result.is_ok());
  assert_eq!(job_status(&harness.pool, &request_id).await, JobStatusPlus::CompleteSuccess);
}

#[tokio::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn a_repeated_failure_report_is_ignored() {
  let harness = Harness::new().await;
  let request_id = submit_fal_job(&harness.pool).await;

  process_fal_webhook_payload(&harness.server_state, &error_webhook(&request_id), "{}").await.unwrap();
  let first_reason = failure_reason(&harness.pool, &request_id).await;
  let mut second = error_webhook(&request_id);
  second.error = Some("Invalid status code: 500".to_string());
  process_fal_webhook_payload(&harness.server_state, &second, "{}").await.unwrap();

  assert_eq!(failure_reason(&harness.pool, &request_id).await, first_reason);
}

struct Harness {
  pool: MySqlPool,
  server_state: ServerState,
}

impl Harness {
  async fn new() -> Self {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let pool = mysql_testing::pool::create_test_pool().await;
    let server_state = build_test_server_state(pool.clone(), "http://localhost".to_string());
    Self { pool, server_state }
  }
}

/// Insert a pending FAL job the way the generation endpoints do. Returns its FAL request ID.
async fn submit_fal_job(pool: &MySqlPool) -> String {
  let fal_request_id = format!("test-{:032x}", rand::rng().random::<u128>());
  let job_token = InferenceJobToken::generate();
  let idempotency_token = format!("{:032x}", rand::rng().random::<u128>());

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
    mysql_executor: pool,
    maybe_inference_args: None,
    starting_job_status_override: None,
    maybe_frontend_failure_category: None,
    maybe_failure_reason: None,
    phantom: Default::default(),
  }).await.expect("insert job");

  fal_request_id
}

/// Take the lock that applying the request's result takes, on a connection of its own, as if a
/// webhook for it were being processed right now.
async fn hold_result_lock(pool: &MySqlPool, request_id: &str) -> sqlx::pool::PoolConnection<sqlx::MySql> {
  let mut connection = pool.acquire().await.expect("connection");
  let locked: Option<i64> = sqlx::query_scalar("SELECT GET_LOCK(?, 0)")
    .bind(result_lock_name(request_id))
    .fetch_one(&mut *connection)
    .await
    .expect("take the lock");
  assert_eq!(locked, Some(1));
  connection
}

async fn release_result_lock(mut connection: sqlx::pool::PoolConnection<sqlx::MySql>, request_id: &str) {
  sqlx::query("SELECT RELEASE_LOCK(?)")
    .bind(result_lock_name(request_id))
    .execute(&mut *connection)
    .await
    .expect("release the lock");
}

fn error_webhook(request_id: &str) -> RawWebhookPayload {
  RawWebhookPayload {
    request_id: request_id.to_string(),
    gateway_request_id: request_id.to_string(),
    status: RawWebhookStatus::Error,
    error: Some("Invalid status code: 422".to_string()),
    payload: Some(json!({"detail": [{"msg": "Prompt was rejected", "type": "content_policy_violation"}]})),
    payload_error: None,
  }
}

async fn set_job_status(pool: &MySqlPool, request_id: &str, status: JobStatusPlus) {
  sqlx::query("UPDATE generic_inference_jobs SET status = ? WHERE maybe_external_third_party_id = ?")
    .bind(status.to_str())
    .bind(request_id)
    .execute(pool)
    .await
    .expect("set job status");
}

async fn job_status(pool: &MySqlPool, request_id: &str) -> JobStatusPlus {
  let status: String = sqlx::query_scalar("SELECT status FROM generic_inference_jobs WHERE maybe_external_third_party_id = ?")
    .bind(request_id)
    .fetch_one(pool)
    .await
    .expect("job status");
  JobStatusPlus::from_str(&status).expect("known status")
}

async fn failure_reason(pool: &MySqlPool, request_id: &str) -> Option<String> {
  sqlx::query_scalar("SELECT internal_debugging_failure_reason FROM generic_inference_jobs WHERE maybe_external_third_party_id = ?")
    .bind(request_id)
    .fetch_one(pool)
    .await
    .expect("failure reason")
}
