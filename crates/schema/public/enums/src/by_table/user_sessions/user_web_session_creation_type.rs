use std::collections::BTreeSet;

#[cfg(test)]
use strum::{EnumCount, EnumIter};
use utoipa::ToSchema;

/// Stored in `user_sessions.maybe_creation_type` (VARCHAR(32)).
///
/// How a cookie-based user session was created; NULL in MySQL means unknown. This is provenance, not an authentication policy or an MCP/API session type.
///
/// Add new values as needed, but never change stored values without a migration.
#[cfg_attr(test, derive(EnumIter, EnumCount))]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize, Serialize, ToSchema)]
pub enum UserWebSessionCreationType {
  /// A normal password or SSO login/signup, including direct desktop password login.
  #[serde(rename = "direct_login")]
  DirectLogin,

  /// A new session authorized through a user_login_challenges device approval.
  #[serde(rename = "device_approval")]
  DeviceApproval,

  /// An explicitly consented staff impersonation; existing impersonation rules still apply.
  #[serde(rename = "impersonation")]
  Impersonation,
}

impl_enum_display_and_debug_using_to_str!(UserWebSessionCreationType);
impl_mysql_enum_coders!(UserWebSessionCreationType);
impl_mysql_from_row!(UserWebSessionCreationType);

impl UserWebSessionCreationType {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::DirectLogin => "direct_login",
      Self::DeviceApproval => "device_approval",
      Self::Impersonation => "impersonation",
    }
  }

  pub fn from_str(value: &str) -> Result<Self, String> {
    match value {
      "direct_login" => Ok(Self::DirectLogin),
      "device_approval" => Ok(Self::DeviceApproval),
      "impersonation" => Ok(Self::Impersonation),
      _ => Err(format!("invalid UserWebSessionCreationType value: {:?}", value)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    BTreeSet::from([Self::DirectLogin, Self::DeviceApproval, Self::Impersonation])
  }
}

#[cfg(test)]
mod tests {
  use super::UserWebSessionCreationType;
  use crate::test_helpers::assert_serialization;
  use strum::{EnumCount, IntoEnumIterator};

  const STORED_VALUES: &[(UserWebSessionCreationType, &str)] = &[(UserWebSessionCreationType::DirectLogin, "direct_login"), (UserWebSessionCreationType::DeviceApproval, "device_approval"), (UserWebSessionCreationType::Impersonation, "impersonation")];
  const MAX_STORED_LENGTH: usize = 32;

  #[test]
  fn stored_values_and_serialization_are_stable() {
    for &(variant, stored) in STORED_VALUES {
      assert_serialization(variant, stored);
      assert_eq!(variant.to_str(), stored);
      assert_eq!(UserWebSessionCreationType::from_str(stored).unwrap(), variant);
      assert_eq!(format!("{}", variant), stored);
      assert_eq!(format!("{:?}", variant), stored);
      assert!(!stored.is_empty());
      assert!(stored.len() <= MAX_STORED_LENGTH);
    }
  }

  #[test]
  fn all_variants_are_covered_and_round_trip() {
    let variants = UserWebSessionCreationType::all_variants();
    assert_eq!(variants.len(), UserWebSessionCreationType::COUNT);
    assert_eq!(variants.len(), STORED_VALUES.len());
    for variant in UserWebSessionCreationType::iter() {
      assert!(variants.contains(&variant));
      assert_eq!(UserWebSessionCreationType::from_str(variant.to_str()).unwrap(), variant);
    }
  }

  #[test]
  fn unknown_stored_values_are_rejected() {
    for value in ["", "unknown", "DIRECT_LOGIN"] {
      assert!(UserWebSessionCreationType::from_str(value).is_err());
      assert!(serde_json::from_str::<UserWebSessionCreationType>(&format!("{:?}", value)).is_err());
    }
  }
}
