use sqlx::{Executor, MySql};

pub struct ReleaseNamedLockArgs<'a, T> {
  pub name: &'a str,
  /// The connection that took the lock.
  pub mysql_executor: T,
}

/// Release a MySQL named lock taken with `try_get_named_lock` on the same connection.
pub async fn release_named_lock<'c, T>(args: ReleaseNamedLockArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!(
    r#"SELECT RELEASE_LOCK(?) AS `released: i64`"#,
    args.name,
  )
    .fetch_one(args.mysql_executor)
    .await?;

  Ok(())
}
