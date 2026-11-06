use sqlx::{Executor, MySql};

pub struct SaveConsentArgs<'a, T> {
  pub user_token: &'a str,
  pub client_token: &'a str,
  pub scopes: &'a str,
  pub mysql_executor: T,
}

/// Inserts or replaces this user's consent for the client and clears a previous revocation.
pub async fn save_oauth_consent<'c, T>(args: SaveConsentArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!("INSERT INTO oauth_consents SET user_token = ?, client_token = ?, scopes = ? ON DUPLICATE KEY UPDATE scopes = VALUES(scopes), maybe_revoked_at = NULL", args.user_token, args.client_token, args.scopes,).execute(args.mysql_executor).await?;
  Ok(())
}
