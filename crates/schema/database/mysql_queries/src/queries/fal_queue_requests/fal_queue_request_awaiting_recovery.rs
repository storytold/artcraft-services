/// A FAL request whose job is still unfinished, with what's needed to poll FAL for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FalQueueRequestAwaitingRecovery {
  pub id: u64,
  pub fal_request_id: String,
  pub status_url: String,
  pub response_url: String,
  pub recovery_attempt_count: u16,
}
