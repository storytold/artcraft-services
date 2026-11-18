use sqlx::{Executor, MySql};

pub struct RevokeGrantArgs<'a, T> {
  pub user_token: &'a str,
  pub client_token: &'a str,
  pub mysql_executor: T,
}

/// Marks the consent row revoked. A later approval can clear that timestamp.
pub async fn revoke_oauth_grant<'c, T>(args: RevokeGrantArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!("UPDATE oauth_consents SET maybe_revoked_at = UTC_TIMESTAMP() WHERE user_token = ? AND client_token = ? AND maybe_revoked_at IS NULL", args.user_token, args.client_token,).execute(args.mysql_executor).await?;
  Ok(())
}
