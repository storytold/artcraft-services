use std::sync::Arc;

use fal_client::webhook_signature::fal_jwks_cache::FalJwksCache;

use crate::http_server::endpoints::webhooks::fal::signature::fal_webhook_signature_mode::FalWebhookSignatureMode;

/// How the FAL webhook endpoint verifies that requests come from FAL.
#[derive(Clone)]
pub struct FalWebhookSignatureSettings {
  pub mode: FalWebhookSignatureMode,

  /// FAL's public keys, shared by all requests.
  pub jwks_cache: Arc<FalJwksCache>,
}

impl FalWebhookSignatureSettings {
  /// Settings for production: the mode from the environment, keys fetched from FAL.
  pub fn from_env() -> errors::AnyhowResult<Self> {
    Ok(Self {
      mode: FalWebhookSignatureMode::from_env()?,
      jwks_cache: Arc::new(FalJwksCache::for_fal()),
    })
  }
}
