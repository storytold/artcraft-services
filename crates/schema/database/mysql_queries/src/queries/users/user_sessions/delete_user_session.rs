use sqlx::MySqlPool;

use errors::AnyhowResult;

/// Revoke a session by soft-deleting it. An already-revoked or unknown
/// session is not an error; a database failure is, since the session may
/// still be valid.
pub async fn delete_user_session(session_token: &str, mysql_pool: &MySqlPool) -> AnyhowResult<()> {
  sqlx::query!(
        r#"
UPDATE user_sessions
SET deleted_at = CURRENT_TIMESTAMP()
WHERE
    token = ?
    AND deleted_at IS NULL
        "#,
        session_token.to_string(),
    )
      .execute(mysql_pool)
      .await?;

  Ok(())
}
