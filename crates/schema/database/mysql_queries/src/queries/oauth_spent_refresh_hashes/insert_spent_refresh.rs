use sqlx::{Executor, MySql};

pub struct InsertSpentRefreshArgs<'a, T> {
  pub refresh_hash: &'a str,
  pub mysql_executor: T,
}

/// Records a refresh token as spent. Call this in the same transaction as `rotate_oauth_refresh`, before it.
pub async fn insert_spent_refresh<'c, T>(args: InsertSpentRefreshArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("INSERT INTO oauth_spent_refresh_hashes (refresh_hash, oauth_token_id, expires_at) SELECT refresh_hash, id, refresh_expires_at FROM oauth_tokens WHERE refresh_hash = ?", args.refresh_hash,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected() == 1)
}
