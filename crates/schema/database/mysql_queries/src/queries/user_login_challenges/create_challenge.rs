use sqlx::{Executor, MySql};
use tokens::tokens::user_login_challenges::UserLoginChallengeToken;

pub struct CreateChallengeArgs<'a, T> {
  pub token: &'a UserLoginChallengeToken,
  pub approval_hash: &'a [u8],
  pub device_hash: &'a [u8],
  pub confirmation_code: &'a str,
  pub ip_address: &'a str,
  pub mysql_executor: T,
}

pub async fn create_challenge<'c, T>(args: CreateChallengeArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
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
    .execute(args.mysql_executor)
    .await?;
  Ok(())
}
