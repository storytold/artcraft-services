use std::time::Duration;

use log::warn;
use mysql_queries::queries::user_login_challenges::expire_abandoned_challenges::{expire_abandoned_challenges, ExpireAbandonedChallengesArgs};
use sqlx::MySqlPool;

pub async fn expire_login_challenges(pool: MySqlPool) {
  loop {
    if let Err(error) = expire_abandoned_challenges(ExpireAbandonedChallengesArgs {
      mysql_executor: &pool,
    }).await {
      warn!("Login challenge expiration sweep failed: {}", error);
    }
    tokio::time::sleep(Duration::from_secs(30)).await;
  }
}
