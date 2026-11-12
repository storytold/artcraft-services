use std::sync::Arc;

use fal_client::webhook_payload::raw::raw_webhook_payload::{RawWebhookPayload, RawWebhookStatus};
use log::{error, info, warn};
use mysql_queries::queries::fal_queue_requests::claim_fal_queue_request_for_recovery::{claim_fal_queue_request_for_recovery, ClaimFalQueueRequestForRecoveryArgs};
use mysql_queries::queries::fal_queue_requests::fal_queue_request_awaiting_recovery::FalQueueRequestAwaitingRecovery;
use mysql_queries::queries::fal_queue_requests::list_fal_queue_requests_awaiting_recovery::{list_fal_queue_requests_awaiting_recovery, ListFalQueueRequestsAwaitingRecoveryArgs};

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::webhooks::fal::process_fal_webhook_payload::process_fal_webhook_payload;
use crate::state::server_state::ServerState;
use crate::threads::recover_lost_fal_webhooks_thread::config::FalWebhookRecoveryConfig;
use crate::threads::recover_lost_fal_webhooks_thread::fal_queue_poller::{FalQueuePoller, FalQueueResult, FalQueueStatus};

/// What one recovery pass did, for logs and tests.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RecoveryPassReport {
  /// Requests whose job was waiting longer than expected.
  pub found: usize,
  /// Of those, the ones this server claimed (the rest belong to another server for now).
  pub claimed: usize,
  /// Results applied to their job: outputs stored, or the job marked failed.
  pub recovered: usize,
  /// FAL is still working on them.
  pub still_running: usize,
  /// FAL couldn't be asked, or applying the result failed. Tried again after the claim lease.
  pub errors: usize,
}

/// Look for FAL jobs whose webhook should have arrived by now, and complete them from FAL's queue.
///
/// Runs forever, one pass every `config.interval`.
pub async fn recover_lost_fal_webhooks_thread(
  server_state: Arc<ServerState>,
  poller: Arc<dyn FalQueuePoller>,
  config: FalWebhookRecoveryConfig,
) {
  info!(
    "FAL webhook recovery: checking every {:?} for jobs still waiting {:?} after submission (up to {:?})",
    config.interval, config.recover_after, config.max_age,
  );

  loop {
    tokio::time::sleep(config.interval).await;

    match run_recovery_pass(&server_state, poller.as_ref(), &config).await {
      Ok(report) if report.found > 0 => info!("FAL webhook recovery pass: {:?}", report),
      Ok(_) => {}
      Err(err) => warn!("FAL webhook recovery pass failed: {:?}", err),
    }
  }
}

/// One pass: claim each waiting request, ask FAL where it is, and apply finished results exactly
/// as the webhook would have.
pub async fn run_recovery_pass(
  server_state: &ServerState,
  poller: &dyn FalQueuePoller,
  config: &FalWebhookRecoveryConfig,
) -> Result<RecoveryPassReport, sqlx::Error> {
  let claim_lease_seconds = config.claim_lease.as_secs() as u32;

  let requests = list_fal_queue_requests_awaiting_recovery(ListFalQueueRequestsAwaitingRecoveryArgs {
    min_age_seconds: config.recover_after.as_secs() as u32,
    max_age_seconds: config.max_age.as_secs() as u32,
    claim_lease_seconds,
    limit: config.batch_size,
    mysql_executor: &server_state.mysql_pool,
  }).await?;

  let mut report = RecoveryPassReport { found: requests.len(), ..Default::default() };

  for request in requests {
    let claimed = claim_fal_queue_request_for_recovery(ClaimFalQueueRequestForRecoveryArgs {
      fal_queue_request_id: request.id,
      claim_lease_seconds,
      mysql_executor: &server_state.mysql_pool,
    }).await?;

    if !claimed {
      continue;
    }

    report.claimed += 1;

    match recover_one(server_state, poller, &request).await {
      Outcome::Recovered => report.recovered += 1,
      Outcome::StillRunning => report.still_running += 1,
      Outcome::Error => report.errors += 1,
    }
  }

  Ok(report)
}

enum Outcome {
  Recovered,
  StillRunning,
  Error,
}

async fn recover_one(
  server_state: &ServerState,
  poller: &dyn FalQueuePoller,
  request: &FalQueueRequestAwaitingRecovery,
) -> Outcome {
  let request_id = request.fal_request_id.as_str();

  match poller.status(&request.status_url).await {
    Ok(FalQueueStatus::Done) => {}
    Ok(FalQueueStatus::NotDone) => return Outcome::StillRunning,
    Ok(FalQueueStatus::Unknown(status)) => {
      warn!("FAL webhook recovery: request {} has an unknown queue status {:?}", request_id, status);
      return Outcome::StillRunning;
    }
    Err(err) => {
      warn!("FAL webhook recovery: could not get the status of request {}: {}", request_id, err);
      return Outcome::Error;
    }
  }

  let (payload, raw_body) = match poller.result(&request.response_url).await {
    FalQueueResult::Output { payload, raw_body } => (succeeded(request_id, payload), raw_body),
    FalQueueResult::Failed { http_status, raw_body } => (failed(request_id, http_status, &raw_body), raw_body),
    FalQueueResult::NotDone => return Outcome::StillRunning,
    FalQueueResult::Unavailable(err) => {
      warn!("FAL webhook recovery: could not fetch the result of request {}: {}", request_id, err);
      return Outcome::Error;
    }
  };

  match process_fal_webhook_payload(server_state, &payload, &raw_body).await {
    Ok(_) => {
      info!(
        "FAL webhook recovery: applied the result of request {} (attempt {}); its webhook never arrived",
        request_id, request.recovery_attempt_count.saturating_add(1),
      );
      Outcome::Recovered
    }
    Err(CommonWebError::TooManyRequests) => {
      info!("FAL webhook recovery: the result of request {} is being applied already (its webhook arrived)", request_id);
      Outcome::StillRunning
    }
    Err(err) => {
      error!("FAL webhook recovery: could not apply the result of request {}: {:?}", request_id, err);
      Outcome::Error
    }
  }
}

/// The webhook FAL would have sent for a successful request.
fn succeeded(request_id: &str, output: serde_json::Value) -> RawWebhookPayload {
  RawWebhookPayload {
    request_id: request_id.to_string(),
    gateway_request_id: request_id.to_string(),
    status: RawWebhookStatus::Ok,
    error: None,
    payload: Some(output),
    payload_error: None,
  }
}

/// The webhook FAL would have sent for a failed request: an error naming the HTTP status, and
/// the error body (FAL's `detail`) as the payload.
fn failed(request_id: &str, http_status: u16, raw_body: &str) -> RawWebhookPayload {
  RawWebhookPayload {
    request_id: request_id.to_string(),
    gateway_request_id: request_id.to_string(),
    status: RawWebhookStatus::Error,
    error: Some(format!("Invalid status code: {}", http_status)),
    payload: serde_json::from_str(raw_body).ok(),
    payload_error: None,
  }
}

#[cfg(test)]
mod tests {
  use fal_client::webhook_payload::hydrate_webhook_contents::hydrate_webhook_contents;
  use fal_client::webhook_payload::hydrated::hydrated_webhook_contents::HydratedWebhookContents;
  use serde_json::json;

  use super::*;

  /// A real `fal-ai/flux/schnell` result, as fetched from its queue `response_url` on 2026-10-15
  /// (only the file URL shortened). Rebuilt into a webhook, it must be read as the same images.
  #[test]
  fn a_fetched_result_is_read_like_the_webhook_it_replaces() {
    let output = json!({
      "images": [{"url": "https://v3b.fal.media/files/b/0aae7d23/example.jpg", "width": 512, "height": 512, "content_type": "image/jpeg"}],
      "timings": {"inference": 0.07},
      "seed": 1234,
      "has_nsfw_concepts": [false],
      "prompt": "a small red circle on a white background"
    });

    let HydratedWebhookContents::Success(success) = hydrate_webhook_contents(&succeeded("req-1", output)) else {
      panic!("expected a success");
    };

    let images = success.extracted_contents.and_then(|contents| contents.images).expect("images");
    assert_eq!(images.len(), 1);
  }

  #[test]
  fn a_failed_result_is_read_as_an_error() {
    let body = json!({"detail": [{"msg": "Prompt was rejected", "type": "content_policy_violation"}]}).to_string();

    assert!(matches!(hydrate_webhook_contents(&failed("req-1", 422, &body)), HydratedWebhookContents::Error(_)));
  }
}
