use log::warn;
use mysql_queries::queries::fal_queue_requests::insert_fal_queue_request::{insert_fal_queue_request, InsertFalQueueRequestArgs};
use sqlx::{Executor, MySql};

pub(crate) struct RecordFalQueueUrlsArgs<'a, T> {
  pub fal_request_id: &'a str,
  pub maybe_status_url: Option<&'a str>,
  pub maybe_response_url: Option<&'a str>,
  pub mysql_executor: T,
}

/// Keep the queue URLs FAL returned for a submitted request, so that its job can still be
/// completed by polling if the webhook never arrives (see `recover_lost_fal_webhooks_thread`).
///
/// Best effort: without both URLs nothing is recorded, and a failure is logged rather than
/// failing the generation request.
pub(crate) async fn record_fal_queue_urls<'c, T>(args: RecordFalQueueUrlsArgs<'_, T>)
where
  T: Executor<'c, Database = MySql>,
{
  let (Some(status_url), Some(response_url)) = (args.maybe_status_url, args.maybe_response_url) else {
    return;
  };

  if args.fal_request_id.is_empty() {
    return;
  }

  let result = insert_fal_queue_request(InsertFalQueueRequestArgs {
    fal_request_id: args.fal_request_id,
    status_url,
    response_url,
    mysql_executor: args.mysql_executor,
  }).await;

  if let Err(err) = result {
    warn!(
      "Could not record the queue URLs of FAL request {}; its job can't be recovered if the webhook is lost: {:?}",
      args.fal_request_id, err,
    );
  }
}
