//! Disposable databases on the local MySQL instance only. No application or test
//! database URL is read, so production configuration cannot redirect these tests.

use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions, MySqlSslMode};
use sqlx::MySqlPool;
use tokens::tokens::users::UserToken;

use crate::schema::ensure_schema;

pub struct IsolatedTestDatabase {
  pub pool: MySqlPool,
  admin_pool: MySqlPool,
  name: String,
}

impl IsolatedTestDatabase {
  pub async fn create() -> Self {
    // Deliberately no URL/env parsing, DNS, TCP (including forwarded local ports),
    // or socket overrides. This is the local Homebrew MySQL socket. Only a new
    // randomly named database can be created/dropped; existing schemas are unused.
    let options = MySqlConnectOptions::new().socket("/tmp/mysql.sock").username("root").ssl_mode(MySqlSslMode::Disabled);
    let admin_pool = MySqlPoolOptions::new().max_connections(1).connect_with(options.clone()).await.expect("local MySQL test connection");
    let name = format!("artcraft_test_bridge_{}", UserToken::generate().as_str());
    assert!(name.len() <= 64 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
    sqlx::query(&format!("CREATE DATABASE `{name}`")).execute(&admin_pool).await.expect("create disposable database");
    let pool = MySqlPoolOptions::new().max_connections(8).connect_with(options.database(&name)).await.expect("connect disposable database");
    ensure_schema(&pool).await;
    Self { pool, admin_pool, name }
  }

  pub async fn destroy(self) {
    self.pool.close().await;
    sqlx::query(&format!("DROP DATABASE `{}`", self.name)).execute(&self.admin_pool).await.expect("drop our disposable database");
    self.admin_pool.close().await;
  }
}
