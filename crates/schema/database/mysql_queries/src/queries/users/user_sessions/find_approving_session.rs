use sqlx::{Executor, MySql};

use super::bridge_session::BridgeSession;

pub struct FindApprovingSessionArgs<'a, T> {
  pub session_token: &'a str,
  pub mysql_executor: T,
}

/// Fresh database lookup, not a cached session. Impersonated sessions cannot consent.
pub async fn find_approving_session<'c, T>(args: FindApprovingSessionArgs<'_, T>) -> Result<Option<BridgeSession>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query_as!(
    BridgeSession,
    r#"
SELECT s.id, s.token, s.user_token, u.username FROM user_sessions s JOIN users u ON u.token = s.user_token
WHERE s.token = ? AND s.deleted_at IS NULL AND s.expires_at > NOW()
  AND s.maybe_impersonation_user_token IS NULL AND u.is_banned = FALSE
  AND u.user_deleted_at IS NULL AND u.mod_deleted_at IS NULL
"#,
    args.session_token
  )
    .fetch_optional(args.mysql_executor)
    .await
}
