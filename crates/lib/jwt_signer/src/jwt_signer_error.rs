use std::error::Error;
use std::fmt::{Display, Formatter};

use jsonwebtoken::errors::Error as JwtLibraryError;

#[derive(Debug)]
pub enum JwtSignerError {
  /// Error decoding and verifying a JWT to claims
  JwtVerifyError(JwtLibraryError),

  /// Error encoding and signing JWT claims
  JwtSignError(JwtLibraryError),

  /// Error constructing the JWT signer.
  JwtInvalidKeyLength,
}

impl Display for JwtSignerError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::JwtVerifyError(e) => write!(f, "JWT verify error: {}", e),
      Self::JwtSignError(e) => write!(f, "JWT sign error: {}", e),
      Self::JwtInvalidKeyLength => write!(f, "JWT signer construction failed: invalid key length"),
    }
  }
}

impl Error for JwtSignerError {}
