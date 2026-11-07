use anyhow::anyhow;
use sqlx::{MySql, Transaction};

use errors::AnyhowResult;
use crate::queries::users::user_sessions::revoke_all_user_sessions::revoke_all_user_sessions;
use tokens::tokens::password_reset::PasswordResetToken;
use tokens::tokens::users::UserToken;

pub struct ChangePasswordFromPasswordResetArgs<'a, 'b> {
  pub password_reset_token: &'a PasswordResetToken,
  pub user_token: &'a UserToken,
  pub new_password_hash: &'a str,
  pub ip_address: &'a str,
  pub mysql_transaction: Transaction<'b, MySql>,
}

pub async fn change_password_from_password_reset<'a, 'b>(
  mut args: ChangePasswordFromPasswordResetArgs<'a, 'b>
) -> AnyhowResult<()> {

  let query = sqlx::query!(
        r#"
UPDATE users
SET
  email_confirmed = true,
  email_is_synthetic = false,
  is_without_password = false,
  is_temporary = false,
  password_hash = ?,
  ip_address_last_update = ?,
  password_version = password_version + 1,
  version = version + 1
WHERE
  token = ?
LIMIT 1
        "#,
        args.new_password_hash,
        args.ip_address,
        args.user_token,
    );

  let query_result = query.execute(&mut *args.mysql_transaction).await;

  if let Err(err) = query_result {
    let _r = args.mysql_transaction.rollback().await.ok();
    return Err(anyhow!("Error updating user password from password reset request: {err}"));
  }

  let query = sqlx::query!(
        r#"
UPDATE user_password_resets
SET
  is_redeemed = true,
  ip_address_redemption = ?,
  version = version + 1
WHERE
  token = ?
LIMIT 1
        "#,
        args.ip_address,
        args.password_reset_token,
    );

  let query_result = query.execute(&mut *args.mysql_transaction).await;

  if let Err(err) = query_result {
    let _r = args.mysql_transaction.rollback().await.ok();
    return Err(anyhow!("Error updating user password from password reset request: {err}"));
  }

  // NB: Recovery must lock out whoever held the account before it, so revoke every existing session
  // in the same transaction. The handler issues a fresh session after commit.
  let query_result = revoke_all_user_sessions(args.user_token, &mut *args.mysql_transaction).await;

  if let Err(err) = query_result {
    let _r = args.mysql_transaction.rollback().await.ok();
    return Err(anyhow!("Error revoking sessions during password reset: {err}"));
  }

  match args.mysql_transaction.commit().await {
    Ok(_) => Ok(()),
    Err(err) => Err(anyhow!("Error committing password reset changes: {err}")),
  }
}
