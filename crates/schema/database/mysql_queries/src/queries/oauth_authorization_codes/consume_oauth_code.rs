use sqlx::{Executor, MySql};

pub struct ConsumeCodeArgs<'a, T> {
  pub code_hash: &'a str,
  pub mysql_executor: T,
}

/// Marks one unexpired, unused authorization code as used. `true` only for the first caller.
pub async fn consume_oauth_code<'c, T>(args: ConsumeCodeArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!("UPDATE oauth_authorization_codes SET maybe_used_at = UTC_TIMESTAMP() WHERE code_hash = ? AND maybe_used_at IS NULL AND expires_at > UTC_TIMESTAMP()", args.code_hash,).execute(args.mysql_executor).await?;
  Ok(result.rows_affected() == 1)
}
