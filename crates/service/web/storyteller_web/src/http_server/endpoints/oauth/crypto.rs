use std::io::Cursor;

use hashing::sha256::sha256_digest_buffer::sha256_digest_buffer;
use hashing::sha256::sha256_hash_string::sha256_hash_string;

/// SHA-256 hex of a raw token. The database stores this, not the token.
pub fn token_hash(raw: &str) -> Result<String, anyhow::Error> {
  sha256_hash_string(raw)
}

/// SHA-256 digest bytes, used for the PKCE S256 challenge.
pub fn sha256(bytes: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
  let digest = sha256_digest_buffer(Cursor::new(bytes))?;
  Ok(digest.as_ref().to_vec())
}
