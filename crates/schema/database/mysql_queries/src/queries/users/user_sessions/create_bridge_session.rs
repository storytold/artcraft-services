use enums::by_table::user_sessions::user_web_session_creation_type::UserWebSessionCreationType;
use sqlx::{Executor, MySql};
use tokens::tokens::user_sessions::UserSessionToken;

pub struct CreateBridgeSessionArgs<'a, T> {
  pub user_token: &'a str,
  pub ip_address: &'a str,
  pub mysql_executor: T,
}

/// Returns the inserted session ID, or None if the user cannot sign in.
/// Must share the transaction holding the challenge lock. A failed/expired
/// redemption rolls back this INSERT; no session is minted when merely approving.
pub async fn create_bridge_session<'c, T>(args: CreateBridgeSessionArgs<'_, T>) -> Result<Option<i64>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let token = UserSessionToken::generate();
  let inserted = sqlx::query!(
    r#"
INSERT INTO user_sessions (token, user_token, ip_address_creation, maybe_creation_type, expires_at)
SELECT ?, token, ?, ?, NOW() + INTERVAL 1 YEAR FROM users
WHERE token = ? AND is_banned = FALSE AND user_deleted_at IS NULL AND mod_deleted_at IS NULL
"#,
    token.as_str(),
    args.ip_address,
    UserWebSessionCreationType::DeviceApproval.to_str(),
    args.user_token
  )
    .execute(args.mysql_executor)
    .await?;
  if inserted.rows_affected() != 1 {
    return Ok(None);
  }
  Ok(Some(inserted.last_insert_id() as i64))
}
