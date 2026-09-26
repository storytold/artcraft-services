use chrono::{DateTime, Utc};

pub struct LoginChallenge {
  pub id: u64,
  pub confirmation_code: String,
  pub status: String,
  pub maybe_deciding_user_token: Option<String>,
  pub maybe_redeemed_user_session_id: Option<i64>,
  pub maybe_failure_type: Option<String>,
  pub ip_address_creation: String,
  pub expires_at: DateTime<Utc>,
}
