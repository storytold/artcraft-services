use enums::by_table::user_login_challenges::user_login_challenge_failure_type::UserLoginChallengeFailureType;
use enums::by_table::user_login_challenges::user_login_challenge_status::UserLoginChallengeStatus;
use sqlx::{Executor, MySql};

pub struct ExpireAbandonedChallengesArgs<T> {
  pub mysql_executor: T,
}

/// Bounded sweep also audits abandoned attempts that are never polled again.
pub async fn expire_abandoned_challenges<'c, T>(args: ExpireAbandonedChallengesArgs<T>) -> Result<u64, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!(
    r#"
UPDATE user_login_challenges SET status = ?, maybe_failure_type = ?, maybe_failed_at = expires_at
WHERE status IN (?, ?) AND expires_at <= NOW() ORDER BY expires_at LIMIT 1000
"#,
    UserLoginChallengeStatus::Failed.to_str(),
    UserLoginChallengeFailureType::Expired.to_str(),
    UserLoginChallengeStatus::Pending.to_str(),
    UserLoginChallengeStatus::Approved.to_str()
  )
    .execute(args.mysql_executor)
    .await?;
  Ok(result.rows_affected())
}
