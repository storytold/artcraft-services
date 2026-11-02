use sqlx::{Executor, MySql};

pub struct InsertTokenArgs<'a, T> {
  pub access_hash: &'a str,
  pub refresh_hash: &'a str,
  pub client_token: &'a str,
  pub user_token: &'a str,
  pub scopes: &'a str,
  pub audience: &'a str,
  pub access_ttl_seconds: i64,
  pub refresh_ttl_seconds: i64,
  pub mysql_executor: T,
}

pub async fn insert_oauth_token<'c, T>(args: InsertTokenArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!("INSERT INTO oauth_tokens SET access_hash = ?, refresh_hash = ?, client_token = ?, user_token = ?, scopes = ?, audience = ?, access_expires_at = TIMESTAMPADD(SECOND, ?, UTC_TIMESTAMP()), refresh_expires_at = TIMESTAMPADD(SECOND, ?, UTC_TIMESTAMP())", args.access_hash, args.refresh_hash, args.client_token, args.user_token, args.scopes, args.audience, args.access_ttl_seconds, args.refresh_ttl_seconds,).execute(args.mysql_executor).await?;
  Ok(())
}
