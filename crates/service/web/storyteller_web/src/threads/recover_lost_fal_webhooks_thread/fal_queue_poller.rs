use async_trait::async_trait;
use fal_client::creds::fal_api_key::FalApiKey;
use fal_client::error::api_generic_error::FalGenericApiError;
use fal_client::error::api_specific_error::FalSpecificApiError;
use fal_client::error::fal_error_plus::FalErrorPlus;
use fal_client::polling::poll_job_response::poll_job_response::{poll_job_response, PollJobResponseArgs};
use fal_client::polling::poll_job_status::poll_job_status::{poll_job_status, FalJobStatus, PollJobStatusArgs};
use serde_json::Value;

/// Where a request is in FAL's queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FalQueueStatus {
  /// Queued or running.
  NotDone,
  /// Finished: the result (output or error) can be fetched.
  Done,
  /// A status this code doesn't know.
  Unknown(String),
}

/// A finished request's result, as fetched from its response URL.
#[derive(Debug, Clone, PartialEq)]
pub enum FalQueueResult {
  /// The model's output: what a successful webhook carries as `payload`.
  Output { payload: Value, raw_body: String },
  /// The request failed. FAL answers the response URL with the failure's HTTP status, the same
  /// one its error webhook reports as "Invalid status code: …".
  Failed { http_status: u16, raw_body: String },
  /// FAL says the request isn't finished after all.
  NotDone,
  /// FAL couldn't be asked right now (network trouble, rate limit, gateway error): try later.
  Unavailable(String),
}

/// Asks FAL about a queued request. A trait so that tests can stand in for FAL.
#[async_trait]
pub trait FalQueuePoller: Send + Sync {
  async fn status(&self, status_url: &str) -> Result<FalQueueStatus, String>;
  async fn result(&self, response_url: &str) -> FalQueueResult;
}

/// The real FAL queue API, through `fal_client`'s polling calls.
pub struct FalApiQueuePoller {
  pub api_key: FalApiKey,
}

#[async_trait]
impl FalQueuePoller for FalApiQueuePoller {
  async fn status(&self, status_url: &str) -> Result<FalQueueStatus, String> {
    let response = poll_job_status(PollJobStatusArgs { status_url, api_key: &self.api_key })
      .await
      .map_err(|err| err.to_string())?;

    Ok(match response.status {
      FalJobStatus::InQueue | FalJobStatus::InProgress => FalQueueStatus::NotDone,
      FalJobStatus::Completed => FalQueueStatus::Done,
      FalJobStatus::Unknown(status) => FalQueueStatus::Unknown(status),
    })
  }

  async fn result(&self, response_url: &str) -> FalQueueResult {
    match poll_job_response(PollJobResponseArgs { response_url, api_key: &self.api_key }).await {
      Ok(response) => FalQueueResult::Output { payload: response.payload, raw_body: response.raw_body },
      Err(FalErrorPlus::ApiSpecific(FalSpecificApiError::IncompleteJob(_))) => FalQueueResult::NotDone,
      Err(FalErrorPlus::ApiGeneric(FalGenericApiError::UncategorizedBadResponseWithStatusAndBody { status_code, body })) => {
        classify_failed_response(status_code.as_u16(), body)
      }
      Err(err) => FalQueueResult::Unavailable(err.to_string()),
    }
  }
}

/// A non-2xx answer from the response URL is the request's own failure, except for statuses that
/// mean FAL (or something in front of it) couldn't answer now.
fn classify_failed_response(http_status: u16, raw_body: String) -> FalQueueResult {
  match http_status {
    408 | 429 | 502 | 503 | 504 => FalQueueResult::Unavailable(format!("HTTP {}: {}", http_status, raw_body)),
    _ => FalQueueResult::Failed { http_status, raw_body },
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn request_failures_are_failures() {
    for status in [400, 401, 404, 422, 500] {
      assert!(matches!(classify_failed_response(status, "{}".to_string()), FalQueueResult::Failed { .. }), "{}", status);
    }
  }

  #[test]
  fn transient_answers_are_retried_later() {
    for status in [408, 429, 502, 503, 504] {
      assert!(matches!(classify_failed_response(status, "{}".to_string()), FalQueueResult::Unavailable(_)), "{}", status);
    }
  }
}
