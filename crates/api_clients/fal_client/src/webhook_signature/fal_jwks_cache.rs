use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use log::{info, warn};
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::webhook_signature::fal_webhook_public_key::FalWebhookPublicKey;
use crate::webhook_signature::fal_webhook_signature_error::FalWebhookSignatureError;
use crate::webhook_signature::parse_fal_jwks::parse_fal_jwks;

/// Where FAL publishes the public keys its webhooks are signed with.
pub const FAL_JWKS_URL: &str = "https://rest.fal.ai/.well-known/jwks.json";

/// Fetched keys are used for this long before they are fetched again.
pub const REFRESH_AFTER: Duration = Duration::from_secs(12 * 60 * 60);

/// Keys are never used once they are this old, even when a refresh fails: FAL's docs say the
/// JWKS may be cached for up to 24 hours, because the keys can change.
pub const MAX_KEY_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// A signature that no cached key verifies triggers an early refresh (FAL may have rotated its
/// keys), but at most once per this interval, so bad requests can't make us hammer FAL.
pub const MIN_UNKNOWN_KEY_REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// After a failed fetch, no new fetch is attempted for this long. The cache's lock is held
/// while fetching, so without this a JWKS outage would make every webhook wait on a timeout.
pub const RETRY_AFTER_FAILED_FETCH: Duration = Duration::from_secs(60);

const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// Returns the JWKS document as text.
pub type JwksFetchFuture = Pin<Box<dyn Future<Output = Result<String, FalWebhookSignatureError>> + Send>>;
pub type JwksFetcher = Arc<dyn Fn() -> JwksFetchFuture + Send + Sync>;

/// Caches FAL's webhook public keys, fetching them when they are missing or stale.
///
/// Concurrent callers share one fetch: the lock is held while fetching.
pub struct FalJwksCache {
  fetcher: JwksFetcher,
  state: Mutex<CacheState>,
}

struct CacheState {
  keys: Vec<FalWebhookPublicKey>,
  fetched_at: Option<Instant>,
  last_unknown_key_refresh: Option<Instant>,
  /// No fetch is attempted before this (set after a failed fetch).
  maybe_retry_at: Option<Instant>,
}

impl FalJwksCache {
  /// A cache that fetches FAL's JWKS from [`FAL_JWKS_URL`].
  pub fn for_fal() -> Self {
    Self::with_fetcher(Arc::new(|| Box::pin(fetch_fal_jwks())))
  }

  /// A cache that gets the JWKS document from `fetcher` (used by tests and alternative sources).
  pub fn with_fetcher(fetcher: JwksFetcher) -> Self {
    Self {
      fetcher,
      state: Mutex::new(CacheState {
        keys: Vec::new(),
        fetched_at: None,
        last_unknown_key_refresh: None,
        maybe_retry_at: None,
      }),
    }
  }

  /// The keys to verify with: the cached ones while they are fresh, otherwise newly fetched.
  ///
  /// If a refresh fails, keys younger than [`MAX_KEY_AGE`] are still used (and the failure is
  /// logged), so a brief JWKS outage doesn't reject genuine webhooks. After a failure, fetching
  /// waits [`RETRY_AFTER_FAILED_FETCH`] before it is tried again.
  pub async fn keys(&self) -> Result<Vec<FalWebhookPublicKey>, FalWebhookSignatureError> {
    let mut state = self.state.lock().await;
    let now = Instant::now();

    let age = state.fetched_at.map(|fetched_at| now.saturating_duration_since(fetched_at));
    let usable = age.is_some_and(|age| age < MAX_KEY_AGE);

    if age.is_some_and(|age| age < REFRESH_AFTER) {
      return Ok(state.keys.clone());
    }

    if state.maybe_retry_at.is_some_and(|retry_at| now < retry_at) {
      return if usable {
        Ok(state.keys.clone())
      } else {
        Err(FalWebhookSignatureError::JwksFetchFailed("waiting to retry after a failed fetch".to_string()))
      };
    }

    match self.fetch().await {
      Ok(keys) => {
        state.keys = keys;
        state.fetched_at = Some(now);
        state.maybe_retry_at = None;
        Ok(state.keys.clone())
      }
      Err(err) => {
        state.maybe_retry_at = Some(now + RETRY_AFTER_FAILED_FETCH);
        if usable {
          warn!("Could not refresh the FAL JWKS, using keys fetched {:?} ago: {}", age, err);
          Ok(state.keys.clone())
        } else {
          Err(err)
        }
      }
    }
  }

  /// Fetch the keys again now, because a signature matched none of the cached ones and FAL may
  /// have rotated its keys. Rate-limited to once per [`MIN_UNKNOWN_KEY_REFRESH_INTERVAL`].
  ///
  /// Returns whether new keys were fetched (so a retry can make a difference).
  pub async fn refresh_after_unknown_key(&self) -> Result<bool, FalWebhookSignatureError> {
    let mut state = self.state.lock().await;
    let now = Instant::now();

    let recently = state.last_unknown_key_refresh
      .is_some_and(|last| now.saturating_duration_since(last) < MIN_UNKNOWN_KEY_REFRESH_INTERVAL);

    if recently {
      return Ok(false);
    }

    state.last_unknown_key_refresh = Some(now);

    let keys = self.fetch().await?;
    let changed = keys != state.keys;

    state.keys = keys;
    state.fetched_at = Some(now);

    if changed {
      info!("The FAL JWKS changed; using the new keys");
    }

    Ok(changed)
  }

  async fn fetch(&self) -> Result<Vec<FalWebhookPublicKey>, FalWebhookSignatureError> {
    let body = (self.fetcher)().await?;
    parse_fal_jwks(&body)
  }
}

async fn fetch_fal_jwks() -> Result<String, FalWebhookSignatureError> {
  let failed = |err: reqwest::Error| FalWebhookSignatureError::JwksFetchFailed(err.to_string());

  let response = reqwest::Client::builder()
    .timeout(FETCH_TIMEOUT)
    .build()
    .map_err(failed)?
    .get(FAL_JWKS_URL)
    .send()
    .await
    .map_err(failed)?;

  let status = response.status();

  if !status.is_success() {
    return Err(FalWebhookSignatureError::JwksFetchFailed(format!("HTTP {}", status)));
  }

  response.text().await.map_err(failed)
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::sync::Mutex as StdMutex;

  use super::*;

  const KEY_A: &str = r#"{"keys":[{"kty":"OKP","crv":"Ed25519","x":"Cghs6Ge6h5B3_vMIurJPYmN23Yk-VcTk-juiIEQ6zuY="}]}"#;
  const KEY_B: &str = r#"{"keys":[{"kty":"OKP","crv":"Ed25519","x":"TZE-E0LB1iT29Ai1LbAh9bam1YQJkK7fEGWAmmkRm_I="}]}"#;

  mod caching {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn fetches_once_while_the_keys_are_fresh() {
      let source = FakeSource::new(KEY_A);
      let cache = source.cache();

      cache.keys().await.unwrap();
      tokio::time::advance(REFRESH_AFTER - Duration::from_secs(1)).await;
      cache.keys().await.unwrap();

      assert_eq!(source.fetches(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn refetches_once_the_keys_are_stale() {
      let source = FakeSource::new(KEY_A);
      let cache = source.cache();

      let before = cache.keys().await.unwrap();
      source.serve(KEY_B);
      tokio::time::advance(REFRESH_AFTER).await;
      let after = cache.keys().await.unwrap();

      assert_eq!(source.fetches(), 2);
      assert_ne!(before, after);
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_using_stale_keys_through_a_failed_refresh() {
      let source = FakeSource::new(KEY_A);
      let cache = source.cache();

      let fetched = cache.keys().await.unwrap();
      source.fail();
      tokio::time::advance(REFRESH_AFTER).await;

      assert_eq!(cache.keys().await.unwrap(), fetched);
    }

    #[tokio::test(start_paused = true)]
    async fn never_uses_keys_older_than_the_max_age() {
      let source = FakeSource::new(KEY_A);
      let cache = source.cache();

      cache.keys().await.unwrap();
      source.fail();
      tokio::time::advance(MAX_KEY_AGE).await;

      assert!(matches!(cache.keys().await, Err(FalWebhookSignatureError::JwksFetchFailed(_))));
    }

    #[tokio::test(start_paused = true)]
    async fn waits_before_retrying_a_failed_fetch() {
      let source = FakeSource::new(KEY_A);
      let cache = source.cache();

      cache.keys().await.unwrap();
      source.fail();
      tokio::time::advance(REFRESH_AFTER).await;
      cache.keys().await.unwrap();
      cache.keys().await.unwrap();

      assert_eq!(source.fetches(), 2);

      source.serve(KEY_B);
      tokio::time::advance(RETRY_AFTER_FAILED_FETCH).await;

      assert_eq!(cache.keys().await.unwrap(), parse_fal_jwks(KEY_B).unwrap());
      assert_eq!(source.fetches(), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn reports_a_failed_first_fetch() {
      let source = FakeSource::new(KEY_A);
      source.fail();

      assert!(matches!(source.cache().keys().await, Err(FalWebhookSignatureError::JwksFetchFailed(_))));
    }
  }

  mod unknown_key_refresh {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn picks_up_rotated_keys() {
      let source = FakeSource::new(KEY_A);
      let cache = source.cache();

      cache.keys().await.unwrap();
      source.serve(KEY_B);

      assert!(cache.refresh_after_unknown_key().await.unwrap());
      assert_eq!(cache.keys().await.unwrap(), parse_fal_jwks(KEY_B).unwrap());
    }

    #[tokio::test(start_paused = true)]
    async fn reports_no_change_when_the_keys_are_the_same() {
      let source = FakeSource::new(KEY_A);
      let cache = source.cache();

      cache.keys().await.unwrap();

      assert!(!cache.refresh_after_unknown_key().await.unwrap());
    }

    #[tokio::test(start_paused = true)]
    async fn is_rate_limited() {
      let source = FakeSource::new(KEY_A);
      let cache = source.cache();

      cache.keys().await.unwrap();
      cache.refresh_after_unknown_key().await.unwrap();
      source.serve(KEY_B);
      tokio::time::advance(MIN_UNKNOWN_KEY_REFRESH_INTERVAL - Duration::from_secs(1)).await;

      assert!(!cache.refresh_after_unknown_key().await.unwrap());
      assert_eq!(source.fetches(), 2);

      tokio::time::advance(Duration::from_secs(1)).await;

      assert!(cache.refresh_after_unknown_key().await.unwrap());
      assert_eq!(source.fetches(), 3);
    }
  }

  /// Fetches FAL's real JWKS. Needs network access, so it only runs on request:
  /// `cargo test -p fal_client -- --ignored fetches_fals_live_jwks`
  #[tokio::test]
  #[ignore]
  async fn fetches_fals_live_jwks() {
    let keys = FalJwksCache::for_fal().keys().await.unwrap();

    assert!(!keys.is_empty());
  }

  /// A JWKS source the test controls: what it serves, whether it fails, and how often it was asked.
  struct FakeSource {
    body: Arc<StdMutex<Option<String>>>,
    fetches: Arc<AtomicUsize>,
  }

  impl FakeSource {
    fn new(body: &str) -> Self {
      Self {
        body: Arc::new(StdMutex::new(Some(body.to_string()))),
        fetches: Arc::new(AtomicUsize::new(0)),
      }
    }

    fn cache(&self) -> FalJwksCache {
      let body = self.body.clone();
      let fetches = self.fetches.clone();

      FalJwksCache::with_fetcher(Arc::new(move || {
        fetches.fetch_add(1, Ordering::SeqCst);
        let served = body.lock().unwrap().clone();
        Box::pin(async move {
          served.ok_or_else(|| FalWebhookSignatureError::JwksFetchFailed("test source is down".to_string()))
        })
      }))
    }

    fn serve(&self, body: &str) {
      *self.body.lock().unwrap() = Some(body.to_string());
    }

    fn fail(&self) {
      *self.body.lock().unwrap() = None;
    }

    fn fetches(&self) -> usize {
      self.fetches.load(Ordering::SeqCst)
    }
  }
}
