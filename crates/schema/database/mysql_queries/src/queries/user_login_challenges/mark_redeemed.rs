use enums::by_table::user_login_challenges::user_login_challenge_status::UserLoginChallengeStatus;
use sqlx::{Executor, MySql};

pub struct MarkRedeemedArgs<'a, T> {
  pub challenge_id: u64,
  pub session_id: i64,
  pub ip_address: &'a str,
  pub mysql_executor: T,
}

pub async fn mark_redeemed<'c, T>(args: MarkRedeemedArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let affected = sqlx::query!(
    r#"
UPDATE user_login_challenges SET status = ?, maybe_redeemed_user_session_id = ?,
  maybe_ip_address_redemption = ?, maybe_redeemed_at = NOW()
WHERE id = ? AND status = ? AND expires_at > NOW() AND maybe_redeemed_user_session_id IS NULL
"#,
    UserLoginChallengeStatus::Redeemed.to_str(),
    args.session_id,
    args.ip_address,
    args.challenge_id,
    UserLoginChallengeStatus::Approved.to_str()
  )
    .execute(args.mysql_executor)
    .await?
    .rows_affected();
  Ok(affected == 1)
}
