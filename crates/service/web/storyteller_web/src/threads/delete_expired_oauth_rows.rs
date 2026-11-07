use std::future::Future;
use std::time::Duration;

use log::warn;
use mysql_queries::queries::oauth_authorization_codes::delete_expired_oauth_codes::{delete_expired_oauth_codes, DeleteExpiredOauthCodesArgs};
use mysql_queries::queries::oauth_consent_nonces::delete_expired_oauth_nonces::{delete_expired_oauth_nonces, DeleteExpiredOauthNoncesArgs};
use mysql_queries::queries::oauth_spent_refresh_hashes::delete_expired_spent_refreshes::{delete_expired_spent_refreshes, DeleteExpiredSpentRefreshesArgs};
use mysql_queries::queries::oauth_tokens::delete_expired_oauth_tokens::{delete_expired_oauth_tokens, DeleteExpiredOauthTokensArgs};
use sqlx::MySqlPool;

const BATCH: u64 = 1000;

/// Hourly deletion of expired OAuth nonces, codes, spent refresh hashes, and token rows.
pub async fn delete_expired_oauth_rows(pool: MySqlPool) {
  loop {
    sweep(&pool).await;
    tokio::time::sleep(Duration::from_secs(60 * 60)).await;
  }
}

/// Each table is cleaned in its own batches. A failure on one table is logged and does not skip the others.
async fn sweep(pool: &MySqlPool) {
  delete_in_batches("oauth_consent_nonces", || delete_expired_oauth_nonces(DeleteExpiredOauthNoncesArgs { limit: BATCH, mysql_executor: pool })).await;
  delete_in_batches("oauth_authorization_codes", || delete_expired_oauth_codes(DeleteExpiredOauthCodesArgs { limit: BATCH, mysql_executor: pool })).await;
  delete_in_batches("oauth_spent_refresh_hashes", || delete_expired_spent_refreshes(DeleteExpiredSpentRefreshesArgs { limit: BATCH, mysql_executor: pool })).await;
  delete_in_batches("oauth_tokens", || delete_expired_oauth_tokens(DeleteExpiredOauthTokensArgs { limit: BATCH, mysql_executor: pool })).await;
}

/// Delete up to `BATCH` rows at a time. A query error is logged and ends this table's pass.
async fn delete_in_batches<F, Fut>(table: &str, delete_batch: F)
where
  F: Fn() -> Fut,
  Fut: Future<Output = Result<u64, sqlx::Error>>,
{
  loop {
    match delete_batch().await {
      Ok(deleted) if deleted == BATCH => {},
      Ok(_) => break,
      Err(error) => {
        warn!("OAuth cleanup failed for {table}: {error}");
        break;
      },
    }
  }
}
