use sqlx::{Executor, MySql};

pub struct OauthRefreshRow {
  pub client_token: String,
  pub user_token: String,
  pub scopes: String,
  pub audience: String,
}

pub struct TakeRefreshArgs<'a, T> {
  pub refresh_hash: &'a str,
  pub mysql_executor: T,
}

/// Reads a live refresh token. The row is unchanged. `rotate_oauth_refresh` stores the presented hash as the previous hash.
pub async fn take_oauth_refresh<'c, T>(args: TakeRefreshArgs<'_, T>) -> Result<Option<OauthRefreshRow>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let row = sqlx::query!("SELECT client_token, user_token, scopes, audience FROM oauth_tokens WHERE refresh_hash = ? AND maybe_revoked_at IS NULL AND refresh_expires_at > UTC_TIMESTAMP()", args.refresh_hash,).fetch_optional(args.mysql_executor).await?;
  Ok(row.map(|row| OauthRefreshRow { client_token: row.client_token, user_token: row.user_token, scopes: row.scopes, audience: row.audience }))
}
