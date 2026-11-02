use sqlx::{Executor, MySql};

pub struct InsertOauthRedirectArgs<'a, T> {
  pub client_token: &'a str,
  pub redirect_uri: &'a str,
  pub redirect_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn insert_oauth_redirect<'c, T>(args: InsertOauthRedirectArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!("INSERT INTO oauth_client_redirects SET client_token = ?, redirect_uri = ?, redirect_hash = ?", args.client_token, args.redirect_uri, args.redirect_hash,).execute(args.mysql_executor).await?;
  Ok(())
}
