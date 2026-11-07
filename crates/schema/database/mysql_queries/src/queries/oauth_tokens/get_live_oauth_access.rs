use sqlx::{Executor, MySql};

pub struct OauthAccessRow {
  pub user_token: String,
  pub scopes: String,
  pub audience: String,
}

pub struct GetAccessArgs<'a, T> {
  pub access_hash: &'a str,
  pub mysql_executor: T,
}

/// Live access-token row, or `None` when the token is missing, revoked, or past `access_expires_at`.
pub async fn get_live_oauth_access<'c, T>(args: GetAccessArgs<'_, T>) -> Result<Option<OauthAccessRow>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let row = sqlx::query!("SELECT user_token, scopes, audience FROM oauth_tokens WHERE access_hash = ? AND maybe_revoked_at IS NULL AND access_expires_at > UTC_TIMESTAMP()", args.access_hash,).fetch_optional(args.mysql_executor).await?;
  Ok(row.map(|row| OauthAccessRow { user_token: row.user_token, scopes: row.scopes, audience: row.audience }))
}
