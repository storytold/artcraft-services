use sqlx::MySqlConnection;
use tokens::tokens::user_sessions::UserSessionToken;

pub struct BridgeSession {
  pub id: i64,
  pub token: String,
  pub user_token: String,
  pub username: String,
}

/// Fresh database lookup, not a cached session. Impersonated sessions cannot consent.
pub async fn find_approving_session(token: &str, conn: &mut MySqlConnection) -> Result<Option<BridgeSession>, sqlx::Error> {
  sqlx::query_as!(
    BridgeSession,
    r#"
SELECT s.id, s.token, s.user_token, u.username FROM user_sessions s JOIN users u ON u.token = s.user_token
WHERE s.token = ? AND s.deleted_at IS NULL AND s.expires_at > NOW()
  AND s.maybe_impersonation_user_token IS NULL AND u.is_banned = FALSE
  AND u.user_deleted_at IS NULL AND u.mod_deleted_at IS NULL
"#,
    token
  )
  .fetch_optional(conn)
  .await
}

/// Returns the one previously minted session, only while it and its owner are live.
pub async fn find_redeemed_session(id: i64, user: &str, conn: &mut MySqlConnection) -> Result<Option<BridgeSession>, sqlx::Error> {
  sqlx::query_as!(
    BridgeSession,
    r#"
SELECT s.id, s.token, s.user_token, u.username FROM user_sessions s JOIN users u ON u.token = s.user_token
WHERE s.id = ? AND s.user_token = ? AND s.maybe_creation_type = 'device_approval'
  AND s.deleted_at IS NULL AND s.expires_at > NOW() AND s.maybe_impersonation_user_token IS NULL
  AND u.is_banned = FALSE AND u.user_deleted_at IS NULL AND u.mod_deleted_at IS NULL
"#,
    id,
    user
  )
  .fetch_optional(conn)
  .await
}

/// Must share the transaction holding the challenge lock. A failed/expired
/// redemption rolls back this INSERT; no session is minted when merely approving.
pub async fn create_bridge_session(user: &str, ip: &str, conn: &mut MySqlConnection) -> Result<Option<BridgeSession>, sqlx::Error> {
  let token = UserSessionToken::generate();
  let inserted = sqlx::query!(
    r#"
INSERT INTO user_sessions (token, user_token, ip_address_creation, maybe_creation_type, expires_at)
SELECT ?, token, ?, 'device_approval', NOW() + INTERVAL 1 YEAR FROM users
WHERE token = ? AND is_banned = FALSE AND user_deleted_at IS NULL AND mod_deleted_at IS NULL
"#,
    token.as_str(),
    ip,
    user
  )
  .execute(&mut *conn)
  .await?;
  if inserted.rows_affected() != 1 {
    return Ok(None);
  }
  find_redeemed_session(inserted.last_insert_id() as i64, user, conn).await
}
