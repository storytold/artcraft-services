use std::collections::HashSet;

use once_cell::sync::Lazy;

use crate::configs::reserved_names::RESERVED_SUBSTRINGS;
use crate::configs::reserved_names::RESERVED_USERNAMES;

pub fn is_reserved_username(username: &str) -> bool {
  // NB: Accounts are stored trimmed and lowercased, so compare the same way.
  let username = username.trim().to_lowercase();
  let username = username.as_str();

  static RESERVED_USERNAMES_SET : Lazy<HashSet<String>> = Lazy::new(|| RESERVED_USERNAMES.lines()
    .map(|line| line.trim())
    .filter(|line| !(line.starts_with("#") || line.is_empty()))
    .map(|line| line.to_string())
    .collect::<HashSet<String>>());

  if RESERVED_USERNAMES_SET.contains(username) {
    return true;
  }

  is_reserved_substring(username)
}

fn is_reserved_substring(username: &str) -> bool {
  static RESERVED_SUBSTRINGS_LIST : Lazy<Vec<String>> = Lazy::new(|| RESERVED_SUBSTRINGS.lines()
    .map(|line| line.trim())
    .filter(|line| !(line.starts_with("#") || line.is_empty()))
    .map(|line| line.to_string())
    .collect::<Vec<String>>());

  let undashed = username.replace("_", "").replace("-", "");

  for substr in RESERVED_SUBSTRINGS_LIST.iter() {
    if username.contains(substr) || undashed.contains(substr) {
      return true;
    }
  }

  false
}

#[cfg(test)]
mod tests {
  use crate::http_server::validations::is_reserved_username::is_reserved_username;

  #[test]
  fn reserved_usernames() {
    assert_eq!(is_reserved_username("vocodes"), true);
    assert_eq!(is_reserved_username("user"), true);
    assert_eq!(is_reserved_username("username"), true);
    assert_eq!(is_reserved_username("thread"), true);
    assert_eq!(is_reserved_username("test"), true);
  }

  #[test]
  fn reserved_usernames_are_case_and_space_insensitive() {
    assert_eq!(is_reserved_username("Vocodes"), true);
    assert_eq!(is_reserved_username(" USERNAME "), true);
  }

  #[test]
  fn unreserved_usernames() {
    assert_eq!(is_reserved_username("echelon"), false);
    assert_eq!(is_reserved_username("asdfasdfadsf"), false);
    assert_eq!(is_reserved_username("bobdole11"), false);
  }

  #[test]
  fn reserved_substrings() {
    assert_eq!(is_reserved_username("111vocodes111"), true);
    assert_eq!(is_reserved_username("thefakeyousite"), true);

    // These are now un-reserved
    assert_eq!(is_reserved_username("test112345"), false);
    assert_eq!(is_reserved_username("12345test"), false);
  }

  #[test]
  fn reserved_substrings_with_dashes() {
    //assert_eq!(is_reserved_username("t_e_s_t"), true);
    assert_eq!(is_reserved_username("-vo-co-de-s--"), true);
    assert_eq!(is_reserved_username("fake_you"), true);
  }
}