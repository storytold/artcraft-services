use sqlx::{Executor, MySql};

pub struct OauthClientRow {
  pub client_name: String,
}

pub struct GetOauthClientArgs<'a, T> {
  pub token: &'a str,
  pub mysql_executor: T,
}

/// Live client name, or `None` when the client is missing or deleted.
pub async fn get_oauth_client<'c, T>(args: GetOauthClientArgs<'_, T>) -> Result<Option<OauthClientRow>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let row = sqlx::query!("SELECT client_name FROM oauth_clients WHERE token = ? AND maybe_deleted_at IS NULL", args.token,).fetch_optional(args.mysql_executor).await?;
  Ok(row.map(|row| OauthClientRow { client_name: row.client_name }))
}
