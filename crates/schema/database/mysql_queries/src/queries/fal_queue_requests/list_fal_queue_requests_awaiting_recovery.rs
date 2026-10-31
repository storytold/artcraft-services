use enums::by_table::generic_inference_jobs::inference_job_external_third_party::InferenceJobExternalThirdParty;
use enums::common::job_status_plus::JobStatusPlus;
use sqlx::{Executor, MySql};

use crate::queries::fal_queue_requests::fal_queue_request_awaiting_recovery::FalQueueRequestAwaitingRecovery;

pub struct ListFalQueueRequestsAwaitingRecoveryArgs<T> {
  /// Only requests submitted at least this long ago: their webhook should have arrived by now.
  pub min_age_seconds: u32,
  /// Requests submitted longer ago than this are left alone.
  pub max_age_seconds: u32,
  /// A request claimed less than this long ago is still leased to the server that claimed it.
  pub claim_lease_seconds: u32,
  pub limit: u32,
  pub mysql_executor: T,
}

/// FAL requests whose job is still pending or started, old enough that their webhook should have
/// arrived, and not currently claimed by another server. Oldest first.
pub async fn list_fal_queue_requests_awaiting_recovery<'c, T>(
  args: ListFalQueueRequestsAwaitingRecoveryArgs<T>,
) -> Result<Vec<FalQueueRequestAwaitingRecovery>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query_as!(
    FalQueueRequestAwaitingRecovery,
    r#"
SELECT
  requests.id as `id: u64`,
  requests.fal_request_id,
  requests.status_url,
  requests.response_url,
  requests.recovery_attempt_count as `recovery_attempt_count: u16`
FROM fal_queue_requests AS requests
JOIN generic_inference_jobs AS jobs
  ON jobs.maybe_external_third_party = ?
  AND jobs.maybe_external_third_party_id = requests.fal_request_id
WHERE jobs.status IN (?, ?)
  AND requests.created_at < NOW() - INTERVAL ? SECOND
  AND requests.created_at > NOW() - INTERVAL ? SECOND
  AND (
    requests.maybe_recovery_claimed_at IS NULL
    OR requests.maybe_recovery_claimed_at < NOW() - INTERVAL ? SECOND
  )
ORDER BY requests.created_at ASC
LIMIT ?
"#,
    InferenceJobExternalThirdParty::Fal.to_str(),
    JobStatusPlus::Pending.to_str(),
    JobStatusPlus::Started.to_str(),
    args.min_age_seconds,
    args.max_age_seconds,
    args.claim_lease_seconds,
    args.limit,
  )
    .fetch_all(args.mysql_executor)
    .await
}
