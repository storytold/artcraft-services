use sqlx::{Executor, MySql};

pub struct ClaimFalQueueRequestForRecoveryArgs<T> {
  pub fal_queue_request_id: u64,
  /// A claim younger than this belongs to whichever server made it.
  pub claim_lease_seconds: u32,
  pub mysql_executor: T,
}

/// Claim a FAL request for one recovery attempt. Returns whether this caller got it: when several
/// servers run the recovery thread, only one of them polls FAL for a request within a lease.
pub async fn claim_fal_queue_request_for_recovery<'c, T>(
  args: ClaimFalQueueRequestForRecoveryArgs<T>,
) -> Result<bool, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let affected = sqlx::query!(
    r#"
UPDATE fal_queue_requests
SET
  maybe_recovery_claimed_at = NOW(),
  recovery_attempt_count = recovery_attempt_count + 1
WHERE id = ?
  AND (
    maybe_recovery_claimed_at IS NULL
    OR maybe_recovery_claimed_at < NOW() - INTERVAL ? SECOND
  )
"#,
    args.fal_queue_request_id,
    args.claim_lease_seconds,
  )
    .execute(args.mysql_executor)
    .await?
    .rows_affected();

  Ok(affected == 1)
}
