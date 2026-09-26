use chrono::{DateTime, Utc};
use sqlx::{MySqlConnection, MySqlPool};
use tokens::tokens::user_login_challenges::UserLoginChallengeToken;

pub struct LoginChallenge {
  pub id: u64,
  pub confirmation_code: String,
  pub status: String,
  pub maybe_deciding_user_token: Option<String>,
  pub maybe_redeemed_user_session_id: Option<i64>,
  pub maybe_failure_type: Option<String>,
  pub ip_address_creation: String,
  pub expires_at: DateTime<Utc>,
}

pub struct CreateChallengeArgs<'a> {
  pub token: &'a UserLoginChallengeToken,
  pub approval_hash: &'a [u8],
  pub device_hash: &'a [u8],
  pub confirmation_code: &'a str,
  pub ip_address: &'a str,
}

pub async fn create_challenge(args: CreateChallengeArgs<'_>, conn: &mut MySqlConnection) -> Result<(), sqlx::Error> {
  sqlx::query!(
    r#"
INSERT INTO user_login_challenges
  (token, approval_token_sha256, device_token_sha256, confirmation_code, ip_address_creation, created_at, expires_at)
VALUES (?, ?, ?, ?, ?, NOW(), NOW() + INTERVAL 20 MINUTE)
"#,
    args.token.as_str(),
    args.approval_hash,
    args.device_hash,
    args.confirmation_code,
    args.ip_address
  )
  .execute(conn)
  .await?;
  Ok(())
}

/// Call inside a transaction. The approval token cannot be used in device lookups.
pub async fn lock_by_approval(hash: &[u8], conn: &mut MySqlConnection) -> Result<Option<LoginChallenge>, sqlx::Error> {
  sqlx::query_as!(
    LoginChallenge,
    r#"
SELECT id, confirmation_code, status, maybe_deciding_user_token, maybe_redeemed_user_session_id,
  maybe_failure_type, ip_address_creation, expires_at
FROM user_login_challenges WHERE approval_token_sha256 = ? FOR UPDATE
"#,
    hash
  )
  .fetch_optional(conn)
  .await
}

pub async fn lock_by_device(hash: &[u8], conn: &mut MySqlConnection) -> Result<Option<LoginChallenge>, sqlx::Error> {
  sqlx::query_as!(
    LoginChallenge,
    r#"
SELECT id, confirmation_code, status, maybe_deciding_user_token, maybe_redeemed_user_session_id,
  maybe_failure_type, ip_address_creation, expires_at
FROM user_login_challenges WHERE device_token_sha256 = ? FOR UPDATE
"#,
    hash
  )
  .fetch_optional(conn)
  .await
}

/// Run AFTER acquiring the row lock: NOW() on the locking statement can precede
/// time spent waiting for another transaction. Never extend the original deadline.
pub async fn expire_locked_challenge(challenge: &mut LoginChallenge, conn: &mut MySqlConnection) -> Result<bool, sqlx::Error> {
  let expired = sqlx::query!(
    r#"
SELECT expires_at <= NOW() AS expired FROM user_login_challenges WHERE id = ?
"#,
    challenge.id
  )
  .fetch_one(&mut *conn)
  .await?
  .expired
    != 0;
  if expired && (challenge.status == "pending" || challenge.status == "approved") {
    sqlx::query!(
      r#"
UPDATE user_login_challenges SET status = 'failed', maybe_failure_type = 'expired', maybe_failed_at = expires_at
WHERE id = ? AND status IN ('pending', 'approved') AND expires_at <= NOW()
"#,
      challenge.id
    )
    .execute(conn)
    .await?;
    challenge.status = "failed".into();
    challenge.maybe_failure_type = Some("expired".into());
  }
  Ok(expired)
}

pub async fn decide_challenge(id: u64, user: &str, ip: &str, approve: bool, conn: &mut MySqlConnection) -> Result<bool, sqlx::Error> {
  let status = if approve { "approved" } else { "failed" };
  let failure = if approve { None } else { Some("user_declined") };
  let failure_ip = if approve { None } else { Some(ip) };
  let affected = sqlx::query!(
    r#"
UPDATE user_login_challenges SET status = ?, maybe_deciding_user_token = ?,
  maybe_ip_address_decision = ?, maybe_decided_at = NOW(), maybe_failure_type = ?,
  maybe_ip_address_failure = ?, maybe_failed_at = IF(?, NULL, NOW())
WHERE id = ? AND status = 'pending' AND expires_at > NOW()
"#,
    status,
    user,
    ip,
    failure,
    failure_ip,
    approve,
    id
  )
  .execute(conn)
  .await?
  .rows_affected();
  Ok(affected == 1)
}

pub async fn mark_redeemed(id: u64, session_id: i64, ip: &str, conn: &mut MySqlConnection) -> Result<bool, sqlx::Error> {
  let affected = sqlx::query!(
    r#"
UPDATE user_login_challenges SET status = 'redeemed', maybe_redeemed_user_session_id = ?,
  maybe_ip_address_redemption = ?, maybe_redeemed_at = NOW()
WHERE id = ? AND status = 'approved' AND expires_at > NOW() AND maybe_redeemed_user_session_id IS NULL
"#,
    session_id,
    ip,
    id
  )
  .execute(conn)
  .await?
  .rows_affected();
  Ok(affected == 1)
}

/// Bounded sweep also audits abandoned attempts that are never polled again.
pub async fn expire_abandoned_challenges(pool: &MySqlPool) -> Result<u64, sqlx::Error> {
  Ok(
    sqlx::query!(
      r#"
UPDATE user_login_challenges SET status = 'failed', maybe_failure_type = 'expired', maybe_failed_at = expires_at
WHERE status IN ('pending', 'approved') AND expires_at <= NOW() ORDER BY expires_at LIMIT 1000
"#
    )
    .execute(pool)
    .await?
    .rows_affected(),
  )
}
