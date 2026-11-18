use sqlx::{Executor, MySql};

pub struct DeleteExpiredSpentRefreshesArgs<T> {
  pub limit: u64,
  pub mysql_executor: T,
}

/// Deletes spent hashes once no access token issued from their grant's rotations can still be live.
pub async fn delete_expired_spent_refreshes<'c, T>(args: DeleteExpiredSpentRefreshesArgs<T>) -> Result<u64, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("DELETE FROM oauth_spent_refresh_hashes WHERE expires_at < UTC_TIMESTAMP() LIMIT ?", args.limit,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected())
}
