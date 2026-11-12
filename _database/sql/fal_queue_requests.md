# FAL queue requests and lost-webhook recovery

FAL reports a finished request by calling our webhook. If that call never arrives, the job
stays `pending` forever. That can happen when the webhook URL can't be reached (a self-hosted
server without a public URL), when FAL doesn't send it (on 2026-09-08 FAL silently dropped every
webhook whose URL wasn't percent-encoded), or when this server is down for longer than FAL keeps
retrying.

FAL can't be asked about a request by its ID alone. Its status and result URLs name the app the
request was sent to (`https://queue.fal.run/{owner}/{app}/requests/{id}`), so they are recorded
when the request is submitted.

## The table

`fal_queue_requests` (migration `2026-10-15-120000-0000_create_fal_queue_requests`, materialized
in `migrations_squashed/fal_queue_requests.sql`) holds one row per submitted request:

| Column                      | Meaning                                                                   |
|-----------------------------|---------------------------------------------------------------------------|
| `fal_request_id`            | FAL's request ID; the job's `generic_inference_jobs.maybe_external_third_party_id` |
| `status_url`                | Where to poll the request's status                                        |
| `response_url`              | Where to fetch its result                                                 |
| `recovery_attempt_count`    | How many times the recovery thread has claimed it (inserted as 0)         |
| `maybe_recovery_claimed_at` | When it was last claimed; a claim is a lease of a few minutes             |

Rows are written in the same transaction as the job, by the generation router's paths
(`omni_gen` image, video, audio, mesh and splat; `omni_api` image). Jobs submitted by the older
`/v1/generate/*` endpoints have no row and are not recoverable.

The URLs come from FAL's submit response. A webhook submit response may name only the request,
in which case `fal_client` derives them from the endpoint, as FAL's own clients do.

## Recovery

storyteller-web runs a recovery thread (`threads/recover_lost_fal_webhooks_thread`, configured by
`FAL_WEBHOOK_RECOVERY_*`). Every minute it:

1. lists requests whose job is still `pending` or `started`, submitted between
   `FAL_WEBHOOK_RECOVERY_AFTER_SECS` (10 minutes) and `FAL_WEBHOOK_RECOVERY_MAX_AGE_SECS` (1 hour)
   ago, and not claimed within the lease;
2. claims each one with a conditional `UPDATE`, so only one server polls a request at a time;
3. asks FAL for the request's status, and once it is finished fetches the result;
4. applies the result through the same code as the webhook (`process_fal_webhook_payload`).

FAL keeps results for about an hour after a request finishes, so the window stops there.

## Applying results once

FAL redelivers a webhook up to 31 times when it thinks delivery failed, and may do so while the
first delivery is still being processed. Results for one request are therefore applied under a
MySQL named lock (`GET_LOCK`, named after a hash of the request ID). A second caller gets `429`,
and FAL retries later. After the first has finished, a repeated success is ignored for a job that
already succeeded, and a failure report is ignored for a job that already finished.
