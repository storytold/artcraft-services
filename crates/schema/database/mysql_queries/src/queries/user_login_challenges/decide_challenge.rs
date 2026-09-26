use enums::by_table::user_login_challenges::user_login_challenge_failure_type::UserLoginChallengeFailureType;
use enums::by_table::user_login_challenges::user_login_challenge_status::UserLoginChallengeStatus;
use sqlx::{Executor, MySql};

pub struct DecideChallengeArgs<'a, T> {
  pub challenge_id: u64,
  pub user_token: &'a str,
  pub ip_address: &'a str,
  pub approve: bool,
  pub mysql_executor: T,
}

pub async fn decide_challenge<'c, T>(args: DecideChallengeArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let status = if args.approve { UserLoginChallengeStatus::Approved } else { UserLoginChallengeStatus::Failed };
  let failure = if args.approve { None } else { Some(UserLoginChallengeFailureType::UserDeclined) };
  let failure_ip = if args.approve { None } else { Some(args.ip_address) };
  let affected = sqlx::query!(
    r#"
UPDATE user_login_challenges SET status = ?, maybe_deciding_user_token = ?,
  maybe_ip_address_decision = ?, maybe_decided_at = NOW(), maybe_failure_type = ?,
  maybe_ip_address_failure = ?, maybe_failed_at = IF(?, NULL, NOW())
WHERE id = ? AND status = ? AND expires_at > NOW()
"#,
    status.to_str(),
    args.user_token,
    args.ip_address,
    failure.map(|value| value.to_str()),
    failure_ip,
    args.approve,
    args.challenge_id,
    UserLoginChallengeStatus::Pending.to_str()
  )
    .execute(args.mysql_executor)
    .await?
    .rows_affected();
  Ok(affected == 1)
}
