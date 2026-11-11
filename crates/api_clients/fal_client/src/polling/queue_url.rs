use url::Url;

use crate::error::client_error::FalClientError;
use crate::error::fal_error_plus::FalErrorPlus;

const EXPECTED_SCHEME: &str = "https";
const EXPECTED_HOST: &str = "queue.fal.run";

/// Validate a FAL queue URL (status or response) before the API key is attached to a request.
///
/// The key must only ever be sent over TLS to the FAL queue host, so both the scheme and host
/// are checked.
pub(crate) fn validate_queue_url(url: &str, url_kind: &str) -> Result<Url, FalErrorPlus> {
  let parsed = Url::parse(url)?;

  if parsed.scheme() != EXPECTED_SCHEME {
    return Err(FalErrorPlus::ClientError(FalClientError::InvalidUrl(format!(
      "Expected scheme '{}' but got '{}' in {} URL: {}",
      EXPECTED_SCHEME,
      parsed.scheme(),
      url_kind,
      url,
    ))));
  }

  let host = parsed.host_str().unwrap_or("");

  if host != EXPECTED_HOST {
    return Err(FalErrorPlus::ClientError(FalClientError::InvalidUrl(format!(
      "Expected host '{}' but got '{}' in {} URL: {}",
      EXPECTED_HOST,
      host,
      url_kind,
      url,
    ))));
  }

  Ok(parsed)
}

/// HTTP client for credentialed queue polling. Refuses plain-HTTP requests, including redirects.
pub(crate) fn queue_http_client() -> Result<reqwest::Client, FalErrorPlus> {
  Ok(reqwest::Client::builder()
    .https_only(true)
    .build()?)
}

#[cfg(test)]
mod tests {
  use super::validate_queue_url;

  #[test]
  fn accepts_https_queue_url() {
    assert!(validate_queue_url("https://queue.fal.run/fal-ai/flux/requests/abc/status", "status").is_ok());
  }

  #[test]
  fn rejects_plain_http_queue_url() {
    assert!(validate_queue_url("http://queue.fal.run/fal-ai/flux/requests/abc/status", "status").is_err());
  }

  #[test]
  fn rejects_other_hosts() {
    assert!(validate_queue_url("https://queue.fal.run.example.com/x", "status").is_err());
    assert!(validate_queue_url("https://example.com/queue.fal.run", "status").is_err());
  }
}
