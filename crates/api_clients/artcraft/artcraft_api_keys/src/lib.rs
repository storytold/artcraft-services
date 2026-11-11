use std::fmt;

use crate::parse_from_header_value::parse_from_header_value;

pub mod parse_from_header_value;

/// Number of leading characters shown when an [`ArtcraftApiKey`] is formatted. The remainder of
/// the secret is redacted so the full key never lands in logs or error messages.
const REDACTED_PREFIX_CHAR_COUNT: usize = 20;

/// An Artcraft API key (e.g. `artcraft_api_<entropy>`).
///
/// `Debug` and `Display` both redact the value, printing only the first
/// [`REDACTED_PREFIX_CHAR_COUNT`] characters followed by an ellipsis so the full secret is never
/// accidentally logged.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ArtcraftApiKey(pub String);

impl ArtcraftApiKey {
  pub fn new_from_str(value: &str) -> ArtcraftApiKey {
    ArtcraftApiKey(value.to_string())
  }

  /// Parse an [`ArtcraftApiKey`] from the value of an HTTP `Authorization` header, accepting the
  /// `Bearer <key>`, `Key <key>`, and bare `<key>` forms. See [`parse_from_header_value`] for the
  /// full parsing rules.
  pub fn parse_from_authorization_header_value(header_value: &str) -> Option<ArtcraftApiKey> {
    parse_from_header_value(header_value)
  }

  /// The full, unredacted key value as a borrowed `&str`.
  ///
  /// NB: unlike `Display`/`Debug` (which redact), this exposes the complete secret — the
  /// `_be_careful` suffix is a reminder to use it only where the real value is genuinely needed
  /// (storage, the create-key response, tests), never in logs or error messages.
  pub fn as_str_be_careful(&self) -> &str {
    &self.0
  }

  /// The full, unredacted key value as an owned `String`. See [`Self::as_str_be_careful`] for the
  /// caveat behind the `_be_careful` suffix.
  pub fn to_string_be_careful(&self) -> String {
    self.0.clone()
  }

  /// The redacted form used by both `Debug` and `Display`: the first
  /// [`REDACTED_PREFIX_CHAR_COUNT`] characters followed by an ellipsis.
  fn redacted(&self) -> String {
    // A key this short would be mostly or entirely revealed by the prefix, so show none of it.
    let shown = if self.0.chars().count() > 2 * REDACTED_PREFIX_CHAR_COUNT { REDACTED_PREFIX_CHAR_COUNT } else { 0 };
    let prefix: String = self.0.chars().take(shown).collect();
    format!("{prefix}…")
  }
}

impl fmt::Display for ArtcraftApiKey {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.redacted())
  }
}

impl fmt::Debug for ArtcraftApiKey {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "ArtcraftApiKey({:?})", self.redacted())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const SAMPLE_KEY: &str = "artcraft_api_55ax0zhd580m598r6n4n7szdwjb2b28sypapvawh";

  #[test]
  fn new_from_str_and_as_str_round_trip() {
    let key = ArtcraftApiKey::new_from_str(SAMPLE_KEY);
    assert_eq!(key.as_str_be_careful(), SAMPLE_KEY);
    assert_eq!(key.0, SAMPLE_KEY);
  }

  #[test]
  fn display_redacts_to_first_twenty_characters() {
    let key = ArtcraftApiKey::new_from_str(SAMPLE_KEY);
    assert_eq!(format!("{key}"), "artcraft_api_55ax0zh…");
  }

  #[test]
  fn debug_redacts_to_first_twenty_characters() {
    let key = ArtcraftApiKey::new_from_str(SAMPLE_KEY);
    assert_eq!(format!("{key:?}"), "ArtcraftApiKey(\"artcraft_api_55ax0zh…\")");
  }

  #[test]
  fn formatting_does_not_leak_the_full_secret() {
    let key = ArtcraftApiKey::new_from_str(SAMPLE_KEY);
    let suffix = &SAMPLE_KEY[REDACTED_PREFIX_CHAR_COUNT..];
    assert!(!format!("{key}").contains(suffix));
    assert!(!format!("{key:?}").contains(suffix));
  }

  #[test]
  fn short_keys_are_fully_redacted() {
    let key = ArtcraftApiKey::new_from_str("abc");
    assert_eq!(format!("{key}"), "…");
    assert_eq!(format!("{key:?}"), "ArtcraftApiKey(\"…\")");
  }
}
