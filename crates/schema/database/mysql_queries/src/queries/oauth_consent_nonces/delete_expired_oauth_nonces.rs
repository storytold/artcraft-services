use sqlx::{Executor, MySql};

pub struct DeleteExpiredOauthNoncesArgs<T> {
  pub limit: u64,
  pub mysql_executor: T,
}

/// Deletes nonces that expired over 7 days ago.
pub async fn delete_expired_oauth_nonces<'c, T>(args: DeleteExpiredOauthNoncesArgs<T>) -> Result<u64, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("DELETE FROM oauth_consent_nonces WHERE expires_at < TIMESTAMPADD(DAY, -7, UTC_TIMESTAMP()) LIMIT ?", args.limit,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected())
}
