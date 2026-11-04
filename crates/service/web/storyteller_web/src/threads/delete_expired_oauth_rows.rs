use std::time::Duration;

use log::warn;
use mysql_queries::queries::oauth_authorization_codes::delete_expired_oauth_codes::{delete_expired_oauth_codes, DeleteExpiredOauthCodesArgs};
use mysql_queries::queries::oauth_consent_nonces::delete_expired_oauth_nonces::{delete_expired_oauth_nonces, DeleteExpiredOauthNoncesArgs};
use mysql_queries::queries::oauth_spent_refresh_hashes::delete_expired_spent_refreshes::{delete_expired_spent_refreshes, DeleteExpiredSpentRefreshesArgs};
use sqlx::MySqlPool;

const BATCH: u64 = 1000;

pub async fn delete_expired_oauth_rows(pool: MySqlPool) {
  loop {
    if let Err(error) = sweep(&pool).await {
      warn!("OAuth row cleanup failed: {}", error);
    }
    tokio::time::sleep(Duration::from_secs(60 * 60)).await;
  }
}

async fn sweep(pool: &MySqlPool) -> Result<(), sqlx::Error> {
  while delete_expired_oauth_nonces(DeleteExpiredOauthNoncesArgs { limit: BATCH, mysql_executor: pool }).await? == BATCH {}
  while delete_expired_oauth_codes(DeleteExpiredOauthCodesArgs { limit: BATCH, mysql_executor: pool }).await? == BATCH {}
  while delete_expired_spent_refreshes(DeleteExpiredSpentRefreshesArgs { limit: BATCH, mysql_executor: pool }).await? == BATCH {}
  Ok(())
}
