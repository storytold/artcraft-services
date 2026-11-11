use crate::constants::BCRYPT_MAX_PASSWORD_BYTES;
use crate::errors::password_hash_error::PasswordHashError;

pub fn bcrypt_hash_password(password: String) -> Result<String, PasswordHashError> {
  // NB: Refuse rather than truncate, so every byte of a stored password matters at login.
  if password.len() > BCRYPT_MAX_PASSWORD_BYTES {
    return Err(PasswordHashError::PasswordTooLong);
  }
  let hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST)?;
  Ok(hash)
}

#[cfg(test)]
mod tests {
  use crate::bcrypt_hash_password::bcrypt_hash_password;

  #[test]
  fn test_password_hash_round_trip() {
    let password = "password";
    let hash = bcrypt_hash_password(password.to_string()).unwrap();
    let valid = bcrypt::verify(password, &hash).unwrap();
    assert!(valid);
  }

  #[test]
  fn test_rejects_passwords_bcrypt_would_truncate() {
    use crate::constants::BCRYPT_MAX_PASSWORD_BYTES;
    use crate::errors::password_hash_error::PasswordHashError;

    assert!(bcrypt_hash_password("a".repeat(BCRYPT_MAX_PASSWORD_BYTES)).is_ok());
    assert!(matches!(
      bcrypt_hash_password("a".repeat(BCRYPT_MAX_PASSWORD_BYTES + 1)),
      Err(PasswordHashError::PasswordTooLong)));
  }
}
