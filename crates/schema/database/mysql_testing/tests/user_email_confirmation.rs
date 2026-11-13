//! `get_user_email_confirmation` reports both confirmation flags of one user, and nothing for an
//! unknown token. Needs the disposable local MySQL, like the other tests here.
use mysql_queries::queries::users::user::get::get_user_email_confirmation::{get_user_email_confirmation, GetUserEmailConfirmationArgs, UserEmailConfirmation};
use mysql_testing::fixtures::users::create_test_user;
use mysql_testing::isolated::IsolatedTestDatabase;
use tokens::tokens::users::UserToken;

#[tokio::test]
async fn email_confirmation_is_read_per_user() {
  let db = IsolatedTestDatabase::create().await;
  let user = create_test_user(&db.pool).await.unwrap();

  let unconfirmed = get_user_email_confirmation(GetUserEmailConfirmationArgs {
    user_token: &user.user_token,
    mysql_executor: &db.pool,
  }).await.unwrap();
  assert_eq!(unconfirmed, Some(UserEmailConfirmation { email_confirmed: false, email_confirmed_by_google: false }));

  sqlx::query("UPDATE users SET email_confirmed = TRUE WHERE token = ?")
    .bind(user.user_token.as_str())
    .execute(&db.pool)
    .await
    .unwrap();

  let confirmed = get_user_email_confirmation(GetUserEmailConfirmationArgs {
    user_token: &user.user_token,
    mysql_executor: &db.pool,
  }).await.unwrap();
  assert_eq!(confirmed, Some(UserEmailConfirmation { email_confirmed: true, email_confirmed_by_google: false }));

  let unknown = get_user_email_confirmation(GetUserEmailConfirmationArgs {
    user_token: &UserToken::generate(),
    mysql_executor: &db.pool,
  }).await.unwrap();
  assert_eq!(unknown, None);

  db.destroy().await;
}
