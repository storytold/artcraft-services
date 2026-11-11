//! `GET /v1/videos/{id}/content` — the video download endpoint, used here to
//! probe WHY a task failed.
//!
//! The regular poll endpoint ([`crate::requests::get_video_task`]) reports
//! failed tasks as bare `status: "failed"` with no reason. The content
//! endpoint's error message, however, leaks the underlying provider status,
//! eg.:
//!
//!   {"error":{"message":"Task is not completed yet, current status: VIOLATION", ...}}
//!
//! "VIOLATION" indicates a content-moderation rejection (these tasks run to
//! `progress: 100` and then fail when the finished output is moderated).

use log::info;

use crate::creds::comet_api_key::CometApiKey;
use crate::error::comet_client_error::CometClientError;
use crate::error::comet_error::CometError;
use crate::error::comet_generic_api_error::CometGenericApiError;
use crate::requests::comet_host::COMET_API_BASE_URL;
use crate::requests::get_video_content::request_types::VideoContentErrorRawResponse;

/// Marker preceding the underlying provider status in the content endpoint's
/// error message.
const CURRENT_STATUS_MARKER: &str = "current status: ";

// ── Public args ──

pub struct ProbeVideoFailureReasonArgs<'a> {
  pub api_key: &'a CometApiKey,

  /// The task id returned by the creation endpoint, eg. "task_abc123".
  pub task_id: &'a str,
}

// ── Public response ──

/// Why a video task isn't downloadable, recovered from the content
/// endpoint's error message.
///
/// NB: The error envelope carries exactly two fields (`message`, `type`) —
/// there is no richer structured reason. The request id comes from the
/// `x-cometapi-request-id` response header and is useful when escalating
/// to CometAPI support.
#[derive(Clone, Debug)]
pub struct CometVideoFailureReason {
  /// The full error message, eg.
  /// "Task is not completed yet, current status: VIOLATION".
  pub raw_message: String,

  /// The underlying provider status parsed out of the message, eg.
  /// "VIOLATION" (content moderation rejection). `None` if the message
  /// didn't follow the known "current status: X" shape.
  pub maybe_underlying_status: Option<String>,

  /// The error envelope's `type` field, eg. "invalid_request_error".
  pub maybe_error_type: Option<String>,

  /// CometAPI's request id (`x-cometapi-request-id` response header), for
  /// support escalation.
  pub maybe_request_id: Option<String>,
}

impl CometVideoFailureReason {
  /// Whether the task was rejected by content moderation.
  pub fn is_content_violation(&self) -> bool {
    self.maybe_underlying_status.as_deref() == Some("VIOLATION")
  }
}

/// Probe why a task's video isn't downloadable.
///
/// Returns `Ok(None)` when the content endpoint responds 2xx — the video
/// exists and there is no failure (the body is NOT downloaded). Returns
/// `Ok(Some(reason))` when the endpoint returns its error envelope, which is
/// where the underlying provider status (eg. "VIOLATION") is surfaced.
///
/// NB: For tasks that are merely still running, the message reports the
/// in-flight status the same way — call this for tasks the poll endpoint
/// already reported as failed.
pub async fn probe_video_failure_reason(
  args: ProbeVideoFailureReasonArgs<'_>,
) -> Result<Option<CometVideoFailureReason>, CometError> {
  let url = format!("{COMET_API_BASE_URL}/v1/videos/{}/content", args.task_id);

  let client = reqwest::Client::builder()
    .build()
    .map_err(CometClientError::ReqwestClientError)?;

  let response = client.get(&url)
    .bearer_auth(args.api_key.as_str())
    .send()
    .await
    .map_err(CometGenericApiError::ReqwestError)?;

  let status_code = response.status();

  if status_code.is_success() {
    // The video exists; don't download the body.
    return Ok(None);
  }

  let maybe_request_id = response.headers()
    .get("x-cometapi-request-id")
    .and_then(|value| value.to_str().ok())
    .map(|value| value.to_string());

  let body = response.text()
    .await
    .map_err(CometGenericApiError::ReqwestError)?;

  let raw: VideoContentErrorRawResponse = serde_json::from_str(&body)
    .map_err(|err| CometGenericApiError::SerdeResponseParseErrorWithBody(err, body.clone()))?;

  let mut reason = failure_reason_from_message(raw.error.message);
  reason.maybe_error_type = raw.error.error_type;
  reason.maybe_request_id = maybe_request_id;

  info!("Comet video task {:?} failure probe: {:?}", args.task_id, reason);

  Ok(Some(reason))
}

fn failure_reason_from_message(raw_message: String) -> CometVideoFailureReason {
  let maybe_underlying_status = raw_message
    .split(CURRENT_STATUS_MARKER)
    .nth(1)
    .map(|status| status.trim().trim_end_matches('.').to_string())
    .filter(|status| !status.is_empty());

  CometVideoFailureReason {
    raw_message,
    maybe_underlying_status,
    maybe_error_type: None,
    maybe_request_id: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extracts_violation_status_from_real_message() {
    let reason = failure_reason_from_message(
      "Task is not completed yet, current status: VIOLATION".to_string());

    assert_eq!(reason.maybe_underlying_status.as_deref(), Some("VIOLATION"));
    assert!(reason.is_content_violation());
  }

  #[test]
  fn extracts_other_statuses() {
    let reason = failure_reason_from_message(
      "Task is not completed yet, current status: IN_PROGRESS".to_string());

    assert_eq!(reason.maybe_underlying_status.as_deref(), Some("IN_PROGRESS"));
    assert!(!reason.is_content_violation());
  }

  #[test]
  fn unknown_message_shape_keeps_raw_message() {
    let reason = failure_reason_from_message("something unexpected".to_string());

    assert_eq!(reason.maybe_underlying_status, None);
    assert!(!reason.is_content_violation());
    assert_eq!(reason.raw_message, "something unexpected");
  }
}
