use std::time::Duration;

use itertools::Itertools;
use log::info;
use sqlx::{FromRow, MySql, MySqlPool, Pool, QueryBuilder, Row};
use sqlx::mysql::{MySqlQueryResult, MySqlRow};

use errors::AnyhowResult;

/// Super simple and lazy migration pattern: run the count query to see if we need
/// to migrate, then run the migrate query. Do this in a loop.
pub struct QueryPair {
  pub count_query: String,
  pub migrate_query: String,
}

impl QueryPair {

  pub async fn run_migration(&self, mysql: &Pool<MySql>) -> AnyhowResult<()> {
    info!("Running count query: {}", self.count_query());

    let mut record_count = self.run_count_query(mysql).await?;
    let mut estimated_remaining_record_count = record_count;

    info!("Count: {}", record_count);
    if record_count == 0 {
      return Ok(());
    }

    loop {
      info!("Running migrate query: {}", self.migrate_query());
      let rows_updated = self.run_migrate_query(&mysql).await?;

      estimated_remaining_record_count = estimated_remaining_record_count.saturating_sub(rows_updated);

      info!("Rows updated: {}; estimated remaining rows: {}", rows_updated, estimated_remaining_record_count);

      if rows_updated == 0 {
        break;
      }

      tokio::time::sleep(Duration::from_millis(1000)).await;
    }

    Ok(())
  }

  async fn run_count_query(&self, mysql_pool: &MySqlPool) -> AnyhowResult<u64> {
    let mut query_builder = QueryBuilder::new(&self.count_query);
    let query = query_builder.build_query_as::<CountRecord>();
    let record = query.fetch_one(mysql_pool).await?;
    Ok(record.record_count as u64)
  }

  async fn run_migrate_query(&self, mysql_pool: &MySqlPool) -> AnyhowResult<u64> {
    let mut query_builder = QueryBuilder::new(&self.migrate_query);
    let query = query_builder.build();
    let result = query.execute(mysql_pool).await?;
    Ok(result.rows_affected())
  }

  fn count_query(&self) -> String {
    Self::single_line_query(&self.count_query)
  }

  fn migrate_query(&self) -> String {
    Self::single_line_query(&self.migrate_query)
  }

  fn single_line_query(query: &str) -> String {
    query.split("\n")
        .map(|s| s.trim())
        .join(" ")
        .trim()
        .to_string()
  }
}

pub struct CountRecord {
  pub record_count: i64
}

impl FromRow<'_, MySqlRow> for CountRecord {
  fn from_row(row: &MySqlRow) -> Result<Self, sqlx::Error> {
    Ok(Self {
      record_count: row.try_get("record_count")?,
    })
  }
}
