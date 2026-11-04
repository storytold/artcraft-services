use std::str::FromStr;

use actix_http::header::HeaderMap;
use actix_http::header::HeaderName;

pub (crate) fn get_ip_from_header(headers: &HeaderMap, header_name: &str) -> Option<String> {
  if let Ok(header_name) = HeaderName::from_str(header_name) {
    headers.get(&header_name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| if header_name == "forwarded" {
          first_ip_from_forwarded(value)
        } else {
          first_ip_from_list(value)
        })
  } else {
    None
  }
}

// NB: GKE packs in multiple IP addresses into the header:
// Example of header: x-forwarded-for: Some("136.55.189.34, 34.117.9.171")
fn first_ip_from_list(ip_list: &str) -> Option<String> {
  ip_list.split(",")
      .into_iter()
      .map(|ip| ip.trim())
      .find(|ip| !ip.is_empty())
      .map(|ip| ip.to_string())
}

// RFC 7239: `Forwarded: for=192.0.2.43;proto=http, for="[2001:db8::1]:4711"`.
// Returns the bare address of the first `for=` parameter, without quotes, brackets or port.
fn first_ip_from_forwarded(value: &str) -> Option<String> {
  let element = value.split(',').next()?;
  let node = element.split(';')
      .filter_map(|pair| pair.split_once('='))
      .find(|(key, _)| key.trim().eq_ignore_ascii_case("for"))
      .map(|(_, node)| node.trim().trim_matches('"'))?;
  let ip = if let Some(rest) = node.strip_prefix('[') {
    rest.split(']').next()?
  } else if node.matches(':').count() == 1 {
    node.split(':').next()?
  } else {
    node
  };
  (!ip.is_empty()).then(|| ip.to_string())
}

#[cfg(test)]
mod tests {
  use crate::extractors::get_ip_from_header::{first_ip_from_forwarded, first_ip_from_list};

  #[test]
  fn test_first_ip() {
    assert_eq!(first_ip_from_list("1.2.3.4"), Some("1.2.3.4".to_string()));
    assert_eq!(first_ip_from_list("1.2.3.4, 34.117.9.171"), Some("1.2.3.4".to_string()));
    assert_eq!(first_ip_from_list(", 1.2.3.4"), Some("1.2.3.4".to_string()));
    assert_eq!(first_ip_from_list("1.2.3.4, "), Some("1.2.3.4".to_string()));
    assert_eq!(first_ip_from_list(""), None);
  }

  #[test]
  fn test_forwarded() {
    assert_eq!(first_ip_from_forwarded("for=1.2.3.4"), Some("1.2.3.4".to_string()));
    assert_eq!(first_ip_from_forwarded("for=1.2.3.4:80;proto=http, for=5.6.7.8"), Some("1.2.3.4".to_string()));
    assert_eq!(first_ip_from_forwarded("proto=https; For=\"[2001:db8::1]:4711\""), Some("2001:db8::1".to_string()));
    assert_eq!(first_ip_from_forwarded("for=\"[2001:db8::1]\""), Some("2001:db8::1".to_string()));
    assert_eq!(first_ip_from_forwarded("proto=http"), None);
  }
}
