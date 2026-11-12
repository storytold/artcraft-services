use errors::{anyhow, AnyhowResult};

/// Environment variable that sets [`FalWebhookSignatureMode`]: `enforce`, `report` or `off`.
pub const FAL_WEBHOOK_SIGNATURE_MODE_ENV: &str = "FAL_WEBHOOK_SIGNATURE_MODE";

/// What the FAL webhook endpoint does with a request whose signature doesn't verify.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FalWebhookSignatureMode {
  /// Reject it before reading the body: 401, or 500 if FAL's keys can't be fetched (so FAL
  /// retries the delivery).
  Enforce,

  /// Log a warning and process it anyway. This is the default, so that failures can be seen
  /// in the logs before switching to `Enforce`.
  Report,

  /// Don't check signatures.
  Off,
}

impl FalWebhookSignatureMode {
  /// Read [`FAL_WEBHOOK_SIGNATURE_MODE_ENV`], defaulting to `Report`. An unrecognised value is an
  /// error, so a typo can't silently turn checking off.
  pub fn from_env() -> AnyhowResult<Self> {
    match easyenv::get_env_string_optional(FAL_WEBHOOK_SIGNATURE_MODE_ENV) {
      None => Ok(Self::Report),
      Some(value) => Self::parse(&value).ok_or_else(|| {
        anyhow!("{} must be one of enforce, report or off (got {:?})", FAL_WEBHOOK_SIGNATURE_MODE_ENV, value)
      }),
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    match value.trim().to_ascii_lowercase().as_str() {
      "enforce" => Some(Self::Enforce),
      "report" => Some(Self::Report),
      "off" => Some(Self::Off),
      _ => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_the_three_modes() {
    assert_eq!(FalWebhookSignatureMode::parse("enforce"), Some(FalWebhookSignatureMode::Enforce));
    assert_eq!(FalWebhookSignatureMode::parse("report"), Some(FalWebhookSignatureMode::Report));
    assert_eq!(FalWebhookSignatureMode::parse("off"), Some(FalWebhookSignatureMode::Off));
  }

  #[test]
  fn ignores_case_and_surrounding_whitespace() {
    assert_eq!(FalWebhookSignatureMode::parse(" Enforce \n"), Some(FalWebhookSignatureMode::Enforce));
  }

  #[test]
  fn rejects_anything_else() {
    for value in ["", "true", "on", "enforced", "1"] {
      assert_eq!(FalWebhookSignatureMode::parse(value), None, "{value:?}");
    }
  }
}
