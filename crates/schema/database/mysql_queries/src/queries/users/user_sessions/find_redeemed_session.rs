use enums::by_table::user_sessions::user_web_session_creation_type::UserWebSessionCreationType;
use sqlx::{Executor, MySql};

use super::bridge_session::BridgeSession;

pub struct FindRedeemedSessionArgs<'a, T> {
  pub session_id: i64,
  pub user_token: &'a str,
  pub mysql_executor: T,
}

/// Returns the one previously minted session, only while it and its owner are live.
pub async fn find_redeemed_session<'c, T>(args: FindRedeemedSessionArgs<'_, T>) -> Result<Option<BridgeSession>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query_as!(
    BridgeSession,
    r#"
SELECT s.id, s.token, s.user_token, u.username FROM user_sessions s JOIN users u ON u.token = s.user_token
WHERE s.id = ? AND s.user_token = ? AND s.maybe_creation_type = ?
  AND s.deleted_at IS NULL AND s.expires_at > NOW() AND s.maybe_impersonation_user_token IS NULL
  AND u.is_banned = FALSE AND u.user_deleted_at IS NULL AND u.mod_deleted_at IS NULL
"#,
    args.session_id,
    args.user_token,
    UserWebSessionCreationType::DeviceApproval.to_str()
  )
    .fetch_optional(args.mysql_executor)
    .await
}
