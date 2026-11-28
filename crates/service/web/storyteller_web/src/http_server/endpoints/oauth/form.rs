/// Parse `application/x-www-form-urlencoded`. Duplicate keys and bodies over 16 KiB are rejected.
pub fn parse_form(body: &str) -> Option<Vec<(String, String)>> {
  if body.len() > 16 * 1024 {
    return None;
  }
  let mut pairs = Vec::new();
  if body.is_empty() {
    return Some(pairs);
  }
  for part in body.split('&') {
    if part.is_empty() {
      return None;
    }
    let (key, value) = part.split_once('=').unwrap_or((part, ""));
    let key = percent_decode(key)?;
    let value = percent_decode(value)?;
    if pairs.iter().any(|(existing, _)| existing == &key) {
      return None;
    }
    pairs.push((key, value));
  }
  Some(pairs)
}

/// First value for `key`, if the client sent it.
pub fn form_value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
  pairs.iter().find(|(name, _)| name == key).map(|(_, value)| value.as_str())
}

/// Decode `+` and `%HH`. Non-ASCII bytes and truncated escapes are rejected.
fn percent_decode(input: &str) -> Option<String> {
  let bytes = input.as_bytes();
  let mut out = Vec::with_capacity(bytes.len());
  let mut index = 0;
  while index < bytes.len() {
    match bytes[index] {
      b'+' => out.push(b' '),
      b'%' => {
        if index + 2 >= bytes.len() {
          return None;
        }
        let high = hex_value(bytes[index + 1])?;
        let low = hex_value(bytes[index + 2])?;
        out.push((high << 4) | low);
        index += 2;
      },
      byte if byte.is_ascii() && !byte.is_ascii_control() => out.push(byte),
      _ => return None,
    }
    index += 1;
  }
  String::from_utf8(out).ok()
}

/// One hex digit, or `None` when `byte` is not ASCII hex.
fn hex_value(byte: u8) -> Option<u8> {
  match byte {
    b'0'..=b'9' => Some(byte - b'0'),
    b'a'..=b'f' => Some(byte - b'a' + 10),
    b'A'..=b'F' => Some(byte - b'A' + 10),
    _ => None,
  }
}
