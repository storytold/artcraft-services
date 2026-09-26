use sqlx::{Executor, MySql};

pub struct IsChallengeExpiredArgs<T> {
  pub challenge_id: u64,
  pub mysql_executor: T,
}

/// Run AFTER acquiring the row lock: NOW() on the locking statement can precede
/// time spent waiting for another transaction. Never extend the original deadline.
pub async fn is_challenge_expired<'c, T>(args: IsChallengeExpiredArgs<T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let result = sqlx::query!(
    r#"
SELECT expires_at <= NOW() AS expired FROM user_login_challenges WHERE id = ?
"#,
    args.challenge_id
  )
    .fetch_one(args.mysql_executor)
    .await?;
  Ok(result.expired != 0)
}
