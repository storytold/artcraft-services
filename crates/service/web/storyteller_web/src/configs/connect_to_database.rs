use std::str::FromStr;
use std::time::Duration;

use log::info;

use errors::AnyhowResult;
use shared_env_var_config::mysql::env_get_mysql_connection_string_or_default;
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::MySqlPool;

pub async fn connect_to_database() -> AnyhowResult<MySqlPool> {
  let db_connection_string = env_get_mysql_connection_string_or_default();

  // NB: The managed DB's connection ceiling is far above what we use, so the binding
  // constraint is this per-pod pool. Keep `replicas * MYSQL_MAX_CONNECTIONS` (plus the
  // background jobs' small pools) comfortably under the server's `max_connections`.
  let max_connections = easyenv::get_env_num("MYSQL_MAX_CONNECTIONS", 20)?;
  let min_connections = easyenv::get_env_num("MYSQL_MIN_CONNECTIONS", 0)?;

  // NB: Fail fast under pool saturation rather than stacking up on sqlx's 30s default — a
  // shorter timeout sheds load and surfaces the problem instead of hiding it behind long waits.
  let acquire_timeout_seconds = easyenv::get_env_num::<u64>("MYSQL_ACQUIRE_TIMEOUT_SECONDS", 10)?;

  // NB: Every cached prepared statement stays open on the server until its connection evicts it
  // or closes, and the server caps the total across ALL clients (`max_prepared_stmt_count`,
  // 16382 on the managed DB). sqlx's default of 100 per connection let `replicas *
  // MYSQL_MAX_CONNECTIONS * 100` blow past that cap, failing new prepares with MySQL error 1461.
  // Keep `replicas * MYSQL_MAX_CONNECTIONS * MYSQL_STATEMENT_CACHE_CAPACITY` well under it.
  let statement_cache_capacity = easyenv::get_env_num::<usize>("MYSQL_STATEMENT_CACHE_CAPACITY", 25)?;

  // NB: Recycling connections releases any server-side statements orphaned by cancelled queries
  // (eg. a client disconnect between prepare and close). sqlx's default lifetime is 30 minutes.
  let max_lifetime_seconds = easyenv::get_env_num::<u64>("MYSQL_MAX_LIFETIME_SECONDS", 600)?;

  // NB: Log the settings before building/connecting so a connect failure has context. We do NOT
  // log the connection string (it contains credentials).
  info!(
    "Building MySQL pool: max_connections={}, min_connections={}, acquire_timeout_seconds={}, \
      statement_cache_capacity={}, max_lifetime_seconds={}",
    max_connections, min_connections, acquire_timeout_seconds,
    statement_cache_capacity, max_lifetime_seconds,
  );

  let connect_options = MySqlConnectOptions::from_str(&db_connection_string)?
      .statement_cache_capacity(statement_cache_capacity);

  let pool_options = MySqlPoolOptions::new()
      .max_connections(max_connections)
      .min_connections(min_connections)
      .acquire_timeout(Duration::from_secs(acquire_timeout_seconds))
      .max_lifetime(Some(Duration::from_secs(max_lifetime_seconds)));

  let pool = pool_options
      .connect_with(connect_options)
      .await?;

  Ok(pool)
}
