use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookResponse {
  pub status: Option<String>,
  pub request_id: Option<String>,
  pub gateway_request_id: Option<String>,

  /// Where to poll the request's status in FAL's queue. Taken from FAL's response when it
  /// includes it, otherwise derived from the endpoint by `queue_webhook` (see `fal_queue_urls`).
  pub status_url: Option<String>,

  /// Where to fetch the request's result from FAL's queue, like `status_url`.
  pub response_url: Option<String>,
}
