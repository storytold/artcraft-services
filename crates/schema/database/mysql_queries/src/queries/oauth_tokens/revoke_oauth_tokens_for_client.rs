use sqlx::{Executor, MySql};

pub struct RevokeOauthTokensForClientArgs<'a, T> {
  pub user_token: &'a str,
  pub client_token: &'a str,
  pub mysql_executor: T,
}

/// Revokes every live token this user issued to this client.
pub async fn revoke_oauth_tokens_for_client<'c, T>(args: RevokeOauthTokensForClientArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!("UPDATE oauth_tokens SET maybe_revoked_at = UTC_TIMESTAMP() WHERE user_token = ? AND client_token = ? AND maybe_revoked_at IS NULL", args.user_token, args.client_token,).execute(args.mysql_executor).await?;
  Ok(())
}
