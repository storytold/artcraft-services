use sqlx::{Executor, MySql};

pub struct TryGetNamedLockArgs<'a, T> {
  /// At most 64 characters (MySQL's limit for lock names).
  pub name: &'a str,
  /// Must be a single connection, not a pool: the lock belongs to the connection that took it,
  /// and only that connection can release it.
  pub mysql_executor: T,
}

/// Take a MySQL named lock (`GET_LOCK`) without waiting. Returns whether it was taken; `false`
/// means another connection holds it. Release it with `release_named_lock` on the same connection.
pub async fn try_get_named_lock<'c, T>(args: TryGetNamedLockArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let record = sqlx::query!(
    r#"SELECT GET_LOCK(?, 0) AS `locked: i64`"#,
    args.name,
  )
    .fetch_one(args.mysql_executor)
    .await?;

  Ok(record.locked == Some(1))
}
