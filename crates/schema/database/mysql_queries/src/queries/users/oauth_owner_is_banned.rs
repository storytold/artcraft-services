use sqlx::{Executor, MySql};

pub struct OwnerBannedArgs<'a, T> {
  pub user_token: &'a str,
  pub mysql_executor: T,
}

/// `Ok(None)` when the user row is gone. `Ok(Some(true))` when the account is banned.
pub async fn oauth_owner_is_banned<'c, T>(args: OwnerBannedArgs<'_, T>) -> Result<Option<bool>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let row = sqlx::query!("SELECT is_banned FROM users WHERE token = ? LIMIT 1", args.user_token,).fetch_optional(args.mysql_executor).await?;
  Ok(row.map(|row| row.is_banned != 0))
}
