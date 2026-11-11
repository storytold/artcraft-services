//! A login that verified the password against an older `password_version` (a reset committed in
//! between) must not get a session. Needs the disposable local MySQL, like the other tests here.
use enums::by_table::user_sessions::user_web_session_creation_type::UserWebSessionCreationType;
use mysql_queries::queries::users::user_sessions::create_user_session_if_password_version::{create_user_session_if_password_version, CreateUserSessionIfPasswordVersionArgs};
use mysql_testing::fixtures::users::create_test_user;
use mysql_testing::isolated::IsolatedTestDatabase;

#[tokio::test]
async fn session_is_created_only_while_password_version_matches() {
  let db = IsolatedTestDatabase::create().await;
  let user = create_test_user(&db.pool).await.unwrap();
  let version: u32 = sqlx::query_scalar("SELECT password_version FROM users WHERE token = ?")
    .bind(user.user_token.as_str())
    .fetch_one(&db.pool)
    .await
    .unwrap();

  let ok = create_user_session_if_password_version(CreateUserSessionIfPasswordVersionArgs {
    user_token: &user.user_token,
    maybe_expected_password_version: Some(version),
    ip_address: "127.0.0.1",
    creation_type: UserWebSessionCreationType::DirectLogin,
    mysql_executor: &db.pool,
  }).await.unwrap();
  assert!(ok.is_some());

  // Simulates the reset committing after the login looked the user up.
  sqlx::query("UPDATE users SET password_version = password_version + 1 WHERE token = ?")
    .bind(user.user_token.as_str())
    .execute(&db.pool)
    .await
    .unwrap();

  let stale = create_user_session_if_password_version(CreateUserSessionIfPasswordVersionArgs {
    user_token: &user.user_token,
    maybe_expected_password_version: Some(version),
    ip_address: "127.0.0.1",
    creation_type: UserWebSessionCreationType::DirectLogin,
    mysql_executor: &db.pool,
  }).await.unwrap();
  assert!(stale.is_none());
  let live: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_sessions WHERE user_token = ? AND deleted_at IS NULL")
    .bind(user.user_token.as_str())
    .fetch_one(&db.pool)
    .await
    .unwrap();
  assert_eq!(live, 2); // fixture session + the first login only

  // No expected version (SSO): the session is created against whatever the current version is.
  let sso = create_user_session_if_password_version(CreateUserSessionIfPasswordVersionArgs {
    user_token: &user.user_token,
    maybe_expected_password_version: None,
    ip_address: "127.0.0.1",
    creation_type: UserWebSessionCreationType::DirectLogin,
    mysql_executor: &db.pool,
  }).await.unwrap();
  assert!(sso.is_some());
}
