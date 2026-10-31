use crate::webhook_signature::fal_webhook_signature_error::FalWebhookSignatureError;

pub const REQUEST_ID_HEADER: &str = "X-Fal-Webhook-Request-Id";
pub const USER_ID_HEADER: &str = "X-Fal-Webhook-User-Id";
pub const TIMESTAMP_HEADER: &str = "X-Fal-Webhook-Timestamp";
pub const SIGNATURE_HEADER: &str = "X-Fal-Webhook-Signature";

/// The four headers FAL sends with every webhook so the receiver can verify it.
///
/// Values are kept exactly as received: the timestamp in particular is signed as the
/// header's text, not as a re-formatted number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FalWebhookSignatureHeaders {
  pub request_id: String,
  pub user_id: String,
  pub timestamp: String,
  /// Hex-encoded ED25519 signature.
  pub signature: String,
}

impl FalWebhookSignatureHeaders {
  /// Read the headers through `get_header`, which looks one up by name.
  ///
  /// HTTP header names are case-insensitive; `get_header` is expected to honour that (as the
  /// header maps of actix-web, reqwest and `http` do).
  pub fn from_lookup<'a, F>(get_header: F) -> Result<Self, FalWebhookSignatureError>
  where
    F: Fn(&str) -> Option<&'a str>,
  {
    let required = |name: &'static str| {
      get_header(name)
        .map(str::to_string)
        .ok_or(FalWebhookSignatureError::MissingHeader(name))
    };

    Ok(Self {
      request_id: required(REQUEST_ID_HEADER)?,
      user_id: required(USER_ID_HEADER)?,
      timestamp: required(TIMESTAMP_HEADER)?,
      signature: required(SIGNATURE_HEADER)?,
    })
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::*;

  #[test]
  fn reads_all_four_headers() {
    let headers = header_map(&[
      (REQUEST_ID_HEADER, "req-1"),
      (USER_ID_HEADER, "user-1"),
      (TIMESTAMP_HEADER, "1700000000"),
      (SIGNATURE_HEADER, "abcd"),
    ]);

    let parsed = FalWebhookSignatureHeaders::from_lookup(|name| headers.get(name).map(String::as_str)).unwrap();

    assert_eq!(parsed.request_id, "req-1");
    assert_eq!(parsed.user_id, "user-1");
    assert_eq!(parsed.timestamp, "1700000000");
    assert_eq!(parsed.signature, "abcd");
  }

  #[test]
  fn names_the_first_missing_header() {
    let headers = header_map(&[
      (REQUEST_ID_HEADER, "req-1"),
      (USER_ID_HEADER, "user-1"),
      (SIGNATURE_HEADER, "abcd"),
    ]);

    let err = FalWebhookSignatureHeaders::from_lookup(|name| headers.get(name).map(String::as_str)).unwrap_err();

    assert_eq!(err, FalWebhookSignatureError::MissingHeader(TIMESTAMP_HEADER));
  }

  fn header_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
  }
}
