use sqlx::{Executor, MySql};
use tokens::tokens::users::UserToken;

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
