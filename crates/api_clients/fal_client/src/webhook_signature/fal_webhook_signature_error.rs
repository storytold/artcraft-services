use std::error::Error;
use std::fmt::{Display, Formatter};

/// Why a FAL webhook request could not be verified as coming from FAL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FalWebhookSignatureError {
  /// A required `X-Fal-Webhook-*` header is absent (or not valid UTF-8).
  MissingHeader(&'static str),

  /// `X-Fal-Webhook-Timestamp` is not a whole number of seconds.
  InvalidTimestamp(String),

  /// The timestamp is further than the tolerance from the current time (a stale or replayed request).
  TimestampOutOfTolerance { timestamp: i64, now: i64 },

  /// `X-Fal-Webhook-Signature` is not 64 bytes of hex.
  InvalidSignatureEncoding,

  /// There are no public keys to verify against.
  NoPublicKeys,

  /// The signature is well formed but no known public key verifies it.
  SignatureMismatch,

  /// The JWKS could not be fetched.
  JwksFetchFailed(String),

  /// The JWKS could not be parsed, or holds no usable ED25519 key.
  JwksParseFailed(String),
}

impl Error for FalWebhookSignatureError {}

impl Display for FalWebhookSignatureError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::MissingHeader(name) => write!(f, "missing webhook header {}", name),
      Self::InvalidTimestamp(value) => write!(f, "webhook timestamp is not an integer: {:?}", value),
      Self::TimestampOutOfTolerance { timestamp, now } => {
        write!(f, "webhook timestamp {} is too far from the current time {}", timestamp, now)
      }
      Self::InvalidSignatureEncoding => write!(f, "webhook signature is not 64 bytes of hex"),
      Self::NoPublicKeys => write!(f, "no FAL webhook public keys to verify against"),
      Self::SignatureMismatch => write!(f, "webhook signature does not match any FAL public key"),
      Self::JwksFetchFailed(reason) => write!(f, "could not fetch the FAL JWKS: {}", reason),
      Self::JwksParseFailed(reason) => write!(f, "could not parse the FAL JWKS: {}", reason),
    }
  }
}
