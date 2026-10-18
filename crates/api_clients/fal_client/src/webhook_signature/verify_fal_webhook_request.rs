use crate::webhook_signature::fal_jwks_cache::FalJwksCache;
use crate::webhook_signature::fal_webhook_signature_error::FalWebhookSignatureError;
use crate::webhook_signature::fal_webhook_signature_headers::FalWebhookSignatureHeaders;
use crate::webhook_signature::verify_fal_webhook_signature::verify_fal_webhook_signature;

/// Verify a webhook request against FAL's published keys, fetching them through `jwks_cache`.
///
/// When no cached key matches, the keys are fetched again once (FAL may have rotated them) and
/// the signature is checked against the new ones. Other failures (stale timestamp, malformed
/// headers) never trigger a fetch.
pub async fn verify_fal_webhook_request(
  jwks_cache: &FalJwksCache,
  headers: &FalWebhookSignatureHeaders,
  raw_body: &[u8],
  now_unix_seconds: i64,
) -> Result<(), FalWebhookSignatureError> {
  let keys = jwks_cache.keys().await?;

  match verify_fal_webhook_signature(headers, raw_body, &keys, now_unix_seconds) {
    Err(FalWebhookSignatureError::SignatureMismatch) => {
      if jwks_cache.refresh_after_unknown_key().await? {
        let keys = jwks_cache.keys().await?;
        verify_fal_webhook_signature(headers, raw_body, &keys, now_unix_seconds)
      } else {
        Err(FalWebhookSignatureError::SignatureMismatch)
      }
    }
    result => result,
  }
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::sync::{Arc, Mutex};

  use base64::engine::general_purpose::URL_SAFE;
  use base64::Engine;
  use ring::rand::SystemRandom;
  use ring::signature::{Ed25519KeyPair, KeyPair};
  use sha2::{Digest, Sha256};

  use super::*;

  const NOW: i64 = 1_760_500_000;
  const BODY: &[u8] = br#"{"request_id":"req-1","status":"OK","payload":{}}"#;

  #[tokio::test(start_paused = true)]
  async fn verifies_with_the_cached_keys() {
    let signer = Signer::new();
    let source = Source::serving(&[&signer]);

    let result = verify_fal_webhook_request(&source.cache(), &signer.sign(BODY), BODY, NOW).await;

    assert_eq!(result, Ok(()));
    assert_eq!(source.fetches(), 1);
  }

  #[tokio::test(start_paused = true)]
  async fn picks_up_a_rotated_key_with_one_extra_fetch() {
    let old = Signer::new();
    let new = Signer::new();
    let source = Source::serving(&[&old]);
    let cache = source.cache();
    cache.keys().await.unwrap();

    source.serve(&[&new]);
    let result = verify_fal_webhook_request(&cache, &new.sign(BODY), BODY, NOW).await;

    assert_eq!(result, Ok(()));
    assert_eq!(source.fetches(), 2);
  }

  #[tokio::test(start_paused = true)]
  async fn a_forged_signature_fails_after_at_most_one_extra_fetch() {
    let genuine = Signer::new();
    let forger = Signer::new();
    let source = Source::serving(&[&genuine]);
    let cache = source.cache();

    for _ in 0..3 {
      let result = verify_fal_webhook_request(&cache, &forger.sign(BODY), BODY, NOW).await;
      assert_eq!(result, Err(FalWebhookSignatureError::SignatureMismatch));
    }

    assert_eq!(source.fetches(), 2);
  }

  #[tokio::test(start_paused = true)]
  async fn a_stale_timestamp_never_triggers_a_fetch() {
    let signer = Signer::new();
    let source = Source::serving(&[&signer]);
    let cache = source.cache();

    let result = verify_fal_webhook_request(&cache, &signer.sign(BODY), BODY, NOW + 3600).await;

    assert!(matches!(result, Err(FalWebhookSignatureError::TimestampOutOfTolerance { .. })));
    assert_eq!(source.fetches(), 1);
  }

  struct Signer {
    key_pair: Ed25519KeyPair,
  }

  impl Signer {
    fn new() -> Self {
      let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
      Self { key_pair: Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap() }
    }

    /// The JWK FAL would publish for this key (`x` padded, as FAL serves it).
    fn jwk(&self) -> String {
      let x = URL_SAFE.encode(self.key_pair.public_key().as_ref());
      format!(r#"{{"kty":"OKP","crv":"Ed25519","x":"{x}","use":"sig"}}"#)
    }

    fn sign(&self, body: &[u8]) -> FalWebhookSignatureHeaders {
      let (request_id, user_id, timestamp) = ("req-1", "user-1", NOW.to_string());
      let message = format!("{request_id}\n{user_id}\n{timestamp}\n{}", hex::encode(Sha256::digest(body)));
      FalWebhookSignatureHeaders {
        request_id: request_id.to_string(),
        user_id: user_id.to_string(),
        timestamp,
        signature: hex::encode(self.key_pair.sign(message.as_bytes()).as_ref()),
      }
    }
  }

  struct Source {
    body: Arc<Mutex<String>>,
    fetches: Arc<AtomicUsize>,
  }

  impl Source {
    fn serving(signers: &[&Signer]) -> Self {
      let source = Self { body: Arc::new(Mutex::new(String::new())), fetches: Arc::new(AtomicUsize::new(0)) };
      source.serve(signers);
      source
    }

    fn serve(&self, signers: &[&Signer]) {
      let keys: Vec<String> = signers.iter().map(|signer| signer.jwk()).collect();
      *self.body.lock().unwrap() = format!(r#"{{"keys":[{}]}}"#, keys.join(","));
    }

    fn cache(&self) -> FalJwksCache {
      let body = self.body.clone();
      let fetches = self.fetches.clone();
      FalJwksCache::with_fetcher(Arc::new(move || {
        fetches.fetch_add(1, Ordering::SeqCst);
        let served = body.lock().unwrap().clone();
        Box::pin(async move { Ok(served) })
      }))
    }

    fn fetches(&self) -> usize {
      self.fetches.load(Ordering::SeqCst)
    }
  }
}
