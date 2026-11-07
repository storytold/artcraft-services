const MAX_REDIRECT_LEN: usize = 2048;

/// Exact-match redirect URIs. `https` anywhere, or `http` only for loopback.
/// The character set excludes query strings, fragments, userinfo, and whitespace.
pub fn redirect_uri_is_allowed(uri: &str) -> bool {
  if uri.is_empty() || uri.len() > MAX_REDIRECT_LEN {
    return false;
  }
  if !uri.bytes().all(redirect_byte_allowed) {
    return false;
  }

  let (rest, loopback_only) = if let Some(rest) = uri.strip_prefix("https://") {
    (rest, false)
  } else if let Some(rest) = uri.strip_prefix("http://") {
    (rest, true)
  } else {
    return false;
  };

  if rest.is_empty() || rest.starts_with('/') || rest.starts_with(':') {
    return false;
  }

  let (authority, _path) = match rest.split_once('/') {
    Some((authority, path)) => {
      if path.is_empty() {
        return false;
      }
      (authority, path)
    },
    None => (rest, ""),
  };

  let host = match authority.split_once(':') {
    Some((host, port)) => {
      if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) || authority.matches(':').count() != 1 {
        return false;
      }
      host
    },
    None => authority,
  };
  if host.is_empty() || host.starts_with('.') || host.ends_with('.') {
    return false;
  }

  if loopback_only && host != "127.0.0.1" && host != "localhost" {
    return false;
  }
  true
}

/// Characters allowed in a redirect URI. Query, fragment, userinfo, and whitespace are excluded.
fn redirect_byte_allowed(byte: u8) -> bool {
  matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b':' | b'/')
}

/// Percent-encode a query value. Unreserved characters are left as-is.
pub fn query_encode(value: &str) -> String {
  let mut out = String::with_capacity(value.len());
  for byte in value.bytes() {
    match byte {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(byte as char),
      _ => out.push_str(&format!("%{byte:02X}")),
    }
  }
  out
}

/// Append encoded query pairs. The redirect URI itself has no query string.
pub fn redirect_with_query(uri: &str, pairs: &[(&str, &str)]) -> String {
  let mut out = String::from(uri);
  out.push('?');
  for (index, (key, value)) in pairs.iter().enumerate() {
    if index > 0 {
      out.push('&');
    }
    out.push_str(key);
    out.push('=');
    out.push_str(&query_encode(value));
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn https_and_loopback_are_allowed() {
    assert!(redirect_uri_is_allowed("https://example.com/callback"));
    assert!(redirect_uri_is_allowed("https://example.com"));
    assert!(redirect_uri_is_allowed("http://127.0.0.1:8734/callback"));
    assert!(redirect_uri_is_allowed("http://localhost/callback"));
  }

  #[test]
  fn query_fragment_userinfo_and_remote_http_are_rejected() {
    assert!(!redirect_uri_is_allowed("https://example.com/callback?next=1"));
    assert!(!redirect_uri_is_allowed("https://example.com/callback#frag"));
    assert!(!redirect_uri_is_allowed("https://user@example.com/callback"));
    assert!(!redirect_uri_is_allowed("http://evil.example/callback"));
    assert!(!redirect_uri_is_allowed("http://127.0.0.1.evil.example/callback"));
    assert!(!redirect_uri_is_allowed("https://example.com/callback path"));
    assert!(!redirect_uri_is_allowed("https://example.com/a<b"));
  }
}
