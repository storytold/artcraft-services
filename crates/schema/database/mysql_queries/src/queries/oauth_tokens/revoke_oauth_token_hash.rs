use sqlx::{Executor, MySql};

pub struct RevokePresentedArgs<'a, T> {
  pub token_hash: &'a str,
  pub mysql_executor: T,
}

/// Revokes the row whose access or refresh hash matches. An unknown hash changes nothing.
pub async fn revoke_oauth_token_hash<'c, T>(args: RevokePresentedArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!("UPDATE oauth_tokens SET maybe_revoked_at = UTC_TIMESTAMP() WHERE maybe_revoked_at IS NULL AND (access_hash = ? OR refresh_hash = ?)", args.token_hash, args.token_hash,).execute(args.mysql_executor).await?;
  Ok(())
}
