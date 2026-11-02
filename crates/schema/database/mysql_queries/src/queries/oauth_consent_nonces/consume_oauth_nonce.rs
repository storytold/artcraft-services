use sqlx::{Executor, MySql};

/// Marks one live nonce used. `true` only for the first caller with that hash and request.
pub struct ConsumeNonceArgs<'a, T> {
  pub nonce_hash: &'a str,
  pub user_token: &'a str,
  pub client_token: &'a str,
  pub request_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn consume_oauth_nonce<'c, T>(args: ConsumeNonceArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("UPDATE oauth_consent_nonces SET maybe_used_at = UTC_TIMESTAMP() WHERE nonce_hash = ? AND user_token = ? AND client_token = ? AND request_hash = ? AND maybe_used_at IS NULL AND expires_at > UTC_TIMESTAMP()", args.nonce_hash, args.user_token, args.client_token, args.request_hash,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected() == 1)
}
