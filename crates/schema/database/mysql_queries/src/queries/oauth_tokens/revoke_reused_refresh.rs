use sqlx::{Executor, MySql};

pub struct ReusedRefreshArgs<'a, T> {
  pub spent_refresh_hash: &'a str,
  pub client_token: &'a str,
  pub mysql_executor: T,
}

/// Revokes every live token the user gave this client, because one approval's spent refresh token was presented.
/// The join uses the spent token row, and cleanup keeps that row until the spent hashes that point at it expire.
/// A different client presenting the token revokes nothing. Returns whether anything was revoked.
pub async fn revoke_reused_refresh<'c, T>(args: ReusedRefreshArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("UPDATE oauth_tokens AS grant_tokens INNER JOIN oauth_tokens AS spent_tokens ON spent_tokens.user_token = grant_tokens.user_token AND spent_tokens.client_token = grant_tokens.client_token INNER JOIN oauth_spent_refresh_hashes ON oauth_spent_refresh_hashes.oauth_token_id = spent_tokens.id SET grant_tokens.maybe_revoked_at = UTC_TIMESTAMP() WHERE oauth_spent_refresh_hashes.refresh_hash = ? AND spent_tokens.client_token = ? AND grant_tokens.maybe_revoked_at IS NULL", args.spent_refresh_hash, args.client_token,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected() > 0)
}
