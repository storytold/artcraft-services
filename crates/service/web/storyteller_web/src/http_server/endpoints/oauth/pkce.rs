use crate::http_server::endpoints::oauth::crypto::sha256;

const VERIFIER_MIN: usize = 43;
const VERIFIER_MAX: usize = 128;
/// A SHA-256 digest in unpadded base64url. Anything else could never match a verifier.
const S256_CHALLENGE_LEN: usize = 43;

/// RFC 7636 verifier: 43 to 128 unreserved characters.
pub fn code_verifier_is_valid(verifier: &str) -> bool {
  (VERIFIER_MIN..=VERIFIER_MAX).contains(&verifier.len()) && verifier.bytes().all(unreserved)
}

/// S256 challenge: exactly 43 unpadded base64url characters, ending on a character an unpadded SHA-256 digest can produce.
pub fn code_challenge_is_valid(challenge: &str) -> bool {
  challenge.len() == S256_CHALLENGE_LEN && challenge.bytes().all(base64url) && s256_final_byte_is_canonical(challenge.as_bytes()[S256_CHALLENGE_LEN - 1])
}

/// Unpadded base64url SHA-256 of `verifier`.
pub fn s256_challenge(verifier: &str) -> Result<String, anyhow::Error> {
  let digest = sha256(verifier.as_bytes())?;
  Ok(base64url_nopad(&digest))
}

/// `true` when `verifier` is the S256 preimage of `challenge`.
pub fn verify_s256(verifier: &str, challenge: &str) -> Result<bool, anyhow::Error> {
  if !code_verifier_is_valid(verifier) || !code_challenge_is_valid(challenge) {
    return Ok(false);
  }
  let computed = s256_challenge(verifier)?;
  Ok(constant_time_eq(computed.as_bytes(), challenge.as_bytes()))
}

/// RFC 7636 unreserved character.
fn unreserved(byte: u8) -> bool {
  matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

/// Unpadded base64url alphabet, which excludes `+`, `/`, and `=`.
fn base64url(byte: u8) -> bool {
  matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_')
}

/// Final character of an unpadded 32-byte digest. The leftover two bits are zero, so only every fourth base64url index occurs.
fn s256_final_byte_is_canonical(byte: u8) -> bool {
  matches!(byte, b'A' | b'E' | b'I' | b'M' | b'Q' | b'U' | b'Y' | b'c' | b'g' | b'k' | b'o' | b's' | b'w' | b'0' | b'4' | b'8')
}

/// Unpadded base64url. A SHA-256 digest encodes to 43 characters.
fn base64url_nopad(input: &[u8]) -> String {
  const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
  let mut out = String::new();
  let mut index = 0;
  while index + 3 <= input.len() {
    let chunk = ((input[index] as u32) << 16) | ((input[index + 1] as u32) << 8) | (input[index + 2] as u32);
    out.push(TABLE[((chunk >> 18) & 63) as usize] as char);
    out.push(TABLE[((chunk >> 12) & 63) as usize] as char);
    out.push(TABLE[((chunk >> 6) & 63) as usize] as char);
    out.push(TABLE[(chunk & 63) as usize] as char);
    index += 3;
  }
  if index < input.len() {
    let first = input[index] as u32;
    let second = if index + 1 < input.len() { input[index + 1] as u32 } else { 0 };
    let chunk = (first << 16) | (second << 8);
    out.push(TABLE[((chunk >> 18) & 63) as usize] as char);
    out.push(TABLE[((chunk >> 12) & 63) as usize] as char);
    if index + 1 < input.len() {
      out.push(TABLE[((chunk >> 6) & 63) as usize] as char);
    }
  }
  out
}

/// Bytewise equality that does not return on the first mismatch.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
  if left.len() != right.len() {
    return false;
  }
  let mut diff = 0u8;
  for (lhs, rhs) in left.iter().zip(right.iter()) {
    diff |= lhs ^ rhs;
  }
  diff == 0
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rfc7636_s256_vector() {
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
    assert_eq!(s256_challenge(verifier).expect("hash"), challenge);
    assert!(verify_s256(verifier, challenge).expect("hash"));
  }

  #[test]
  fn wrong_verifier_and_plain_challenge_are_rejected() {
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    assert!(!verify_s256(verifier, verifier).expect("hash"));
    assert!(!verify_s256("short", "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM").expect("hash"));
  }

  #[test]
  fn only_a_full_s256_digest_is_a_valid_challenge() {
    let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
    assert!(code_challenge_is_valid(challenge));
    assert!(!code_challenge_is_valid(&challenge[..42]));
    assert!(!code_challenge_is_valid(&format!("{challenge}A")));
    assert!(!code_challenge_is_valid("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw.cM"));
  }

  #[test]
  fn a_noncanonical_s256_final_character_is_rejected() {
    let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
    assert!(code_challenge_is_valid(challenge));
    let mut mutated = challenge.as_bytes().to_vec();
    mutated[S256_CHALLENGE_LEN - 1] = b'N';
    let mutated = std::str::from_utf8(&mutated).expect("ascii");
    assert_eq!(mutated, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cN");
    assert!(!code_challenge_is_valid(mutated));
    assert!(code_challenge_is_valid(&s256_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").expect("hash")));
  }
}
