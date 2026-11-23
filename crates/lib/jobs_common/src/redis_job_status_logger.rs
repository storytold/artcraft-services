use r2d2::PooledConnection;
use redis::{Client, Commands};

use errors::AnyhowResult;
use redis_common::redis_keys::RedisKeys;

#[deprecated(note="Use JobProgressReporterBuilder")]
pub type RedisPool = PooledConnection<Client>;

#[deprecated(note="Use JobProgressReporterBuilder")]
pub struct RedisJobStatusLogger <'a> {
  redis: &'a mut RedisPool,
  status_key: String,
}

impl <'a> RedisJobStatusLogger <'a> {
  pub const STATUS_KEY_TTL_SECONDS : usize = 60 * 60;

  #[deprecated(note="Use JobProgressReporterBuilder")]
  pub fn new_tts_download(redis: &'a mut RedisPool, tts_job_token: &str) -> Self {
    let status_key = RedisKeys::tts_download_extra_status_info(tts_job_token);
    Self { redis, status_key }
  }

  #[deprecated(note="Use JobProgressReporterBuilder")]
  pub fn new_tts_inference(redis: &'a mut RedisPool, tts_job_token: &str) -> Self {
    let status_key = RedisKeys::tts_inference_extra_status_info(tts_job_token);
    Self { redis, status_key }
  }

  #[deprecated(note="Use JobProgressReporterBuilder")]
  pub fn new_generic_download(redis: &'a mut RedisPool, job_token: &str) -> Self {
    let status_key = RedisKeys::generic_download_extra_status_info(job_token);
    Self { redis, status_key }
  }

  #[deprecated(note="Use JobProgressReporterBuilder")]
  pub fn new_w2l_download(redis: &'a mut RedisPool, w2l_job_token: &str) -> Self {
    let status_key = RedisKeys::w2l_download_extra_status_info(w2l_job_token);
    Self { redis, status_key }
  }

  #[deprecated(note="Use JobProgressReporterBuilder")]
  pub fn new_w2l_inference(redis: &'a mut RedisPool, w2l_job_token: &str) -> Self {
    let status_key = RedisKeys::w2l_inference_extra_status_info(w2l_job_token);
    Self { redis, status_key }
  }

  /// Record the status to Redis so the frontend has the latest state.
  #[deprecated(note="Use JobProgressReporterBuilder")]
  pub fn log_status(&mut self, logging_details: &str) -> AnyhowResult<()> {
    let _r : String = self.redis // NB: Compiler can't figure out the throwaway result type
        .set_ex(&self.status_key,
          logging_details,
          Self::STATUS_KEY_TTL_SECONDS)?;
    Ok(())
  }
}
