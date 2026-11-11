use sqlx::{Executor, MySql};
use tokens::tokens::users::UserToken;

pub struct ListLiveSessionTokensArgs<'a, T> {
  pub user_token: &'a UserToken,
  pub mysql_executor: T,
}

/// Returns the tokens of the user's live sessions, so callers can purge their cache entries after revoking.
pub async fn list_live_session_tokens<'c, T>(args: ListLiveSessionTokensArgs<'_, T>) -> Result<Vec<String>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  // NB: runtime (non-macro) query so no offline sqlx metadata is needed.
  sqlx::query_scalar::<_, String>("SELECT token FROM user_sessions WHERE user_token = ? AND deleted_at IS NULL")
      .bind(args.user_token.as_str())
      .fetch_all(args.mysql_executor)
      .await
}
