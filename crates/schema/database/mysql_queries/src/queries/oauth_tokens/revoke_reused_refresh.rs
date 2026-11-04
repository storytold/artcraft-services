use sqlx::{Executor, MySql};

pub struct ReusedRefreshArgs<'a, T> {
  pub spent_refresh_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn revoke_reused_refresh<'c, T>(args: ReusedRefreshArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("UPDATE oauth_tokens INNER JOIN oauth_spent_refresh_hashes ON oauth_spent_refresh_hashes.oauth_token_id = oauth_tokens.id SET oauth_tokens.maybe_revoked_at = UTC_TIMESTAMP() WHERE oauth_spent_refresh_hashes.refresh_hash = ? AND oauth_tokens.maybe_revoked_at IS NULL", args.spent_refresh_hash,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected() == 1)
}
