use sqlx::{Executor, MySql};

pub struct ConsumeUnusedOauthCodesArgs<'a, T> {
  pub user_token: &'a str,
  pub client_token: &'a str,
  pub mysql_executor: T,
}

/// Marks every still-unused code for this user and client as used, so a revoke cannot be followed by a code exchange.
pub async fn consume_unused_oauth_codes<'c, T>(args: ConsumeUnusedOauthCodesArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!("UPDATE oauth_authorization_codes SET maybe_used_at = UTC_TIMESTAMP() WHERE user_token = ? AND client_token = ? AND maybe_used_at IS NULL", args.user_token, args.client_token,).execute(args.mysql_executor).await?;
  Ok(())
}
