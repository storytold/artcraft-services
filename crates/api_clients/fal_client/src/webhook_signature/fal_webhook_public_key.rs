use std::fmt::{Debug, Formatter};

/// An ED25519 public key that FAL signs its webhooks with.
///
/// FAL publishes these in its JWKS (see `parse_fal_jwks`); each key's `x` member is the
/// 32-byte public key.
#[derive(Clone, PartialEq, Eq)]
pub struct FalWebhookPublicKey([u8; 32]);

impl FalWebhookPublicKey {
  pub fn from_bytes(bytes: [u8; 32]) -> Self {
    Self(bytes)
  }

  pub fn as_bytes(&self) -> &[u8; 32] {
    &self.0
  }
}

impl Debug for FalWebhookPublicKey {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "FalWebhookPublicKey({})", hex::encode(self.0))
  }
}
