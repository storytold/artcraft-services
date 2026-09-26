use chrono::{DateTime, Utc};
use enums::by_table::user_login_challenges::user_login_challenge_failure_type::UserLoginChallengeFailureType;
use enums::by_table::user_login_challenges::user_login_challenge_status::UserLoginChallengeStatus;

pub struct LoginChallenge {
  pub id: u64,
  pub confirmation_code: String,
  pub status: UserLoginChallengeStatus,
  pub maybe_deciding_user_token: Option<String>,
  pub maybe_redeemed_user_session_id: Option<i64>,
  pub maybe_failure_type: Option<UserLoginChallengeFailureType>,
  pub ip_address_creation: String,
  pub expires_at: DateTime<Utc>,
}
