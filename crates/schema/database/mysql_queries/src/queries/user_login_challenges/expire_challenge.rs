use enums::by_table::user_login_challenges::user_login_challenge_failure_type::UserLoginChallengeFailureType;
use enums::by_table::user_login_challenges::user_login_challenge_status::UserLoginChallengeStatus;
use sqlx::{Executor, MySql};

pub struct ExpireChallengeArgs<T> {
  pub challenge_id: u64,
  pub mysql_executor: T,
}

/// Audit expiration of a pending/approved challenge in the caller's transaction.
pub async fn expire_challenge<'c, T>(args: ExpireChallengeArgs<T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!(
    r#"
UPDATE user_login_challenges SET status = ?, maybe_failure_type = ?, maybe_failed_at = expires_at
WHERE id = ? AND status IN (?, ?) AND expires_at <= NOW()
"#,
    UserLoginChallengeStatus::Failed.to_str(),
    UserLoginChallengeFailureType::Expired.to_str(),
    args.challenge_id,
    UserLoginChallengeStatus::Pending.to_str(),
    UserLoginChallengeStatus::Approved.to_str()
  )
    .execute(args.mysql_executor)
    .await?;
  Ok(())
}
