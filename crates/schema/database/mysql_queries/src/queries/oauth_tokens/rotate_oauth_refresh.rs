use sqlx::{Executor, MySql};

pub struct RotateRefreshArgs<'a, T> {
  pub old_refresh_hash: &'a str,
  pub client_token: &'a str,
  pub new_access_hash: &'a str,
  pub new_refresh_hash: &'a str,
  pub access_ttl_seconds: i64,
  pub mysql_executor: T,
}

pub async fn rotate_oauth_refresh<'c, T>(args: RotateRefreshArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("UPDATE oauth_tokens SET refresh_hash = ?, access_hash = ?, access_expires_at = TIMESTAMPADD(SECOND, ?, UTC_TIMESTAMP()) WHERE refresh_hash = ? AND client_token = ? AND maybe_revoked_at IS NULL AND refresh_expires_at > UTC_TIMESTAMP()", args.new_refresh_hash, args.new_access_hash, args.access_ttl_seconds, args.old_refresh_hash, args.client_token,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected() == 1)
}
