use sqlx::{Executor, MySql};

pub struct ReusedRefreshArgs<'a, T> {
  pub previous_refresh_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn revoke_reused_refresh<'c, T>(args: ReusedRefreshArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("UPDATE oauth_tokens SET maybe_revoked_at = UTC_TIMESTAMP() WHERE maybe_previous_refresh_hash = ? AND maybe_revoked_at IS NULL", args.previous_refresh_hash,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected() == 1)
}
