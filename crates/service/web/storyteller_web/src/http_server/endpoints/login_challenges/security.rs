use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use actix_web::HttpRequest;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use http_server_common::request::get_request_ip::get_request_ip;
use rand::{
  rngs::{OsRng, StdRng},
  Rng, SeedableRng, TryRngCore,
};
use sha2::{Digest, Sha256};

use crate::http_server::common_responses::common_web_error::CommonWebError;

const CODE_ALPHABET: &[u8] = b"BCDFGHJKLMNPQRSTVWXZ";
const APPROVAL_ORIGINS: &[&str] = &["https://app.getartcraft.com", "https://getartcraft.com", "https://www.getartcraft.com"];
const WINDOW: Duration = Duration::from_secs(60);
const MAX_BUCKETS: usize = 10_000;

/// Follow the existing gateway's forwarded-IP policy, but normalize stored IPs
/// and use the socket address for direct IPv6 requests (the legacy helper splits
/// on ':'). Gateway deployments must strip caller-supplied forwarding headers.
pub fn request_ip(request: &HttpRequest) -> String {
  get_request_ip(request).parse::<IpAddr>().ok().or_else(|| request.peer_addr().map(|address| address.ip())).map(|ip| ip.to_string()).unwrap_or_else(|| "0.0.0.0".into())
}

pub fn new_secret() -> Result<String, CommonWebError> {
  let mut bytes = [0u8; 32];
  OsRng.try_fill_bytes(&mut bytes).map_err(CommonWebError::from_error)?;
  Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn secret_hash(secret: &str) -> Result<[u8; 32], CommonWebError> {
  if secret.len() != 43 {
    return Err(CommonWebError::NotAuthorized);
  }
  let bytes = URL_SAFE_NO_PAD.decode(secret).map_err(|_| CommonWebError::NotAuthorized)?;
  if bytes.len() != 32 || URL_SAFE_NO_PAD.encode(&bytes) != secret {
    return Err(CommonWebError::NotAuthorized);
  }
  Ok(Sha256::digest(secret.as_bytes()).into())
}

pub fn new_confirmation_code() -> Result<String, CommonWebError> {
  let mut rng = StdRng::try_from_os_rng().map_err(CommonWebError::from_error)?;
  Ok((0..8).map(|_| CODE_ALPHABET[rng.random_range(0..CODE_ALPHABET.len())] as char).collect())
}

/// Approval is browser-only and requires an exact trusted Origin. JSON POSTs
/// plus Origin validation prevent ambient cookies from granting cross-site consent.
pub fn require_approval_origin(request: &HttpRequest) -> Result<(), CommonWebError> {
  let origin = request.headers().get("Origin").and_then(|v| v.to_str().ok()).ok_or(CommonWebError::Forbidden)?;
  if APPROVAL_ORIGINS.contains(&origin) {
    return Ok(());
  }
  // Local web development is permitted only when the API is also local.
  let host = request.connection_info().host().to_owned();
  let local_api = host.starts_with("localhost:") || host.starts_with("127.0.0.1:");
  if local_api && ["http://localhost:4200", "http://127.0.0.1:4200", "http://localhost:4201", "http://127.0.0.1:4201"].contains(&origin) {
    return Ok(());
  }
  Err(CommonWebError::Forbidden)
}

/// Per-process abuse cap, shared by all Actix workers. Entries expire and memory
/// is bounded. Deployments can impose stricter aggregate limits at their gateway.
pub fn rate_limit(ip: &str, create: bool) -> Result<(), CommonWebError> {
  static BUCKETS: OnceLock<Mutex<HashMap<(String, bool), (Instant, u32)>>> = OnceLock::new();
  let mut buckets = BUCKETS.get_or_init(|| Mutex::new(HashMap::new())).lock().map_err(|_| CommonWebError::TooManyRequests)?;
  let now = Instant::now();
  buckets.retain(|_, (started, _)| now.duration_since(*started) < WINDOW);
  let key = (ip.to_owned(), create);
  if buckets.len() >= MAX_BUCKETS && !buckets.contains_key(&key) {
    return Err(CommonWebError::TooManyRequests);
  }
  let (_, count) = buckets.entry(key).or_insert((now, 0));
  let limit = if create { 10 } else { 120 };
  if *count >= limit {
    return Err(CommonWebError::TooManyRequests);
  }
  *count += 1;
  Ok(())
}
