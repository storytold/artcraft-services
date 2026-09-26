use anyhow::anyhow;
use log::info;
use sqlx::MySqlPool;

use crate::utils::transactor::Transactor;
use errors::AnyhowResult;
use tokens::tokens::user_sessions::UserSessionToken;
use tokens::tokens::users::UserToken;

pub async fn create_user_session_with_transactor(
  user_token: &UserToken,
  ip_address: &str,
  mut transactor: Transactor<'_, '_>
)
    -> AnyhowResult<UserSessionToken>
{
  let session_token = UserSessionToken::generate();

  let query = sqlx::query!(
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
        session_token.as_str(),
        user_token.as_str(),
        ip_address,
    );

  let query_result = transactor.execute(query).await;

  let record_id = match query_result {
    Ok(res) => res.last_insert_id(),
    Err(err) => {
      return Err(anyhow!("session creation DB error: {:?}", err));
    }
  };

  info!("Created session id: {}", record_id);

  Ok(session_token)
}
