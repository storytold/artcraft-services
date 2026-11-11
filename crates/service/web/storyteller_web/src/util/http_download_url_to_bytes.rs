use bytes::Bytes;
use log::warn;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr};
use wreq::redirect::{Attempt, Policy};
use wreq::{Client, StatusCode};

const USER_AGENT: &str = "storyteller-web/1.0";

/// Follow up to this many redirects. Many legitimate file URLs (CDN edges,
/// Wikimedia's `Special:FilePath`, S3 pre-sign indirections) answer with a 3xx
/// before serving the bytes, so we must follow them rather than treat the 3xx
/// as a download failure.
const MAX_REDIRECTS: usize = 10;

/// Cap on decoded bytes, enforced per chunk so gzip bombs are cut off too.
const MAX_DOWNLOAD_BYTES: usize = 1 << 30;

// TODO(bt, 2025-06-03): Don't load the entire file into memory!

#[derive(Debug)]
pub enum DownloadFileToBytesError {
  WreqError(wreq::Error),
  TooLarge { url: String },
  /// The URL (or a redirect target) is not http(s) or names a loopback/private/link-local host.
  ForbiddenDestination { url: String },
  BadResponseError {
    status_code: StatusCode,
    body: Bytes,
    url: String,
  }
}

fn is_forbidden_ipv4(ip: Ipv4Addr) -> bool {
  let o = ip.octets();
  ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_unspecified()
      || ip.is_broadcast() || (o[0] == 100 && (o[1] & 0xc0) == 64) // 100.64.0.0/10 CGNAT
}

/// shortcut: checks the URL text only (IP literals and local-looking names); a public DNS name that
/// resolves to a private address still passes. Upgrade to a resolver-level check when the owner wants it.
fn is_forbidden_destination(uri: &wreq::Uri) -> bool {
  if !matches!(uri.scheme_str(), Some("http") | Some("https")) {
    return true;
  }
  let Some(host) = uri.host() else { return true };
  let host = host.trim_start_matches('[').trim_end_matches(']').trim_end_matches('.').to_ascii_lowercase();
  match host.parse::<IpAddr>() {
    Ok(IpAddr::V4(ip)) => is_forbidden_ipv4(ip),
    Ok(IpAddr::V6(ip)) => match ip.to_ipv4_mapped() {
      Some(v4) => is_forbidden_ipv4(v4),
      None => ip.is_loopback() || ip.is_unspecified()
          || (ip.segments()[0] & 0xfe00) == 0xfc00 // fc00::/7 unique local
          || (ip.segments()[0] & 0xffc0) == 0xfe80, // fe80::/10 link local
    },
    Err(_) => host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local")
        || host.ends_with(".internal"),
  }
}

fn redirect_policy(attempt: Attempt) -> wreq::redirect::Action {
  if attempt.previous.len() > MAX_REDIRECTS {
    attempt.error("too many redirects")
  } else if is_forbidden_destination(&attempt.uri) {
    attempt.error("redirect to forbidden destination")
  } else {
    attempt.follow()
  }
}

/// Downloads a (binary) file to memory. Good for images, etc. Not great for large files.
pub async fn http_download_url_to_bytes(url: &str) -> Result<Bytes, DownloadFileToBytesError> {
  match url.parse::<wreq::Uri>() {
    Ok(uri) if !is_forbidden_destination(&uri) => {}
    _ => return Err(DownloadFileToBytesError::ForbiddenDestination { url: url.to_string() }),
  }

  let client = Client::builder()
      .gzip(true)
      .redirect(Policy::custom(redirect_policy))
      .build()?;

  let mut response = client.get(url) // NB: No IntoUrl for &Url.
      .header("User-Agent", USER_AGENT)
      .header("Accept", "*/*")
      .send()
      .await?;

  let status = response.status();

  let mut buffer = Vec::new();
  while let Some(chunk) = response.chunk().await? {
    if buffer.len() + chunk.len() > MAX_DOWNLOAD_BYTES {
      return Err(DownloadFileToBytesError::TooLarge { url: url.to_string() });
    }
    buffer.extend_from_slice(&chunk);
  }
  let bytes = Bytes::from(buffer);

  if !status.is_success() {
    warn!("Error downloading URL (status={}): {:?}", status.as_u16(), url);

    return Err(DownloadFileToBytesError::BadResponseError {
      status_code: status,
      body: bytes,
      url: url.to_string(),
    });
  }

  Ok(bytes)
}

impl Error for DownloadFileToBytesError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      DownloadFileToBytesError::WreqError(e) => Some(e),
      DownloadFileToBytesError::BadResponseError { .. }
        | DownloadFileToBytesError::TooLarge { .. }
        | DownloadFileToBytesError::ForbiddenDestination { .. } => None,
    }
  }
}

impl Display for DownloadFileToBytesError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      DownloadFileToBytesError::WreqError(e) => write!(f, "Wreq error: {}", e),
      DownloadFileToBytesError::TooLarge { url } => write!(f, "Download of {} exceeded {} bytes", url, MAX_DOWNLOAD_BYTES),
      DownloadFileToBytesError::ForbiddenDestination { url } => write!(f, "Forbidden download destination: {}", url),
      DownloadFileToBytesError::BadResponseError { status_code, body, url } => {
        write!(f, "Bad response (status={}) fetching {} : {:?}", status_code.as_u16(), url, body)
      }
    }
  }
}

impl From<wreq::Error> for DownloadFileToBytesError {
  fn from(e: wreq::Error) -> Self {
    DownloadFileToBytesError::WreqError(e)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn forbidden(u: &str) -> bool { is_forbidden_destination(&u.parse().unwrap()) }

  #[test]
  fn blocks_internal_destinations() {
    for u in ["http://127.0.0.1/x", "http://10.1.2.3/", "http://192.168.0.1/", "http://169.254.169.254/latest",
      "http://172.16.0.1/", "http://100.64.0.1/", "http://0.0.0.0/", "http://[::1]/", "http://[fd00::1]/",
      "http://[fe80::1]/", "http://[::ffff:127.0.0.1]/", "http://localhost/", "http://svc.internal/", "ftp://example.com/x"] {
      assert!(forbidden(u), "{u}");
    }
  }

  #[test]
  fn allows_public_destinations() {
    for u in ["https://example.com/a.png", "http://8.8.8.8/", "https://cdn.example.com:8443/x", "http://100.128.0.1/"] {
      assert!(!forbidden(u), "{u}");
    }
  }
}
