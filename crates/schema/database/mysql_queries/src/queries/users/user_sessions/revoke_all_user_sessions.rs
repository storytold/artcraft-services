use sqlx::{Executor, MySql};
use tokens::tokens::users::UserToken;

/// Returns the tokens of the user's live sessions, so callers can purge their cache entries after revoking.
pub async fn list_live_session_tokens<'e, 'c: 'e, E>(
  user_token: &UserToken,
  mysql_executor: E,
) -> Result<Vec<String>, sqlx::Error>
where
  E: 'e + Executor<'c, Database = MySql>,
{
  // NB: runtime (non-macro) query so no offline sqlx metadata is needed.
  sqlx::query_scalar::<_, String>("SELECT token FROM user_sessions WHERE user_token = ? AND deleted_at IS NULL")
      .bind(user_token.as_str())
      .fetch_all(mysql_executor)
      .await
}

/// Soft-deletes every live session of the user. Takes any executor so callers can run it inside a transaction.
pub async fn revoke_all_user_sessions<'e, 'c: 'e, E>(
  user_token: &UserToken,
  mysql_executor: E,
) -> Result<(), sqlx::Error>
where
  E: 'e + Executor<'c, Database = MySql>,
{
  sqlx::query!(
        r#"
UPDATE user_sessions
SET deleted_at = CURRENT_TIMESTAMP()
WHERE
  user_token = ?
  AND deleted_at IS NULL
        "#,
        user_token.as_str(),
    )
      .execute(mysql_executor)
      .await?;

  Ok(())
}
