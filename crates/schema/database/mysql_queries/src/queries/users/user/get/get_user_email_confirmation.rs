use sqlx::{Executor, MySql};

use tokens::tokens::users::UserToken;

/// Whether a user's email address was shown to belong to them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserEmailConfirmation {
  /// Confirmed by us (a password reset sent to the address was redeemed).
  pub email_confirmed: bool,

  /// Confirmed by Google when the account was created through Google sign-in.
  pub email_confirmed_by_google: bool,
}

pub struct GetUserEmailConfirmationArgs<'a, T> {
  pub user_token: &'a UserToken,
  pub mysql_executor: T,
}

pub async fn get_user_email_confirmation<'c, T>(
  args: GetUserEmailConfirmationArgs<'_, T>,
) -> Result<Option<UserEmailConfirmation>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query_as!(
    UserEmailConfirmation,
    r#"
SELECT
  email_confirmed as `email_confirmed: bool`,
  email_confirmed_by_google as `email_confirmed_by_google: bool`
FROM users
WHERE
  token = ?
LIMIT 1
    "#,
    args.user_token.as_str(),
  )
    .fetch_optional(args.mysql_executor)
    .await
}
