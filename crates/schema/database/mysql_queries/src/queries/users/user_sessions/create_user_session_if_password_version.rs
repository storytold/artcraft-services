use enums::by_table::user_sessions::user_web_session_creation_type::UserWebSessionCreationType;
use log::info;
use sqlx::{Executor, MySql};
use tokens::tokens::user_sessions::UserSessionToken;
use tokens::tokens::users::UserToken;

pub struct CreateUserSessionIfPasswordVersionArgs<'a, T> {
  pub user_token: &'a UserToken,
  /// The `password_version` the caller verified a password against. `None` for paths that verify no
  /// password (SSO): the version is then read inside the statement, under the same row lock.
  pub maybe_expected_password_version: Option<u32>,
  pub ip_address: &'a str,
  pub creation_type: UserWebSessionCreationType,
  pub mysql_executor: T,
}

/// Creates a session only if `users.password_version` still equals the expected version (when one is
/// given). The version check and the INSERT are one statement, and the `FOR UPDATE` row lock orders
/// it against a password reset (which updates that row, then revokes sessions, in one transaction):
/// either the session exists before the reset's revoke, or it is created after the reset committed.
/// Returns `Ok(None)` when the user is missing or the version moved.
pub async fn create_user_session_if_password_version<'c, T>(
  args: CreateUserSessionIfPasswordVersionArgs<'_, T>,
) -> Result<Option<UserSessionToken>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
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
  AND (? IS NULL OR password_version = ?)
FOR UPDATE
    "#)
    .bind(session_token.as_str())
    .bind(args.ip_address)
    .bind(args.creation_type.to_str())
    .bind(args.user_token.as_str())
    .bind(args.maybe_expected_password_version)
    .bind(args.maybe_expected_password_version)
    .execute(args.mysql_executor)
    .await?;

  if result.rows_affected() == 0 {
    return Ok(None);
  }

  info!("Created session id: {}", result.last_insert_id());

  Ok(Some(session_token))
}
