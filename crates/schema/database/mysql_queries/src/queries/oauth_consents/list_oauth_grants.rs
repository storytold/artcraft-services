use sqlx::{Executor, MySql};

pub struct OauthGrantRow {
  pub client_token: String,
  pub client_name: String,
  pub scopes: String,
}

pub struct ListGrantsArgs<'a, T> {
  pub user_token: &'a str,
  pub mysql_executor: T,
}

pub async fn list_oauth_grants<'c, T>(args: ListGrantsArgs<'_, T>) -> Result<Vec<OauthGrantRow>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let rows = sqlx::query!("SELECT c.token AS client_token, c.client_name, s.scopes FROM oauth_consents s JOIN oauth_clients c ON c.token = s.client_token WHERE s.user_token = ? AND s.maybe_revoked_at IS NULL AND c.maybe_deleted_at IS NULL ORDER BY c.client_name", args.user_token,).fetch_all(args.mysql_executor).await?;
  Ok(rows.into_iter().map(|row| OauthGrantRow { client_token: row.client_token, client_name: row.client_name, scopes: row.scopes }).collect())
}
