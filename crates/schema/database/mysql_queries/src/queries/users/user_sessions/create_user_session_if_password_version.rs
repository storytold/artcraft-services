use enums::by_table::user_sessions::user_web_session_creation_type::UserWebSessionCreationType;
use log::info;
use sqlx::{Executor, MySql};
use tokens::tokens::user_sessions::UserSessionToken;
use tokens::tokens::users::UserToken;

/// Creates a password-login session only if `users.password_version` still equals the version the
/// caller verified the password against. The version check and the INSERT are one statement, and
/// the `FOR UPDATE` row lock orders it against a password reset (which updates that row, then
/// revokes sessions, in one transaction): either the session exists before the reset's revoke, or
/// the version has moved and no session is created. Returns `Ok(None)` when the version moved.
pub async fn create_user_session_if_password_version<'e, 'c: 'e, E>(
  user_token: &UserToken,
  expected_password_version: u32,
  ip_address: &str,
  mysql_executor: E,
) -> Result<Option<UserSessionToken>, sqlx::Error>
where
  E: 'e + Executor<'c, Database = MySql>,
{
  let session_token = UserSessionToken::generate();

  // NB: runtime (non-macro) query so no offline sqlx metadata is needed.
  let result = sqlx::query(
    r#"
INSERT INTO user_sessions (
  token,
  user_token,
  ip_address_creation,
  maybe_creation_type,
  expires_at
)
SELECT ?, token, ?, ?, NOW() + interval 1 year
FROM users
WHERE token = ?
  AND password_version = ?
FOR UPDATE
    "#)
    .bind(session_token.as_str())
    .bind(ip_address)
    .bind(UserWebSessionCreationType::DirectLogin.to_str())
    .bind(user_token.as_str())
    .bind(expected_password_version)
    .execute(mysql_executor)
    .await?;

  if result.rows_affected() == 0 {
    return Ok(None);
  }

  info!("Created session id: {}", result.last_insert_id());

  Ok(Some(session_token))
}
