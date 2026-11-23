use r2d2::PooledConnection;
use redis::{Client, Commands};

use errors::AnyhowResult;
use redis_common::redis_keys::RedisKeys;

use crate::job_progress_reporter::job_progress_reporter::{JobProgressReporter, JobProgressReporterBuilder};

/// A job progress reporter that reports to Redis
pub struct RedisJobProgressReporterBuilder {
  redis_pool: r2d2::Pool<Client>,
}

/// A job progress reporter that reports to Redis
pub struct RedisJobProgressReporter {
  redis: PooledConnection<Client>,
  redis_key: String,
}

impl RedisJobProgressReporterBuilder {
  pub fn from_redis_pool(redis_pool: r2d2::Pool<Client>) -> Self {
    Self {
      redis_pool,
    }
  }

  /// Create a new instance. The backing Redis pool is Sync/Send behind an Arc.
  fn create_instance(redis_pool: r2d2::Pool<Client>, redis_key: String) -> AnyhowResult<Box<dyn JobProgressReporter>> {
    let redis = redis_pool.get()?;

    Ok(Box::new(RedisJobProgressReporter {
      redis,
      redis_key,
    }))
  }
}

impl RedisJobProgressReporter {
  pub const STATUS_KEY_TTL_SECONDS : u64 = 60 * 60;
}

impl JobProgressReporterBuilder for RedisJobProgressReporterBuilder {
  fn new_generic_download(&self, job_token: &str) -> AnyhowResult<Box<dyn JobProgressReporter>> {
    let redis_key = RedisKeys::generic_download_extra_status_info(job_token);
    RedisJobProgressReporterBuilder::create_instance(self.redis_pool.clone(), redis_key)
  }

  fn new_generic_inference(&self, job_token: &str) -> AnyhowResult<Box<dyn JobProgressReporter>> {
    let redis_key = RedisKeys::generic_inference_extra_status_info(job_token);
    RedisJobProgressReporterBuilder::create_instance(self.redis_pool.clone(), redis_key)
  }

  fn new_tts_download(&self, tts_job_token: &str) -> AnyhowResult<Box<dyn JobProgressReporter>> {
    let redis_key = RedisKeys::tts_download_extra_status_info(tts_job_token);
    RedisJobProgressReporterBuilder::create_instance(self.redis_pool.clone(), redis_key)
  }

  fn new_tts_inference(&self, tts_job_token: &str) -> AnyhowResult<Box<dyn JobProgressReporter>> {
    let redis_key = RedisKeys::tts_inference_extra_status_info(tts_job_token);
    RedisJobProgressReporterBuilder::create_instance(self.redis_pool.clone(), redis_key)
  }

  fn new_w2l_download(&self, w2l_job_token: &str) -> AnyhowResult<Box<dyn JobProgressReporter>> {
    let redis_key = RedisKeys::w2l_download_extra_status_info(w2l_job_token);
    RedisJobProgressReporterBuilder::create_instance(self.redis_pool.clone(), redis_key)
  }

  fn new_w2l_inference(&self, w2l_job_token: &str) -> AnyhowResult<Box<dyn JobProgressReporter>> {
    let redis_key = RedisKeys::w2l_inference_extra_status_info(w2l_job_token);
    RedisJobProgressReporterBuilder::create_instance(self.redis_pool.clone(), redis_key)
  }
}

impl JobProgressReporter for RedisJobProgressReporter {
  fn log_status(&mut self, logging_details: &str) -> AnyhowResult<()> {
    let _r : String = self.redis // NB: Compiler can't figure out the throwaway result type
        .set_ex(&self.redis_key,
          logging_details,
          Self::STATUS_KEY_TTL_SECONDS)?;
    Ok(()) // No-op
  }
}
