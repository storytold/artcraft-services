use anyhow::anyhow;
use log::info;
use sqlx::MySqlPool;

use errors::AnyhowResult;
use tokens::tokens::user_sessions::UserSessionToken;

#[deprecated(note="use create_user_session_with_transactor")]
pub async fn create_user_session(user_token: &str, ip_address: &str, mysql_pool: &MySqlPool)
    -> AnyhowResult<String>
{
  let session_token = UserSessionToken::generate().to_string();

  let query_result = sqlx::query!(
        r#"
INSERT INTO user_sessions (
  token,
  user_token,
  ip_address_creation,
  maybe_creation_type,
  expires_at
)
VALUES ( ?, ?, ?, 'direct_login', NOW() + interval 1 year )
        "#,
        session_token,
        user_token.to_string(),
        ip_address.to_string(),
    )
    .execute(mysql_pool)
    .await;

  let record_id = match query_result {
    Ok(res) => {
      res.last_insert_id()
    },
    Err(err) => {
      return Err(anyhow!("session creation DB error: {:?}", err));
    }
  };

  info!("Created session id: {}", record_id);

  Ok(session_token)
}
