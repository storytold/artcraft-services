use sqlx::{Executor, MySql};

pub struct DeleteExpiredOauthCodesArgs<T> {
  pub limit: u64,
  pub mysql_executor: T,
}

/// Deletes codes that expired over 7 days ago. Replaying a code older than that is refused
/// without revoking its tokens.
pub async fn delete_expired_oauth_codes<'c, T>(args: DeleteExpiredOauthCodesArgs<T>) -> Result<u64, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("DELETE FROM oauth_authorization_codes WHERE expires_at < TIMESTAMPADD(DAY, -7, UTC_TIMESTAMP()) LIMIT ?", args.limit,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected())
}
