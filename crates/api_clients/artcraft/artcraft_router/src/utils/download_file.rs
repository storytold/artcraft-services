use crate::errors::artcraft_router_error::ArtcraftRouterError;
use crate::errors::download_error::DownloadError;

/// Cap on decoded bytes (checked per chunk, so gzip bombs are cut off too).
const MAX_DOWNLOAD_BYTES: usize = 1 << 30;

/// Whole-transfer deadline.
const DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// Download a file from a URL, returning its bytes.
pub async fn download_file(url: &str) -> Result<Vec<u8>, ArtcraftRouterError> {
  let client = reqwest::Client::builder()
    .timeout(DOWNLOAD_TIMEOUT)
    .build()
    .map_err(|err| DownloadError::ReqwestDownload {
      url: url.to_string(),
      maybe_status: err.status(),
      error: err,
    })?;

  let mut response = client.get(url).send()
    .await
    .map_err(|err| DownloadError::ReqwestDownload {
      url: url.to_string(),
      maybe_status: err.status(),
      error: err,
    })?;

  let status = response.status();
  if !status.is_success() {
    return Err(DownloadError::BadStatus {
      url: url.to_string(),
      status_code: status.as_u16(),
    }.into());
  }

  let to_err = |err: reqwest::Error| DownloadError::ReqwestDownload {
    url: url.to_string(),
    maybe_status: err.status(),
    error: err,
  };

  let mut bytes = Vec::new();
  while let Some(chunk) = response.chunk().await.map_err(to_err)? {
    if bytes.len() + chunk.len() > MAX_DOWNLOAD_BYTES {
      return Err(DownloadError::TooLarge {
        url: url.to_string(),
        max_bytes: MAX_DOWNLOAD_BYTES,
      }.into());
    }
    bytes.extend_from_slice(&chunk);
  }
  Ok(bytes)
}
