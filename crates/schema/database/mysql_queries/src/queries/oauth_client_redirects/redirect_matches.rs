use sqlx::{Executor, MySql};

pub struct RedirectMatchesArgs<'a, T> {
  pub client_token: &'a str,
  pub redirect_uri: &'a str,
  pub mysql_executor: T,
}

pub async fn redirect_matches<'c, T>(args: RedirectMatchesArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let row = sqlx::query!("SELECT client_token FROM oauth_client_redirects WHERE client_token = ? AND redirect_uri = ? LIMIT 1", args.client_token, args.redirect_uri,).fetch_optional(args.mysql_executor).await?;
  Ok(row.is_some())
}
