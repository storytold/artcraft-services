//! How storyteller-web recovers FAL jobs whose webhook never arrived, from the environment.
//!
//! | Variable                                  | Default        | Meaning                                                   |
//! |-------------------------------------------|----------------|-----------------------------------------------------------|
//! | `FAL_WEBHOOK_RECOVERY_ENABLED`            | `true`         | Run the recovery thread at all                            |
//! | `FAL_WEBHOOK_RECOVERY_AFTER_SECS`         | `600` (10 min) | How long after submitting to wait for the webhook         |
//! | `FAL_WEBHOOK_RECOVERY_INTERVAL_SECS`      | `60`           | How often to look for jobs to recover; at least `10`      |
//! | `FAL_WEBHOOK_RECOVERY_MAX_AGE_SECS`       | `3600` (1 h)   | Jobs submitted longer ago than this are left alone        |
//!
//! FAL keeps a request's result for about an hour after it finishes (less for large results), so
//! there is no point looking for much longer than that. FAL also redelivers a failed webhook for
//! about that long, which covers outages of this server; recovery covers webhooks that are never
//! sent or can't reach this server (as on 2026-09-08, when FAL silently dropped every webhook
//! whose URL wasn't percent-encoded).
//!
//! Only jobs submitted through the generation router are recoverable: that is where FAL's queue
//! URLs are recorded (`fal_queue_requests`).
//!
//! A server that FAL can't reach (self-hosting without a public URL) can set
//! `FAL_WEBHOOK_RECOVERY_AFTER_SECS` to something like `30`, so results arrive by polling soon
//! after FAL finishes.

use std::time::Duration;

const DEFAULT_RECOVER_AFTER_SECS: u64 = 600;
const DEFAULT_INTERVAL_SECS: u64 = 60;
const MIN_INTERVAL_SECS: u64 = 10;
const DEFAULT_MAX_AGE_SECS: u64 = 3_600;

/// A claimed request is left to the server that claimed it for this long. It has to outlast
/// processing one result (which downloads the outputs), and it is also how soon a job that FAL
/// is still working on gets polled again.
const CLAIM_LEASE_SECS: u32 = 300;

/// The most requests looked at in one pass.
const BATCH_SIZE: u32 = 50;

#[derive(Clone, Debug)]
pub struct FalWebhookRecoveryConfig {
  pub enabled: bool,
  pub recover_after: Duration,
  pub interval: Duration,
  pub max_age: Duration,
  pub claim_lease: Duration,
  pub batch_size: u32,
}

impl FalWebhookRecoveryConfig {
  pub fn from_env() -> Self {
    let recover_after_secs = easyenv::get_env_num::<u64>("FAL_WEBHOOK_RECOVERY_AFTER_SECS", DEFAULT_RECOVER_AFTER_SECS)
      .unwrap_or(DEFAULT_RECOVER_AFTER_SECS);
    let interval_secs = easyenv::get_env_num::<u64>("FAL_WEBHOOK_RECOVERY_INTERVAL_SECS", DEFAULT_INTERVAL_SECS)
      .unwrap_or(DEFAULT_INTERVAL_SECS)
      .max(MIN_INTERVAL_SECS);
    let max_age_secs = easyenv::get_env_num::<u64>("FAL_WEBHOOK_RECOVERY_MAX_AGE_SECS", DEFAULT_MAX_AGE_SECS)
      .unwrap_or(DEFAULT_MAX_AGE_SECS);

    Self {
      enabled: easyenv::get_env_bool_or_default("FAL_WEBHOOK_RECOVERY_ENABLED", true),
      recover_after: Duration::from_secs(recover_after_secs),
      interval: Duration::from_secs(interval_secs),
      max_age: Duration::from_secs(max_age_secs.max(recover_after_secs)),
      claim_lease: Duration::from_secs(CLAIM_LEASE_SECS as u64),
      batch_size: BATCH_SIZE,
    }
  }
}
