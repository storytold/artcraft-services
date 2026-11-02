use std::io::Cursor;

use hashing::sha256::sha256_digest_buffer::sha256_digest_buffer;
use hashing::sha256::sha256_hash_string::sha256_hash_string;

pub fn token_hash(raw: &str) -> Result<String, anyhow::Error> {
  sha256_hash_string(raw)
}

pub fn sha256(bytes: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
  let digest = sha256_digest_buffer(Cursor::new(bytes))?;
  Ok(digest.as_ref().to_vec())
}
