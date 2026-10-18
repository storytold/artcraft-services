use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::HttpRequest;
use errors::anyhow;
use fal_client::webhook_signature::fal_webhook_signature_error::FalWebhookSignatureError;
use fal_client::webhook_signature::fal_webhook_signature_headers::FalWebhookSignatureHeaders;
use fal_client::webhook_signature::verify_fal_webhook_request::verify_fal_webhook_request;
use log::{error, warn};

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::webhooks::fal::signature::fal_webhook_signature_mode::{FalWebhookSignatureMode, FAL_WEBHOOK_SIGNATURE_MODE_ENV};
use crate::http_server::endpoints::webhooks::fal::signature::fal_webhook_signature_settings::FalWebhookSignatureSettings;

/// Check that a webhook request was sent by FAL, as the configured mode asks.
///
/// - `Enforce`: a request that doesn't verify is rejected here, before its body is parsed or
///   any job is looked up. A bad or missing signature is a 401; FAL's keys being unavailable is
///   a 500, so that FAL retries the delivery rather than having it refused.
/// - `Report`: a failure is logged and the request goes on to be processed.
/// - `Off`: nothing is checked.
pub async fn check_fal_webhook_signature(
  http_request: &HttpRequest,
  raw_body: &[u8],
  settings: &FalWebhookSignatureSettings,
) -> Result<(), CommonWebError> {
  check_fal_webhook_signature_at(http_request, raw_body, settings, unix_now()).await
}

async fn check_fal_webhook_signature_at(
  http_request: &HttpRequest,
  raw_body: &[u8],
  settings: &FalWebhookSignatureSettings,
  now_unix_seconds: i64,
) -> Result<(), CommonWebError> {
  if settings.mode == FalWebhookSignatureMode::Off {
    return Ok(());
  }

  let verification = match read_headers(http_request) {
    Ok(headers) => verify_fal_webhook_request(&settings.jwks_cache, &headers, raw_body, now_unix_seconds).await,
    Err(err) => Err(err),
  };

  let Err(err) = verification else {
    return Ok(());
  };

  match settings.mode {
    FalWebhookSignatureMode::Enforce if is_our_problem(&err) => {
      error!("Could not verify a FAL webhook because FAL's keys are unavailable: {}", err);
      Err(CommonWebError::from_anyhow_error(anyhow!("FAL webhook keys unavailable: {}", err)))
    }
    FalWebhookSignatureMode::Enforce => {
      warn!("Rejected a FAL webhook that failed its signature check: {}", err);
      Err(CommonWebError::NotAuthorized)
    }
    FalWebhookSignatureMode::Report => {
      warn!(
        "A FAL webhook failed its signature check ({}); processing it anyway because {}=report",
        err, FAL_WEBHOOK_SIGNATURE_MODE_ENV,
      );
      Ok(())
    }
    FalWebhookSignatureMode::Off => Ok(()),
  }
}

fn read_headers(http_request: &HttpRequest) -> Result<FalWebhookSignatureHeaders, FalWebhookSignatureError> {
  FalWebhookSignatureHeaders::from_lookup(|name| {
    http_request.headers().get(name).and_then(|value| value.to_str().ok())
  })
}

/// Failures that are about our copy of FAL's keys, not about the request.
fn is_our_problem(err: &FalWebhookSignatureError) -> bool {
  matches!(
    err,
    FalWebhookSignatureError::JwksFetchFailed(_)
      | FalWebhookSignatureError::JwksParseFailed(_)
      | FalWebhookSignatureError::NoPublicKeys
  )
}

fn unix_now() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|elapsed| elapsed.as_secs() as i64)
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use actix_web::http::StatusCode;
  use actix_web::test::TestRequest;
  use actix_web::ResponseError;
  use fal_client::webhook_signature::fal_jwks_cache::FalJwksCache;
  use fal_client::webhook_signature::fal_webhook_signature_error::FalWebhookSignatureError;
  use fal_client::webhook_signature::fal_webhook_signature_headers::{REQUEST_ID_HEADER, SIGNATURE_HEADER, TIMESTAMP_HEADER, USER_ID_HEADER};

  use super::*;

  // A webhook signed with PyNaCl by following FAL's documented algorithm (the same vector as
  // `fal_client::webhook_signature::verify_fal_webhook_signature`'s tests).
  const PUBLIC_KEY_JWKS: &str = r#"{"keys":[{"kty":"OKP","crv":"Ed25519","x":"A6EHv_POEL4dcN0Y50vAmWfk1jCbpQ1fHdyGZBJVMbg=","use":"sig"}]}"#;
  const REQUEST_ID: &str = "019e18d8-8c36-7bc1-aa77-2bc2f70268c6";
  const USER_ID: &str = "test-user-1";
  const TIMESTAMP: &str = "1760500000";
  const BODY: &[u8] = br#"{"request_id":"019e18d8-8c36-7bc1-aa77-2bc2f70268c6","status":"OK","payload":{"images":[]}}"#;
  const SIGNATURE: &str = "d925f70845ad2e5a410c3b671a443a27cf538bf1f20b9918b6ce583f0f3640c6f9b2a407a442fa3d770e914ea27f4d4bfc07fd64f3496672e0da6af9a930040e";
  const NOW: i64 = 1760500000;

  mod enforce {
    use super::*;

    #[tokio::test]
    async fn accepts_a_genuine_webhook() {
      let result = check(FalWebhookSignatureMode::Enforce, &signed_request(), BODY).await;

      assert!(result.is_ok());
    }

    #[tokio::test]
    async fn rejects_an_unsigned_webhook_with_401() {
      let result = check(FalWebhookSignatureMode::Enforce, &TestRequest::post().to_http_request(), BODY).await;

      assert_eq!(status_of(result), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_a_tampered_body_with_401() {
      let tampered = br#"{"request_id":"019e18d8-8c36-7bc1-aa77-2bc2f70268c6","status":"OK","payload":{"images":["https://example.com/x.png"]}}"#;

      let result = check(FalWebhookSignatureMode::Enforce, &signed_request(), tampered).await;

      assert_eq!(status_of(result), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_a_replayed_webhook_with_401() {
      let settings = settings(FalWebhookSignatureMode::Enforce, Some(PUBLIC_KEY_JWKS));

      let result = check_fal_webhook_signature_at(&signed_request(), BODY, &settings, NOW + 3600).await;

      assert_eq!(status_of(result), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn answers_500_when_fals_keys_are_unavailable() {
      let settings = settings(FalWebhookSignatureMode::Enforce, None);

      let result = check_fal_webhook_signature_at(&signed_request(), BODY, &settings, NOW).await;

      assert_eq!(status_of(result), StatusCode::INTERNAL_SERVER_ERROR);
    }
  }

  mod report_and_off {
    use super::*;

    #[tokio::test]
    async fn report_lets_a_failing_webhook_through() {
      let result = check(FalWebhookSignatureMode::Report, &TestRequest::post().to_http_request(), BODY).await;

      assert!(result.is_ok());
    }

    #[tokio::test]
    async fn report_lets_a_webhook_through_when_the_keys_are_unavailable() {
      let settings = settings(FalWebhookSignatureMode::Report, None);

      let result = check_fal_webhook_signature_at(&signed_request(), BODY, &settings, NOW).await;

      assert!(result.is_ok());
    }

    #[tokio::test]
    async fn off_checks_nothing_and_fetches_no_keys() {
      // The key source fails, so any attempt to verify would surface as an error in Enforce
      // mode; Off must not even try.
      let settings = settings(FalWebhookSignatureMode::Off, None);

      let result = check_fal_webhook_signature_at(&TestRequest::post().to_http_request(), BODY, &settings, NOW).await;

      assert!(result.is_ok());
    }
  }

  async fn check(mode: FalWebhookSignatureMode, request: &HttpRequest, body: &[u8]) -> Result<(), CommonWebError> {
    check_fal_webhook_signature_at(request, body, &settings(mode, Some(PUBLIC_KEY_JWKS)), NOW).await
  }

  /// Settings whose key source serves `maybe_jwks`, or fails when it is `None`.
  fn settings(mode: FalWebhookSignatureMode, maybe_jwks: Option<&'static str>) -> FalWebhookSignatureSettings {
    FalWebhookSignatureSettings {
      mode,
      jwks_cache: Arc::new(FalJwksCache::with_fetcher(Arc::new(move || {
        Box::pin(async move {
          maybe_jwks
            .map(str::to_string)
            .ok_or_else(|| FalWebhookSignatureError::JwksFetchFailed("test source is down".to_string()))
        })
      }))),
    }
  }

  fn signed_request() -> HttpRequest {
    TestRequest::post()
      .insert_header((REQUEST_ID_HEADER, REQUEST_ID))
      .insert_header((USER_ID_HEADER, USER_ID))
      .insert_header((TIMESTAMP_HEADER, TIMESTAMP))
      .insert_header((SIGNATURE_HEADER, SIGNATURE))
      .to_http_request()
  }

  fn status_of(result: Result<(), CommonWebError>) -> StatusCode {
    result.expect_err("expected the webhook to be rejected").status_code()
  }
}
