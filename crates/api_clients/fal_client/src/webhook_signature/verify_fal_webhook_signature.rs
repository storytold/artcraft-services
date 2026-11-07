use ring::signature::{UnparsedPublicKey, ED25519};
use sha2::{Digest, Sha256};

use crate::webhook_signature::fal_webhook_public_key::FalWebhookPublicKey;
use crate::webhook_signature::fal_webhook_signature_error::FalWebhookSignatureError;
use crate::webhook_signature::fal_webhook_signature_headers::FalWebhookSignatureHeaders;

/// FAL webhooks older or newer than this (in seconds) are rejected, as FAL's docs require.
pub const TIMESTAMP_TOLERANCE_SECONDS: i64 = 300;

const ED25519_SIGNATURE_LENGTH: usize = 64;

/// Verify that a webhook was signed by FAL, following FAL's published scheme:
///
/// 1. The timestamp must be within [`TIMESTAMP_TOLERANCE_SECONDS`] of `now_unix_seconds`.
/// 2. The signed message is the request ID, user ID, timestamp and the hex SHA-256 of the
///    *raw* request body, joined by `\n`.
/// 3. The hex signature must verify as ED25519 under at least one of `public_keys`.
///
/// This checks the signature only. It does not compare the header's request ID with the one in
/// the body; callers that act on the body's request ID should do that as well.
pub fn verify_fal_webhook_signature(
  headers: &FalWebhookSignatureHeaders,
  raw_body: &[u8],
  public_keys: &[FalWebhookPublicKey],
  now_unix_seconds: i64,
) -> Result<(), FalWebhookSignatureError> {
  let timestamp: i64 = headers.timestamp.trim().parse()
    .map_err(|_| FalWebhookSignatureError::InvalidTimestamp(headers.timestamp.clone()))?;

  // i128 so that extreme (hostile) timestamps can't overflow the subtraction.
  if (now_unix_seconds as i128 - timestamp as i128).abs() > TIMESTAMP_TOLERANCE_SECONDS as i128 {
    return Err(FalWebhookSignatureError::TimestampOutOfTolerance { timestamp, now: now_unix_seconds });
  }

  let signature = hex::decode(headers.signature.trim())
    .map_err(|_| FalWebhookSignatureError::InvalidSignatureEncoding)?;

  if signature.len() != ED25519_SIGNATURE_LENGTH {
    return Err(FalWebhookSignatureError::InvalidSignatureEncoding);
  }

  if public_keys.is_empty() {
    return Err(FalWebhookSignatureError::NoPublicKeys);
  }

  let message = signed_message(headers, raw_body);

  let verified = public_keys.iter().any(|key| {
    UnparsedPublicKey::new(&ED25519, key.as_bytes())
      .verify(message.as_bytes(), &signature)
      .is_ok()
  });

  if verified {
    Ok(())
  } else {
    Err(FalWebhookSignatureError::SignatureMismatch)
  }
}

/// The exact bytes FAL signs. The timestamp is the header's own text, not a re-formatted number.
fn signed_message(headers: &FalWebhookSignatureHeaders, raw_body: &[u8]) -> String {
  let body_hash = hex::encode(Sha256::digest(raw_body));
  format!("{}\n{}\n{}\n{}", headers.request_id, headers.user_id, headers.timestamp, body_hash)
}

#[cfg(test)]
mod tests {
  use ring::rand::SystemRandom;
  use ring::signature::{Ed25519KeyPair, KeyPair};

  use super::*;

  // An independent test vector, made with PyNaCl (libsodium) by following FAL's documented
  // algorithm, so the message format isn't only checked against this file's own code:
  //
  //   sk = nacl.signing.SigningKey(bytes(range(32)))
  //   message = "\n".join([request_id, user_id, timestamp, sha256(body).hexdigest()])
  //   sk.sign(message.encode()).signature.hex()
  const VECTOR_PUBLIC_KEY: &str = "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";
  const VECTOR_REQUEST_ID: &str = "019e18d8-8c36-7bc1-aa77-2bc2f70268c6";
  const VECTOR_USER_ID: &str = "test-user-1";
  const VECTOR_TIMESTAMP: &str = "1760500000";
  const VECTOR_BODY: &[u8] = br#"{"request_id":"019e18d8-8c36-7bc1-aa77-2bc2f70268c6","status":"OK","payload":{"images":[]}}"#;
  const VECTOR_SIGNATURE: &str = "d925f70845ad2e5a410c3b671a443a27cf538bf1f20b9918b6ce583f0f3640c6f9b2a407a442fa3d770e914ea27f4d4bfc07fd64f3496672e0da6af9a930040e";
  const VECTOR_NOW: i64 = 1760500000;

  mod independent_vector {
    use super::*;

    #[test]
    fn verifies() {
      assert_eq!(verify(&vector_headers(), VECTOR_BODY, &[vector_key()], VECTOR_NOW), Ok(()));
    }

    #[test]
    fn verifies_with_the_right_key_among_others() {
      let keys = [random_key_pair().1, vector_key(), random_key_pair().1];

      assert_eq!(verify(&vector_headers(), VECTOR_BODY, &keys, VECTOR_NOW), Ok(()));
    }

    #[test]
    fn rejects_a_changed_body() {
      let body = br#"{"request_id":"019e18d8-8c36-7bc1-aa77-2bc2f70268c6","status":"OK","payload":{"images":[1]}}"#;

      assert_eq!(verify(&vector_headers(), body, &[vector_key()], VECTOR_NOW), Err(FalWebhookSignatureError::SignatureMismatch));
    }

    #[test]
    fn rejects_a_changed_request_id_user_id_or_timestamp() {
      let changed = [
        FalWebhookSignatureHeaders { request_id: "019e18d8-8c36-7bc1-aa77-2bc2f70268c7".to_string(), ..vector_headers() },
        FalWebhookSignatureHeaders { user_id: "test-user-2".to_string(), ..vector_headers() },
        FalWebhookSignatureHeaders { timestamp: "1760500001".to_string(), ..vector_headers() },
      ];

      for headers in changed {
        assert_eq!(verify(&headers, VECTOR_BODY, &[vector_key()], VECTOR_NOW), Err(FalWebhookSignatureError::SignatureMismatch), "{headers:?}");
      }
    }

    #[test]
    fn rejects_a_signature_from_another_key() {
      assert_eq!(
        verify(&vector_headers(), VECTOR_BODY, &[random_key_pair().1], VECTOR_NOW),
        Err(FalWebhookSignatureError::SignatureMismatch),
      );
    }

    #[test]
    fn accepts_uppercase_hex() {
      let headers = FalWebhookSignatureHeaders { signature: VECTOR_SIGNATURE.to_uppercase(), ..vector_headers() };

      assert_eq!(verify(&headers, VECTOR_BODY, &[vector_key()], VECTOR_NOW), Ok(()));
    }
  }

  mod timestamps {
    use super::*;

    #[test]
    fn accepts_the_edges_of_the_tolerance() {
      for now in [VECTOR_NOW - TIMESTAMP_TOLERANCE_SECONDS, VECTOR_NOW + TIMESTAMP_TOLERANCE_SECONDS] {
        assert_eq!(verify(&vector_headers(), VECTOR_BODY, &[vector_key()], now), Ok(()), "now = {now}");
      }
    }

    #[test]
    fn rejects_stale_and_future_requests() {
      for now in [VECTOR_NOW - TIMESTAMP_TOLERANCE_SECONDS - 1, VECTOR_NOW + TIMESTAMP_TOLERANCE_SECONDS + 1] {
        assert_eq!(
          verify(&vector_headers(), VECTOR_BODY, &[vector_key()], now),
          Err(FalWebhookSignatureError::TimestampOutOfTolerance { timestamp: 1760500000, now }),
        );
      }
    }

    #[test]
    fn rejects_a_timestamp_that_is_not_a_number() {
      for timestamp in ["", "soon", "1760500000.5", "99999999999999999999999"] {
        let headers = FalWebhookSignatureHeaders { timestamp: timestamp.to_string(), ..vector_headers() };

        assert!(matches!(verify(&headers, VECTOR_BODY, &[vector_key()], VECTOR_NOW), Err(FalWebhookSignatureError::InvalidTimestamp(_))), "{timestamp:?}");
      }
    }

    #[test]
    fn extreme_timestamps_do_not_overflow() {
      let headers = FalWebhookSignatureHeaders { timestamp: i64::MIN.to_string(), ..vector_headers() };

      assert!(matches!(
        verify(&headers, VECTOR_BODY, &[vector_key()], i64::MAX),
        Err(FalWebhookSignatureError::TimestampOutOfTolerance { .. }),
      ));
    }
  }

  mod malformed_input {
    use super::*;

    #[test]
    fn rejects_signatures_that_are_not_64_bytes_of_hex() {
      for signature in ["", "zz", "abcd", &VECTOR_SIGNATURE[..126], &format!("{VECTOR_SIGNATURE}00")] {
        let headers = FalWebhookSignatureHeaders { signature: signature.to_string(), ..vector_headers() };

        assert_eq!(verify(&headers, VECTOR_BODY, &[vector_key()], VECTOR_NOW), Err(FalWebhookSignatureError::InvalidSignatureEncoding), "{signature:?}");
      }
    }

    #[test]
    fn rejects_when_there_are_no_keys() {
      assert_eq!(verify(&vector_headers(), VECTOR_BODY, &[], VECTOR_NOW), Err(FalWebhookSignatureError::NoPublicKeys));
    }
  }

  mod round_trip {
    use super::*;

    #[test]
    fn a_freshly_signed_webhook_verifies() {
      let (key_pair, public_key) = random_key_pair();
      let body = br#"{"status":"ERROR","request_id":"abc","error":"boom"}"#;
      let mut headers = FalWebhookSignatureHeaders {
        request_id: "abc".to_string(),
        user_id: "user".to_string(),
        timestamp: VECTOR_NOW.to_string(),
        signature: String::new(),
      };
      headers.signature = hex::encode(key_pair.sign(signed_message(&headers, body).as_bytes()).as_ref());

      assert_eq!(verify(&headers, body, &[public_key], VECTOR_NOW), Ok(()));
    }
  }

  fn verify(
    headers: &FalWebhookSignatureHeaders,
    body: &[u8],
    keys: &[FalWebhookPublicKey],
    now: i64,
  ) -> Result<(), FalWebhookSignatureError> {
    verify_fal_webhook_signature(headers, body, keys, now)
  }

  fn vector_headers() -> FalWebhookSignatureHeaders {
    FalWebhookSignatureHeaders {
      request_id: VECTOR_REQUEST_ID.to_string(),
      user_id: VECTOR_USER_ID.to_string(),
      timestamp: VECTOR_TIMESTAMP.to_string(),
      signature: VECTOR_SIGNATURE.to_string(),
    }
  }

  fn vector_key() -> FalWebhookPublicKey {
    FalWebhookPublicKey::from_bytes(hex::decode(VECTOR_PUBLIC_KEY).unwrap().try_into().unwrap())
  }

  fn random_key_pair() -> (Ed25519KeyPair, FalWebhookPublicKey) {
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
    let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let public_key = FalWebhookPublicKey::from_bytes(key_pair.public_key().as_ref().try_into().unwrap());
    (key_pair, public_key)
  }
}
