use std::collections::BTreeSet;

#[cfg(test)]
use strum::{EnumCount, EnumIter};
use utoipa::ToSchema;

/// Stored in `user_login_challenges.maybe_failure_type` (VARCHAR(32)).
///
/// Terminal outcome for a failed device approval request. Transient transport/database errors and invalid credentials are not challenge outcomes.
///
/// Add new values as needed, but never change stored values without a migration.
#[cfg_attr(test, derive(EnumIter, EnumCount))]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize, Serialize, ToSchema)]
pub enum UserLoginChallengeFailureType {
  /// An authenticated user explicitly declined the request.
  #[serde(rename = "user_declined")]
  UserDeclined,

  /// The 20-minute deadline passed before the challenge was redeemed.
  #[serde(rename = "expired")]
  Expired,
}

impl_enum_display_and_debug_using_to_str!(UserLoginChallengeFailureType);
impl_mysql_enum_coders!(UserLoginChallengeFailureType);
impl_mysql_from_row!(UserLoginChallengeFailureType);

impl UserLoginChallengeFailureType {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::UserDeclined => "user_declined",
      Self::Expired => "expired",
    }
  }

  pub fn from_str(value: &str) -> Result<Self, String> {
    match value {
      "user_declined" => Ok(Self::UserDeclined),
      "expired" => Ok(Self::Expired),
      _ => Err(format!("invalid UserLoginChallengeFailureType value: {:?}", value)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    BTreeSet::from([Self::UserDeclined, Self::Expired])
  }
}

#[cfg(test)]
mod tests {
  use super::UserLoginChallengeFailureType;
  use crate::test_helpers::assert_serialization;
  use strum::{EnumCount, IntoEnumIterator};

  const STORED_VALUES: &[(UserLoginChallengeFailureType, &str)] = &[(UserLoginChallengeFailureType::UserDeclined, "user_declined"), (UserLoginChallengeFailureType::Expired, "expired")];
  const MAX_STORED_LENGTH: usize = 32;

  #[test]
  fn stored_values_and_serialization_are_stable() {
    for &(variant, stored) in STORED_VALUES {
      assert_serialization(variant, stored);
      assert_eq!(variant.to_str(), stored);
      assert_eq!(UserLoginChallengeFailureType::from_str(stored).unwrap(), variant);
      assert_eq!(format!("{}", variant), stored);
      assert_eq!(format!("{:?}", variant), stored);
      assert!(!stored.is_empty());
      assert!(stored.len() <= MAX_STORED_LENGTH);
    }
  }

  #[test]
  fn all_variants_are_covered_and_round_trip() {
    let variants = UserLoginChallengeFailureType::all_variants();
    assert_eq!(variants.len(), UserLoginChallengeFailureType::COUNT);
    assert_eq!(variants.len(), STORED_VALUES.len());
    for variant in UserLoginChallengeFailureType::iter() {
      assert!(variants.contains(&variant));
      assert_eq!(UserLoginChallengeFailureType::from_str(variant.to_str()).unwrap(), variant);
    }
  }

  #[test]
  fn unknown_stored_values_are_rejected() {
    for value in ["", "unknown", "USER_DECLINED"] {
      assert!(UserLoginChallengeFailureType::from_str(value).is_err());
      assert!(serde_json::from_str::<UserLoginChallengeFailureType>(&format!("{:?}", value)).is_err());
    }
  }
}
