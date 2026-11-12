use actix_web::web::Json;
use fal_client::webhook_payload::hydrate_webhook_contents::hydrate_webhook_contents;
use fal_client::webhook_payload::hydrated::hydrated_webhook_contents::HydratedWebhookContents;
use fal_client::webhook_payload::raw::raw_webhook_payload::RawWebhookPayload;
use http_server_common::response::response_success_helpers::SimpleGenericJsonSuccess;
use log::{info, warn};
use mysql_queries::queries::named_locks::release_named_lock::{release_named_lock, ReleaseNamedLockArgs};
use mysql_queries::queries::named_locks::try_get_named_lock::{try_get_named_lock, TryGetNamedLockArgs};
use sha2::{Digest, Sha256};
use sqlx::pool::PoolConnection;
use sqlx::MySql;

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::webhooks::fal::process_failure::handle_failed_fal_webhook::handle_failed_fal_webhook;
use crate::http_server::endpoints::webhooks::fal::process_success::handle_successful_fal_webhook::handle_successful_fal_webhook;
use crate::state::server_state::ServerState;

/// Apply a FAL result to its job: store the outputs and complete the job, or mark it failed.
///
/// This is the one path for FAL results, whether they arrive as a webhook or are fetched by the
/// lost-webhook recovery thread. `raw_body` is kept in the job's debug log.
///
/// Results for the same request are applied one at a time. FAL redelivers a webhook it thinks
/// failed (even while the first delivery is still being processed, if that takes a while), and
/// the recovery thread may fetch a result whose webhook is arriving at the same moment. A caller
/// that finds the request already being processed gets `TooManyRequests` (FAL retries it later);
/// once the first one has finished, the handlers' checks for finished jobs make repeats no-ops.
pub(crate) async fn process_fal_webhook_payload(
  server_state: &ServerState,
  webhook_payload: &RawWebhookPayload,
  raw_body: &str,
) -> Result<Json<SimpleGenericJsonSuccess>, CommonWebError> {
  let request_id = webhook_payload.request_id.as_str();

  let mut mysql_connection = server_state.mysql_pool.acquire().await?;

  let lock_name = result_lock_name(request_id);

  let locked = try_get_named_lock(TryGetNamedLockArgs {
    name: &lock_name,
    mysql_executor: &mut *mysql_connection,
  }).await.map_err(CommonWebError::from_error)?;

  if !locked {
    info!("The result of FAL request {} is already being applied; asking to retry later", request_id);
    return Err(CommonWebError::TooManyRequests);
  }

  let result = apply_result(server_state, &mut mysql_connection, webhook_payload, raw_body).await;

  let released = release_named_lock(ReleaseNamedLockArgs {
    name: &lock_name,
    mysql_executor: &mut *mysql_connection,
  }).await;

  if let Err(err) = released {
    // The lock belongs to the connection, so close it rather than return it to the pool still
    // holding the lock.
    warn!("Could not release {} ({:?}); closing its connection", lock_name, err);
    let _ = mysql_connection.close().await;
  }

  result
}

async fn apply_result(
  server_state: &ServerState,
  mysql_connection: &mut PoolConnection<MySql>,
  webhook_payload: &RawWebhookPayload,
  raw_body: &str,
) -> Result<Json<SimpleGenericJsonSuccess>, CommonWebError> {
  let request_id = webhook_payload.request_id.as_str();

  match hydrate_webhook_contents(webhook_payload) {
    HydratedWebhookContents::Success(success_data) => {
      handle_successful_fal_webhook(server_state, mysql_connection, request_id, &success_data, raw_body, &server_state.pager).await
    }
    HydratedWebhookContents::Error(error_data) => {
      handle_failed_fal_webhook(
        server_state,
        mysql_connection,
        request_id,
        &error_data,
        webhook_payload.error.as_deref(),
        raw_body,
      ).await
    }
    HydratedWebhookContents::PayloadError(payload_error_data) => {
      warn!(
        "FAL webhook payload_error for request_id {}: {}",
        request_id, payload_error_data.payload_error,
      );
      Err(CommonWebError::from_anyhow_error(
        anyhow::anyhow!("FAL payload_error: {}", payload_error_data.payload_error)
      ))
    }
  }
}

/// The named lock for applying one request's result. Hashed, so any request ID fits MySQL's
/// 64-character limit for lock names.
pub(crate) fn result_lock_name(request_id: &str) -> String {
  let digest: String = Sha256::digest(request_id.as_bytes())
    .iter()
    .take(20)
    .map(|byte| format!("{:02x}", byte))
    .collect();
  format!("fal_result:{}", digest)
}
