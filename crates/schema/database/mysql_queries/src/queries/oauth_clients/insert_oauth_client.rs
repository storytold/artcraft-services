use sqlx::{Executor, MySql};

pub struct InsertOauthClientArgs<'a, T> {
  pub token: &'a str,
  pub client_name: &'a str,
  pub owner_user_token: &'a str,
  pub mysql_executor: T,
}

pub async fn insert_oauth_client<'c, T>(args: InsertOauthClientArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!("INSERT INTO oauth_clients SET token = ?, client_name = ?, owner_user_token = ?", args.token, args.client_name, args.owner_user_token,).execute(args.mysql_executor).await?;
  Ok(())
}
