use std::collections::BTreeMap;

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};

use crate::jwt_signer_error::JwtSignerError;

#[derive(Clone)]
pub struct JwtSigner {
  encoding_key: EncodingKey,
  decoding_key: DecodingKey,
}

impl JwtSigner {

  pub fn new(hmac_secret: &str) -> Result<Self, JwtSignerError> {
    // NB: Reject an empty secret. The previous HMAC constructor rejected it, and an empty
    // key would make every token forgeable by anyone who knows the scheme.
    if hmac_secret.is_empty() {
      return Err(JwtSignerError::JwtInvalidKeyLength);
    }

    Ok(Self {
      encoding_key: EncodingKey::from_secret(hmac_secret.as_bytes()),
      decoding_key: DecodingKey::from_secret(hmac_secret.as_bytes()),
    })
  }

  /// Turn a map of claims into a signed JWT cookie payload (string)
  pub fn claims_to_jwt(&self, claims: &impl Claimable) -> Result<String, JwtSignerError> {
    claims.sign(&self.encoding_key)
  }

  /// Turn a JWT cookie payload (string) into a map of (key, value) claims
  pub fn jwt_to_claims(&self, jwt: &str) -> Result<BTreeMap<String, String>, JwtSignerError> {
    let mut validation = Validation::new(Algorithm::HS256);

    // NB: These are session cookies, not bearer tokens: they carry no `exp`/`nbf` claims and
    // are expired by the cookie itself, so the library's default claim requirements do not apply.
    validation.required_spec_claims.clear();
    validation.validate_exp = false;
    validation.validate_nbf = false;

    jsonwebtoken::decode::<BTreeMap<String, String>>(jwt, &self.decoding_key, &validation)
      .map(|token_data| token_data.claims)
      .map_err(JwtSignerError::JwtVerifyError)
  }
}

pub trait Claimable {
  fn sign(&self, encoding_key: &EncodingKey) -> Result<String, JwtSignerError>;
}

impl Claimable for BTreeMap<String, String> {
  fn sign(&self, encoding_key: &EncodingKey) -> Result<String, JwtSignerError> {
    encode_claims(self, encoding_key)
  }
}

impl Claimable for BTreeMap<&str, &str> {
  fn sign(&self, encoding_key: &EncodingKey) -> Result<String, JwtSignerError> {
    encode_claims(self, encoding_key)
  }
}

fn encode_claims(claims: &impl serde::Serialize, encoding_key: &EncodingKey) -> Result<String, JwtSignerError> {
  let mut header = Header::new(Algorithm::HS256);

  // NB: Omit the default `"typ":"JWT"` header field. Tokens issued before this change did not
  // carry it, and keeping the header identical means existing cookies decode the same way.
  header.typ = None;

  jsonwebtoken::encode(&header, claims, encoding_key)
    .map_err(JwtSignerError::JwtSignError)
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;

  use crate::jwt_signer::JwtSigner;

  #[test]
  fn test_round_trip() {
    let signer = JwtSigner::new("secret").unwrap();

    let mut claims = BTreeMap::new();
    claims.insert("foo", "yes");
    claims.insert("bar", "no");

    let jwt = signer.claims_to_jwt(&claims).unwrap();

    let output_claims = signer.jwt_to_claims(&jwt).unwrap();

    assert_eq!(claims.len(), output_claims.len());

    for (k, v) in output_claims.iter() {
      assert_eq!(v, claims[k.as_str()]);
    }
  }

  #[test]
  fn test_stability() {
    let signer = JwtSigner::new("secret").unwrap();

    let mut claims = BTreeMap::new();
    claims.insert("foo", "yes");
    claims.insert("bar", "no");

    let jwt = signer.claims_to_jwt(&claims).unwrap();

    assert_eq!("eyJhbGciOiJIUzI1NiJ9.eyJiYXIiOiJubyIsImZvbyI6InllcyJ9.k4eRvIrtw4tuGDtQqSmEDngtGYAaQVRYVMCK5N3JcZA", &jwt);
  }

  #[test]
  fn test_rejects_empty_secret() {
    assert!(JwtSigner::new("").is_err());
  }

  #[test]
  fn test_rejects_tampered_payload() {
    let signer = JwtSigner::new("secret").unwrap();

    let mut claims = BTreeMap::new();
    claims.insert("foo", "yes");

    let jwt = signer.claims_to_jwt(&claims).unwrap();
    let mut tampered = jwt.clone();
    tampered.replace_range(0..1, "x");

    assert!(signer.jwt_to_claims(&tampered).is_err());
  }
}
