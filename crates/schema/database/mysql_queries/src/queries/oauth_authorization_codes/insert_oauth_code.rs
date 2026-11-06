use sqlx::{Executor, MySql};

pub struct InsertCodeArgs<'a, T> {
  pub code_hash: &'a str,
  pub client_token: &'a str,
  pub user_token: &'a str,
  pub redirect_uri: &'a str,
  pub scopes: &'a str,
  pub code_challenge: &'a str,
  pub audience: &'a str,
  pub ttl_seconds: i64,
  pub mysql_executor: T,
}

/// Stores the hash of a new authorization code together with its PKCE challenge and audience.
pub async fn insert_oauth_code<'c, T>(args: InsertCodeArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!("INSERT INTO oauth_authorization_codes SET code_hash = ?, client_token = ?, user_token = ?, redirect_uri = ?, scopes = ?, code_challenge = ?, audience = ?, expires_at = TIMESTAMPADD(SECOND, ?, UTC_TIMESTAMP())", args.code_hash, args.client_token, args.user_token, args.redirect_uri, args.scopes, args.code_challenge, args.audience, args.ttl_seconds,).execute(args.mysql_executor).await?;
  Ok(())
}
