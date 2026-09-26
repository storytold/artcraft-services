//! Mutators/inspection for disposable bridge integration databases only.
use sqlx::{MySqlPool, Row};
use chrono::{DateTime, Utc};
use enums::by_table::user_login_challenges::user_login_challenge_failure_type::UserLoginChallengeFailureType;
use enums::by_table::user_login_challenges::user_login_challenge_status::UserLoginChallengeStatus;
use enums::by_table::user_sessions::user_web_session_creation_type::UserWebSessionCreationType;
use enums::by_table::users::user_feature_flag::UserFeatureFlag;
use tokens::tokens::user_login_challenges::UserLoginChallengeToken;
use tokens::tokens::users::UserToken;

use super::users::{create_test_user, TestUser};

pub struct ChallengeAudit {
  pub status: UserLoginChallengeStatus,
  pub failure: Option<UserLoginChallengeFailureType>,
  pub creation_ip: String,
  pub failure_ip: Option<String>,
  pub session_id: Option<i64>,
  pub lifetime_seconds: i64,
  pub deciding_user: Option<String>,
  pub decision_ip: Option<String>,
  pub redemption_ip: Option<String>,
  pub decided_at: Option<DateTime<Utc>>,
  pub redeemed_at: Option<DateTime<Utc>>,
  pub failed_at: Option<DateTime<Utc>>,
}

pub async fn create_passwordless_user(pool: &MySqlPool, flags: &[UserFeatureFlag]) -> TestUser {
  let user = create_test_user(pool).await.expect("create passwordless fixture");
  set_passwordless(pool, &user.user_token).await;
  let flags = flags.iter().map(UserFeatureFlag::to_str).collect::<Vec<_>>().join(",");
  sqlx::query("UPDATE users SET maybe_feature_flags = ? WHERE token = ?")
    .bind(flags).bind(user.user_token.as_str()).execute(pool).await.expect("set fixture feature flags");
  user
}

pub async fn audit(pool: &MySqlPool, device_hash: &[u8]) -> ChallengeAudit {
  let row = sqlx::query("SELECT status, maybe_failure_type, ip_address_creation, maybe_ip_address_failure, maybe_redeemed_user_session_id, TIMESTAMPDIFF(SECOND, created_at, expires_at) AS lifetime, maybe_deciding_user_token, maybe_ip_address_decision, maybe_ip_address_redemption, maybe_decided_at, maybe_redeemed_at, maybe_failed_at FROM user_login_challenges WHERE device_token_sha256 = ?").bind(device_hash).fetch_one(pool).await.expect("challenge audit");
  ChallengeAudit {
    status: row.get("status"), failure: row.get("maybe_failure_type"), creation_ip: row.get("ip_address_creation"),
    failure_ip: row.get("maybe_ip_address_failure"), session_id: row.get("maybe_redeemed_user_session_id"), lifetime_seconds: row.get("lifetime"),
    deciding_user: row.get("maybe_deciding_user_token"), decision_ip: row.get("maybe_ip_address_decision"),
    redemption_ip: row.get("maybe_ip_address_redemption"), decided_at: row.get("maybe_decided_at"),
    redeemed_at: row.get("maybe_redeemed_at"), failed_at: row.get("maybe_failed_at"),
  }
}

pub async fn bridge_session_count(pool: &MySqlPool) -> i64 {
  sqlx::query_scalar("SELECT COUNT(*) FROM user_sessions WHERE maybe_creation_type = ?")
    .bind(UserWebSessionCreationType::DeviceApproval.to_str()).fetch_one(pool).await.expect("bridge session count")
}

pub async fn assert_challenge_status_is_explicit(pool: &MySqlPool) {
  let column = sqlx::query("SELECT COLUMN_DEFAULT, IS_NULLABLE FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'user_login_challenges' AND COLUMN_NAME = 'status'")
    .fetch_one(pool).await.expect("inspect migrated status column");
  assert!(column.get::<Option<String>, _>("COLUMN_DEFAULT").is_none());
  assert_eq!(column.get::<String, _>("IS_NULLABLE"), "NO");

  // Every other required field is supplied; MySQL must reject the omitted status.
  let token = UserLoginChallengeToken::generate();
  let error = sqlx::query("INSERT INTO user_login_challenges (token, approval_token_sha256, device_token_sha256, confirmation_code, ip_address_creation, expires_at) VALUES (?, ?, ?, ?, ?, NOW() + INTERVAL 20 MINUTE)")
    .bind(token.as_str()).bind(&[0u8; 32][..]).bind(&[1u8; 32][..])
    .bind("BCDFGHJK").bind("192.0.2.1").execute(pool).await.expect_err("status must be explicit");
  let error = error.as_database_error().expect("MySQL missing-column error");
  assert_eq!(error.try_downcast_ref::<sqlx::mysql::MySqlDatabaseError>().unwrap().number(), 1364);
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
