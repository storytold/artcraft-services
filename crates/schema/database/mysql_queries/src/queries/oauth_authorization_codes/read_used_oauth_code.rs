use sqlx::{Executor, MySql};

pub struct OauthCodeRow {
  pub client_token: String,
  pub user_token: String,
  pub redirect_uri: String,
  pub scopes: String,
  pub code_challenge: String,
  pub audience: String,
}

pub struct ReadCodeArgs<'a, T> {
  pub code_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn read_used_oauth_code<'c, T>(args: ReadCodeArgs<'_, T>) -> Result<Option<OauthCodeRow>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let row = sqlx::query!("SELECT client_token, user_token, redirect_uri, scopes, code_challenge, audience FROM oauth_authorization_codes WHERE code_hash = ? AND maybe_used_at IS NOT NULL", args.code_hash,).fetch_optional(args.mysql_executor).await?;
  Ok(row.map(|row| OauthCodeRow { client_token: row.client_token, user_token: row.user_token, redirect_uri: row.redirect_uri, scopes: row.scopes, code_challenge: row.code_challenge, audience: row.audience }))
}
