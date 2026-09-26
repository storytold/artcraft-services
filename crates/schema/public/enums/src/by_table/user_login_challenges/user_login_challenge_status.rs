use std::collections::BTreeSet;

#[cfg(test)]
use strum::{EnumCount, EnumIter};
use utoipa::ToSchema;

/// Stored in `user_login_challenges.status` (VARCHAR(16)).
///
/// Persisted state of a device approval request. Always check expires_at independently; a pending or approved row can already be expired.
///
/// Add new values as needed, but never change stored values without a migration.
#[cfg_attr(test, derive(EnumIter, EnumCount))]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize, Serialize, ToSchema)]
pub enum UserLoginChallengeStatus {
  /// Waiting for an authenticated user's explicit decision.
  #[serde(rename = "pending")]
  Pending,

  /// Approved for the deciding user; no session has been minted yet.
  #[serde(rename = "approved")]
  Approved,

  /// Exactly one session has been minted. Retries must never mint another.
  #[serde(rename = "redeemed")]
  Redeemed,

  /// Terminal failure, described by maybe_failure_type.
  #[serde(rename = "failed")]
  Failed,
}

impl_enum_display_and_debug_using_to_str!(UserLoginChallengeStatus);
impl_mysql_enum_coders!(UserLoginChallengeStatus);
impl_mysql_from_row!(UserLoginChallengeStatus);

impl UserLoginChallengeStatus {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::Pending => "pending",
      Self::Approved => "approved",
      Self::Redeemed => "redeemed",
      Self::Failed => "failed",
    }
  }

  pub fn from_str(value: &str) -> Result<Self, String> {
    match value {
      "pending" => Ok(Self::Pending),
      "approved" => Ok(Self::Approved),
      "redeemed" => Ok(Self::Redeemed),
      "failed" => Ok(Self::Failed),
      _ => Err(format!("invalid UserLoginChallengeStatus value: {:?}", value)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    BTreeSet::from([Self::Pending, Self::Approved, Self::Redeemed, Self::Failed])
  }
}

#[cfg(test)]
mod tests {
  use super::UserLoginChallengeStatus;
  use crate::test_helpers::assert_serialization;
  use strum::{EnumCount, IntoEnumIterator};

  const STORED_VALUES: &[(UserLoginChallengeStatus, &str)] = &[(UserLoginChallengeStatus::Pending, "pending"), (UserLoginChallengeStatus::Approved, "approved"), (UserLoginChallengeStatus::Redeemed, "redeemed"), (UserLoginChallengeStatus::Failed, "failed")];
  const MAX_STORED_LENGTH: usize = 16;

  #[test]
  fn stored_values_and_serialization_are_stable() {
    for &(variant, stored) in STORED_VALUES {
      assert_serialization(variant, stored);
      assert_eq!(variant.to_str(), stored);
      assert_eq!(UserLoginChallengeStatus::from_str(stored).unwrap(), variant);
      assert_eq!(format!("{}", variant), stored);
      assert_eq!(format!("{:?}", variant), stored);
      assert!(!stored.is_empty());
      assert!(stored.len() <= MAX_STORED_LENGTH);
    }
  }

  #[test]
  fn all_variants_are_covered_and_round_trip() {
    let variants = UserLoginChallengeStatus::all_variants();
    assert_eq!(variants.len(), UserLoginChallengeStatus::COUNT);
    assert_eq!(variants.len(), STORED_VALUES.len());
    for variant in UserLoginChallengeStatus::iter() {
      assert!(variants.contains(&variant));
      assert_eq!(UserLoginChallengeStatus::from_str(variant.to_str()).unwrap(), variant);
    }
  }

  #[test]
  fn unknown_stored_values_are_rejected() {
    for value in ["", "unknown", "PENDING"] {
      assert!(UserLoginChallengeStatus::from_str(value).is_err());
      assert!(serde_json::from_str::<UserLoginChallengeStatus>(&format!("{:?}", value)).is_err());
    }
  }
}
