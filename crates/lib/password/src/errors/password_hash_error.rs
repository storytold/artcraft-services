use std::fmt;

#[derive(Debug)]
pub enum PasswordHashError {
  /// An error with hashing the password.
  BcryptHashError(bcrypt::BcryptError),
  /// The password is longer than bcrypt can hash without truncating it.
  PasswordTooLong,
}

impl fmt::Display for PasswordHashError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::BcryptHashError(e) => write!(f, "bcrypt hash error: {}", e),
      Self::PasswordTooLong => write!(f, "password is too long"),
    }
  }
}

impl std::error::Error for PasswordHashError {}

impl From<bcrypt::BcryptError> for PasswordHashError {
  fn from(err: bcrypt::BcryptError) -> Self {
    Self::BcryptHashError(err)
  }
}
