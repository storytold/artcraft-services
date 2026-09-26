use sqlx::{Executor, MySql};

use super::login_challenge::LoginChallenge;

pub struct LockByDeviceArgs<'a, T> {
  pub device_hash: &'a [u8],
  pub mysql_executor: T,
}

/// Use the caller's transaction to retain the row lock throughout redemption.
pub async fn lock_by_device<'c, T>(args: LockByDeviceArgs<'_, T>) -> Result<Option<LoginChallenge>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query_as!(
    LoginChallenge,
    r#"
SELECT id, confirmation_code, status, maybe_deciding_user_token, maybe_redeemed_user_session_id,
  maybe_failure_type, ip_address_creation, expires_at
FROM user_login_challenges WHERE device_token_sha256 = ? FOR UPDATE
"#,
    args.device_hash
  )
    .fetch_optional(args.mysql_executor)
    .await
}
