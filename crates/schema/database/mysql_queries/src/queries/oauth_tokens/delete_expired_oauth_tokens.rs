use sqlx::{Executor, MySql};

pub struct DeleteExpiredOauthTokensArgs<T> {
  pub limit: u64,
  pub mysql_executor: T,
}

/// Deletes token rows only after both the access and refresh expiries have passed.
/// A rotation near the end of the refresh window issues an access token that outlives
/// `refresh_expires_at`, and that token must stay valid until `access_expires_at`.
pub async fn delete_expired_oauth_tokens<'c, T>(args: DeleteExpiredOauthTokensArgs<T>) -> Result<u64, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("DELETE FROM oauth_tokens WHERE refresh_expires_at < UTC_TIMESTAMP() AND access_expires_at < UTC_TIMESTAMP() LIMIT ?", args.limit,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected())
}
