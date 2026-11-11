use sqlx::{Executor, MySql};
use tokens::tokens::users::UserToken;

pub struct RevokeAllUserSessionsArgs<'a, T> {
  pub user_token: &'a UserToken,
  pub mysql_executor: T,
}

/// Soft-deletes every live session of the user. Takes any executor so callers can run it inside a transaction.
pub async fn revoke_all_user_sessions<'c, T>(args: RevokeAllUserSessionsArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!(
        r#"
UPDATE user_sessions
SET deleted_at = CURRENT_TIMESTAMP()
WHERE
  user_token = ?
  AND deleted_at IS NULL
        "#,
        args.user_token.as_str(),
    )
      .execute(args.mysql_executor)
      .await?;

  Ok(())
}
