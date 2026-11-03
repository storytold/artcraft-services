
use password::constants::BCRYPT_MAX_PASSWORD_BYTES;

pub fn validate_passwords(password: &str, password_confirmation: &str) -> Result<(), String> {
  if password.len() < 6 {
    return Err("password is too short".to_string());
  }

  if password.len() > BCRYPT_MAX_PASSWORD_BYTES {
    return Err(format!("password is too long (at most {} bytes)", BCRYPT_MAX_PASSWORD_BYTES));
  }

  if password != password_confirmation {
    return Err("passwords do not match".to_string());
  }

  Ok(())
}
