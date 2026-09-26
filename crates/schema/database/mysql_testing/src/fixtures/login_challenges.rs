//! Mutators/inspection for disposable bridge integration databases only.
use sqlx::{MySqlPool, Row};
use tokens::tokens::users::UserToken;

pub struct ChallengeAudit {
  pub status: String,
  pub failure: Option<String>,
  pub creation_ip: String,
  pub failure_ip: Option<String>,
  pub session_id: Option<i64>,
  pub lifetime_seconds: i64,
}

pub async fn audit(pool: &MySqlPool, device_hash: &[u8]) -> ChallengeAudit {
  let row = sqlx::query("SELECT status, maybe_failure_type, ip_address_creation, maybe_ip_address_failure, maybe_redeemed_user_session_id, TIMESTAMPDIFF(SECOND, created_at, expires_at) AS lifetime FROM user_login_challenges WHERE device_token_sha256 = ?").bind(device_hash).fetch_one(pool).await.expect("challenge audit");
  ChallengeAudit { status: row.get("status"), failure: row.get("maybe_failure_type"), creation_ip: row.get("ip_address_creation"), failure_ip: row.get("maybe_ip_address_failure"), session_id: row.get("maybe_redeemed_user_session_id"), lifetime_seconds: row.get("lifetime") }
}

pub async fn bridge_session_count(pool: &MySqlPool) -> i64 {
  sqlx::query_scalar("SELECT COUNT(*) FROM user_sessions WHERE maybe_creation_type = 'device_approval'").fetch_one(pool).await.expect("bridge session count")
}

pub async fn set_challenge_deadline(pool: &MySqlPool, device_hash: &[u8], seconds_from_now: i32) {
  sqlx::query("UPDATE user_login_challenges SET expires_at = TIMESTAMPADD(SECOND, ?, NOW()) WHERE device_token_sha256 = ?").bind(seconds_from_now).bind(device_hash).execute(pool).await.expect("change test deadline");
}

pub async fn expire_session(pool: &MySqlPool, token: &str) {
  sqlx::query("UPDATE user_sessions SET expires_at = NOW() - INTERVAL 1 SECOND WHERE token = ?").bind(token).execute(pool).await.expect("expire test session");
}

pub async fn revoke_session(pool: &MySqlPool, token: &str) {
  sqlx::query("UPDATE user_sessions SET deleted_at = NOW() WHERE token = ?").bind(token).execute(pool).await.expect("revoke test session");
}

pub async fn set_passwordless(pool: &MySqlPool, user: &UserToken) {
  sqlx::query("UPDATE users SET is_without_password = TRUE, email_confirmed_by_google = TRUE WHERE token = ?").bind(user.as_str()).execute(pool).await.expect("passwordless test account");
}

pub async fn ban_user(pool: &MySqlPool, user: &UserToken) {
  sqlx::query("UPDATE users SET is_banned = TRUE WHERE token = ?").bind(user.as_str()).execute(pool).await.expect("ban test account");
}

pub async fn impersonate_session(pool: &MySqlPool, token: &str, actor: &UserToken) {
  sqlx::query("UPDATE user_sessions SET maybe_impersonation_user_token = ? WHERE token = ?").bind(actor.as_str()).bind(token).execute(pool).await.expect("test impersonation");
}
