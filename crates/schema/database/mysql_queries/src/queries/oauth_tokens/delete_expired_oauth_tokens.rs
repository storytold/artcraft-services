use sqlx::{Executor, MySql};

pub struct DeleteExpiredOauthTokensArgs<T> {
  pub limit: u64,
  pub mysql_executor: T,
}

/// Deletes a token row only after both expiries have passed and every spent hash that points at it has expired.
/// `revoke_reused_refresh` joins that row to revoke the user's other live tokens for the client.
/// A spent hash outlives `refresh_expires_at` by one access lifetime, so the row must stay through that window.
pub async fn delete_expired_oauth_tokens<'c, T>(args: DeleteExpiredOauthTokensArgs<T>) -> Result<u64, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("DELETE FROM oauth_tokens WHERE refresh_expires_at < UTC_TIMESTAMP() AND access_expires_at < UTC_TIMESTAMP() AND NOT EXISTS (SELECT 1 FROM oauth_spent_refresh_hashes WHERE oauth_spent_refresh_hashes.oauth_token_id = oauth_tokens.id AND oauth_spent_refresh_hashes.expires_at >= UTC_TIMESTAMP()) LIMIT ?", args.limit,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected())
}

/// Both expiries have passed, and no spent hash that points at this row is still unexpired.
#[cfg(test)]
fn token_row_may_be_deleted(access_expired: bool, refresh_expired: bool, spent_hash_still_unexpired: bool) -> bool {
  access_expired && refresh_expired && !spent_hash_still_unexpired
}

#[cfg(test)]
mod tests {
  use super::token_row_may_be_deleted;

  #[test]
  fn an_unexpired_spent_hash_keeps_the_token_row() {
    assert!(!token_row_may_be_deleted(true, true, true));
    assert!(token_row_may_be_deleted(true, true, false));
    assert!(!token_row_may_be_deleted(false, true, false));
    assert!(!token_row_may_be_deleted(true, false, false));
    assert!(!token_row_may_be_deleted(false, false, true));
    let source = include_str!("delete_expired_oauth_tokens.rs");
    let marker = "sqlx::query!(\"";
    let start = source.find(marker).expect("delete query");
    let rest = &source[start + marker.len()..];
    let sql = &rest[..rest.find("\",").expect("query end")];
    assert!(sql.contains("refresh_expires_at < UTC_TIMESTAMP()"));
    assert!(sql.contains("access_expires_at < UTC_TIMESTAMP()"));
    assert!(sql.contains("NOT EXISTS (SELECT 1 FROM oauth_spent_refresh_hashes WHERE oauth_spent_refresh_hashes.oauth_token_id = oauth_tokens.id AND oauth_spent_refresh_hashes.expires_at >= UTC_TIMESTAMP())"));
  }
}
