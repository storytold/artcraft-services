use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;

use crate::webhook_signature::fal_webhook_public_key::FalWebhookPublicKey;
use crate::webhook_signature::fal_webhook_signature_error::FalWebhookSignatureError;

#[derive(Deserialize)]
struct Jwks {
  keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct Jwk {
  kty: Option<String>,
  crv: Option<String>,
  x: Option<String>,
}

/// Parse FAL's JWKS (`https://rest.fal.ai/.well-known/jwks.json`) into its ED25519 public keys.
///
/// Keys of any other type are skipped. It is an error if no ED25519 key is left, because then
/// no webhook could ever verify.
///
/// FAL encodes `x` as base64url *with* `=` padding, although RFC 7515 calls for none, so both
/// forms are accepted.
pub fn parse_fal_jwks(json: &str) -> Result<Vec<FalWebhookPublicKey>, FalWebhookSignatureError> {
  let jwks: Jwks = serde_json::from_str(json)
    .map_err(|err| FalWebhookSignatureError::JwksParseFailed(err.to_string()))?;

  let keys: Vec<FalWebhookPublicKey> = jwks.keys.iter().filter_map(ed25519_key).collect();

  if keys.is_empty() {
    return Err(FalWebhookSignatureError::JwksParseFailed("the JWKS has no ED25519 keys".to_string()));
  }

  Ok(keys)
}

fn ed25519_key(jwk: &Jwk) -> Option<FalWebhookPublicKey> {
  let is_okp = jwk.kty.as_deref().map_or(true, |kty| kty == "OKP");
  if !is_okp || jwk.crv.as_deref() != Some("Ed25519") {
    return None;
  }

  let x = jwk.x.as_deref()?.trim_end_matches('=');
  let bytes: [u8; 32] = URL_SAFE_NO_PAD.decode(x).ok()?.try_into().ok()?;

  Some(FalWebhookPublicKey::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
  use super::*;

  /// FAL's JWKS as served on 2026-10-15: two keys (so keys can rotate), `x` padded.
  const FAL_JWKS_2026_10_15: &str = r#"{"keys":[{"kty":"OKP","crv":"Ed25519","kid":"Eyv1ENn6NyZnEorJHsWnEDHRQ45-hnGqP0Lb-yX7Itw","x":"Cghs6Ge6h5B3_vMIurJPYmN23Yk-VcTk-juiIEQ6zuY=","use":"sig"},{"kty":"OKP","crv":"Ed25519","kid":"q_RdVYvPKoXPwp5YOM4bhSvtWynZeoAWGOyxQiXgsmQ","x":"TZE-E0LB1iT29Ai1LbAh9bam1YQJkK7fEGWAmmkRm_I=","use":"sig"}]}"#;

  #[test]
  fn parses_fals_published_jwks() {
    let keys = parse_fal_jwks(FAL_JWKS_2026_10_15).unwrap();

    assert_eq!(keys.len(), 2);
    assert_eq!(
      hex::encode(keys[0].as_bytes()),
      "0a086ce867ba879077fef308bab24f626376dd893e55c4e4fa3ba220443acee6",
    );
  }

  #[test]
  fn accepts_x_without_padding() {
    let keys = parse_fal_jwks(r#"{"keys":[{"kty":"OKP","crv":"Ed25519","x":"Cghs6Ge6h5B3_vMIurJPYmN23Yk-VcTk-juiIEQ6zuY"}]}"#).unwrap();

    assert_eq!(keys.len(), 1);
  }

  #[test]
  fn skips_keys_that_are_not_ed25519() {
    let json = r#"{"keys":[
      {"kty":"RSA","n":"abc","e":"AQAB"},
      {"kty":"OKP","crv":"X25519","x":"Cghs6Ge6h5B3_vMIurJPYmN23Yk-VcTk-juiIEQ6zuY"},
      {"kty":"OKP","crv":"Ed25519","x":"TZE-E0LB1iT29Ai1LbAh9bam1YQJkK7fEGWAmmkRm_I="}
    ]}"#;

    let keys = parse_fal_jwks(json).unwrap();

    assert_eq!(keys.len(), 1);
  }

  #[test]
  fn skips_keys_of_the_wrong_length() {
    let err = parse_fal_jwks(r#"{"keys":[{"kty":"OKP","crv":"Ed25519","x":"AAAA"}]}"#).unwrap_err();

    assert!(matches!(err, FalWebhookSignatureError::JwksParseFailed(_)));
  }

  #[test]
  fn rejects_a_jwks_without_usable_keys() {
    assert!(matches!(parse_fal_jwks(r#"{"keys":[]}"#), Err(FalWebhookSignatureError::JwksParseFailed(_))));
    assert!(matches!(parse_fal_jwks("not json"), Err(FalWebhookSignatureError::JwksParseFailed(_))));
  }
}
