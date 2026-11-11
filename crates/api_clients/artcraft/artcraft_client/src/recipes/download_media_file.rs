use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use log::{info, warn};
use reqwest::Client;
use url::Url;

use tokens::tokens::media_files::MediaFileToken;

use crate::endpoints::media_files::get_media_file::{get_media_file, GetMediaFileSuccessResponse};
use crate::error::api_error::ApiError;
use crate::error::client_error::ClientError;
use crate::error::storyteller_error::StorytellerError;
use crate::utils::api_host::ApiHost;

pub struct DownloadMediaFileArgs<'a, P: AsRef<Path>> {
  pub media_token: &'a MediaFileToken,
  pub api_host: &'a ApiHost,
  pub download_path: DownloadPath<P>,
}

pub enum DownloadPath<P: AsRef<Path>> {
  /// Write the file to this exact path.
  ExactFilename(P),
  /// Write the file into this directory, generating a filename from the CDN URL extension.
  Directory(P),
}

pub struct DownloadMediaFileResult {
  /// The path the file was written to.
  pub downloaded_file_path: PathBuf,

  /// The full media file response from the API.
  pub media_file_response: GetMediaFileSuccessResponse,

  /// Size of downloaded file in bytes.
  pub filesize_bytes: usize,
}

pub async fn download_media_file<P: AsRef<Path>>(
  args: DownloadMediaFileArgs<'_, P>,
) -> Result<DownloadMediaFileResult, StorytellerError> {
  let DownloadMediaFileArgs {
    media_token,
    api_host,
    download_path
  } = args;

  // 1. Fetch media file info from the API.
  let response = get_media_file(api_host, media_token).await?;
  let media_class = &response.media_file.media_class;
  let cdn_url = &response.media_file.media_links.cdn_url;

  info!("Downloading media file {} of class {} from CDN: {}",
    media_token.as_str(), media_class.to_str(), cdn_url);

  // 2. Determine the output file path.
  let output_path = match &download_path {
    DownloadPath::ExactFilename(path) => path.as_ref().to_path_buf(),
    DownloadPath::Directory(dir) => {
      let filename = derive_filename_from_url(cdn_url, &media_token);
      dir.as_ref().join(filename)
    }
  };

  // 3. Download the bytes from the CDN.
  let bytes = download_bytes(cdn_url).await?;

  // 4. Write to disk. NB: Remove any existing entry (a symlink is removed, not followed) and create
  // exclusively, so a planted link can't redirect the write onto another file.
  match fs::remove_file(&output_path) {
    Err(err) if err.kind() != std::io::ErrorKind::NotFound =>
      return Err(StorytellerError::Client(ClientError::IoError(err))),
    _ => {}
  }

  let mut file = fs::OpenOptions::new()
    .write(true)
    .create_new(true)
    .open(&output_path)
    .map_err(|err| StorytellerError::Client(ClientError::IoError(err)))?;

  file.write_all(&bytes)
    .map_err(|err| StorytellerError::Client(ClientError::IoError(err)))?;

  file.flush()
    .map_err(|err| StorytellerError::Client(ClientError::IoError(err)))?;

  info!("Downloaded {} bytes to {:?}", bytes.len(), output_path);

  Ok(DownloadMediaFileResult {
    downloaded_file_path: output_path,
    media_file_response: response,
    filesize_bytes: bytes.len(),
  })
}

// ── Helpers ──

async fn download_bytes(url: &Url) -> Result<Vec<u8>, StorytellerError> {
  let client = Client::builder()
    .gzip(true)
    .build()
    .map_err(|err| StorytellerError::Client(ClientError::ReqwestError(err)))?;

  let response = client.get(url.as_str())
    .send()
    .await
    .map_err(|err| StorytellerError::Api(ApiError::OtherReqwestError(err)))?;

  let status_code = response.status();

  if !status_code.is_success() {
    let body = response.text().await.unwrap_or_else(|err| {
      warn!("Failed to retrieve response body: {}", err);
      "".to_string()
    });
    return Err(StorytellerError::Api(ApiError::UncategorizedBadResponseWithStatusAndBody {
      status_code,
      body,
    }));
  }

  let bytes = response.bytes()
    .await
    .map_err(|err| StorytellerError::Client(ClientError::ReqwestError(err)))?;

  Ok(bytes.to_vec())
}

/// Derive a filename from the CDN URL's path extension, falling back to the media token.
fn derive_filename_from_url(url: &Url, media_token: &MediaFileToken) -> String {
  let path = url.path();
  let extension = Path::new(path)
    .extension() // NB: without dot '.'
    .and_then(|ext| ext.to_str())
    .unwrap_or("bin");

  // NB: The token and extension come from the request and the server; keep them to filename-safe
  // characters so neither can add path separators or `..`.
  format!("{}.{}", filename_safe(media_token.as_str()), filename_safe(extension))
}

fn filename_safe(value: &str) -> String {
  value.chars()
    .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn derive_filename_from_url_extracts_extension() {
    let url = Url::parse("https://cdn.example.com/files/abc123.png").unwrap();
    let token = MediaFileToken::new_from_str("mf_test123");
    assert_eq!(derive_filename_from_url(&url, &token), "mf_test123.png");
  }

  #[test]
  fn derive_filename_from_url_handles_no_extension() {
    let url = Url::parse("https://cdn.example.com/files/abc123").unwrap();
    let token = MediaFileToken::new_from_str("mf_test123");
    assert_eq!(derive_filename_from_url(&url, &token), "mf_test123.bin");
  }

  #[test]
  fn derive_filename_from_url_cannot_traverse() {
    let url = Url::parse("https://cdn.example.com/files/abc123.png").unwrap();
    let token = MediaFileToken::new_from_str("../../etc/cron.d/x");
    assert_eq!(derive_filename_from_url(&url, &token), "______etc_cron_d_x.png");
  }

  #[test]
  fn derive_filename_from_url_handles_query_params() {
    let url = Url::parse("https://cdn.example.com/files/abc123.mp4?token=xyz").unwrap();
    let token = MediaFileToken::new_from_str("mf_test123");
    assert_eq!(derive_filename_from_url(&url, &token), "mf_test123.mp4");
  }
}
