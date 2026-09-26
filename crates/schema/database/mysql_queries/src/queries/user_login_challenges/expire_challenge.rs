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
UPDATE user_login_challenges SET status = 'failed', maybe_failure_type = 'expired', maybe_failed_at = expires_at
WHERE id = ? AND status IN ('pending', 'approved') AND expires_at <= NOW()
"#,
    args.challenge_id
  )
    .execute(args.mysql_executor)
    .await?;
  Ok(())
}
