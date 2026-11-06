use sqlx::{Executor, MySql};

pub struct InsertNonceArgs<'a, T> {
  pub nonce_hash: &'a str,
  pub user_token: &'a str,
  pub client_token: &'a str,
  pub request_hash: &'a str,
  pub ttl_seconds: i64,
  pub mysql_executor: T,
}

/// Stores the hash of a consent or revoke nonce bound to one user, client, and request.
pub async fn insert_oauth_nonce<'c, T>(args: InsertNonceArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!("INSERT INTO oauth_consent_nonces SET nonce_hash = ?, user_token = ?, client_token = ?, request_hash = ?, expires_at = TIMESTAMPADD(SECOND, ?, UTC_TIMESTAMP())", args.nonce_hash, args.user_token, args.client_token, args.request_hash, args.ttl_seconds,).execute(args.mysql_executor).await?;
  Ok(())
}
