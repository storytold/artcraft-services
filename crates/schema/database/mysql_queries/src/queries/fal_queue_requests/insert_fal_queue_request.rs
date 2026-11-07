use sqlx::{Executor, MySql};

pub struct InsertFalQueueRequestArgs<'a, T> {
  /// FAL's queue request ID (the job's `maybe_external_third_party_id`).
  pub fal_request_id: &'a str,
  /// From FAL's queue submit response.
  pub status_url: &'a str,
  /// From FAL's queue submit response.
  pub response_url: &'a str,
  pub mysql_executor: T,
}

/// Record the queue URLs FAL returned for a submitted request, so the job can be recovered by
/// polling if its webhook never arrives. Insert it in the same transaction as the job.
pub async fn insert_fal_queue_request<'c, T>(args: InsertFalQueueRequestArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!(
    r#"
INSERT INTO fal_queue_requests
SET
  fal_request_id = ?,
  status_url = ?,
  response_url = ?,
  recovery_attempt_count = 0
"#,
    args.fal_request_id,
    args.status_url,
    args.response_url,
  )
    .execute(args.mysql_executor)
    .await?;

  Ok(())
}
