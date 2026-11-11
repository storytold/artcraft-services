//! Mesh-only download bound. Leave the shared, non-mesh downloader unchanged.

use std::error::Error;
use std::fmt::{Display, Formatter};

use bytes::{Bytes, BytesMut};
use wreq::redirect::Policy;
use wreq::{Client, StatusCode};

// Independent of the ZIP's declared-size and extracted-OBJ limits. Count the
// actual response chunks (after HTTP decompression), not an untrusted length.
const MAX_MESH_DOWNLOAD_BYTES: usize = 256 * 1024 * 1024;
const MAX_REDIRECTS: usize = 10;
const USER_AGENT: &str = "storyteller-web/1.0";

#[derive(Debug)]
pub(super) enum MeshDownloadError {
  Request(wreq::Error),
  BadStatus(StatusCode),
  TooLarge { max_bytes: usize },
}

pub(super) async fn download_mesh_file(url: &str) -> Result<Bytes, MeshDownloadError> {
  download_mesh_file_with_limit(url, MAX_MESH_DOWNLOAD_BYTES).await
}

async fn download_mesh_file_with_limit(url: &str, max_bytes: usize) -> Result<Bytes, MeshDownloadError> {
  let client = Client::builder().gzip(true).redirect(Policy::limited(MAX_REDIRECTS)).build()?;
  let mut response = client.get(url).header("User-Agent", USER_AGENT).header("Accept", "*/*").send().await?;

  if !response.status().is_success() {
    return Err(MeshDownloadError::BadStatus(response.status()));
  }
  // An early rejection is useful, but this is never the only size check.
  if response.content_length().map(|size| size > max_bytes as u64).unwrap_or(false) {
    return Err(MeshDownloadError::TooLarge { max_bytes });
  }

  let mut bytes = BytesMut::new();
  while let Some(chunk) = response.chunk().await? {
    if chunk.len() > max_bytes.saturating_sub(bytes.len()) {
      return Err(MeshDownloadError::TooLarge { max_bytes });
    }
    bytes.extend_from_slice(&chunk);
  }
  Ok(bytes.freeze())
}

impl Display for MeshDownloadError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Request(err) => write!(f, "mesh download failed: {}", err),
      Self::BadStatus(status) => write!(f, "mesh download returned HTTP {}", status.as_u16()),
      Self::TooLarge { max_bytes } => write!(f, "mesh download exceeds {} byte limit", max_bytes),
    }
  }
}

impl Error for MeshDownloadError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      Self::Request(err) => Some(err),
      _ => None,
    }
  }
}

impl From<wreq::Error> for MeshDownloadError {
  fn from(err: wreq::Error) -> Self {
    Self::Request(err)
  }
}

#[cfg(test)]
mod tests {
  use std::io::{Read, Write};
  use std::net::TcpListener;
  use std::thread;
  use std::time::Duration;

  use super::*;

  #[tokio::test]
  async fn known_length_non_zip_mesh_bytes_are_unchanged() {
    for body in [b"v 0 0 0\nf 1 1 1\n".as_slice(), b"Kaydara FBX Binary  \0\x1a\0", b"glTF\x02\0\0\0", b"ply\nformat ascii 1.0\n"] {
      let headers = format!("Content-Length: {}\r\nContent-Type: application/octet-stream\r\n", body.len());
      let url = serve_once(ok_response(&headers, body));
      assert_eq!(download_mesh_file_with_limit(&url, body.len()).await.unwrap().as_ref(), body);
    }
  }

  #[tokio::test]
  async fn declared_oversize_is_rejected_before_reading_body() {
    let url = serve_once(ok_response("Content-Length: 1000\r\n", b""));
    assert!(matches!(download_mesh_file_with_limit(&url, 4).await, Err(MeshDownloadError::TooLarge { max_bytes: 4 })));
  }

  #[tokio::test]
  async fn chunked_response_is_bounded_across_chunks() {
    let exact = serve_once(chunked_response(&[b"ab", b"cd"]));
    assert_eq!(download_mesh_file_with_limit(&exact, 4).await.unwrap().as_ref(), b"abcd");
    let oversized = serve_once(chunked_response(&[b"ab", b"cde"]));
    assert!(matches!(download_mesh_file_with_limit(&oversized, 4).await, Err(MeshDownloadError::TooLarge { max_bytes: 4 })));
  }

  #[tokio::test]
  async fn response_without_content_length_is_bounded() {
    let url = serve_once(ok_response("Content-Type: application/octet-stream\r\n", b"abcde"));
    assert!(matches!(download_mesh_file_with_limit(&url, 4).await, Err(MeshDownloadError::TooLarge { max_bytes: 4 })));
  }

  #[tokio::test]
  async fn gzip_is_bounded_after_decompression() {
    // Genuine gzip encoding of 128 'x' bytes: compressed bytes fit, decoded bytes do not.
    let gzip: &[u8] = &[31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 171, 168, 24, 88, 0, 0, 55, 138, 224, 172, 128, 0, 0, 0];
    assert!(gzip.len() < 32);
    let headers = format!("Content-Encoding: gzip\r\nContent-Length: {}\r\n", gzip.len());
    let url = serve_once(ok_response(&headers, gzip));
    assert!(matches!(download_mesh_file_with_limit(&url, 32).await, Err(MeshDownloadError::TooLarge { max_bytes: 32 })));
  }

  #[tokio::test]
  async fn redirects_still_work() {
    let target = serve_once(ok_response("Content-Length: 4\r\n", b"mesh"));
    let redirect = format!("HTTP/1.1 302 Found\r\nLocation: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", target);
    let url = serve_once(redirect.into_bytes());
    assert_eq!(download_mesh_file_with_limit(&url, 4).await.unwrap().as_ref(), b"mesh");
  }

  #[tokio::test]
  async fn bad_status_is_rejected_without_reading_body() {
    let url = serve_once(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n".to_vec());
    assert!(matches!(download_mesh_file_with_limit(&url, 4).await, Err(MeshDownloadError::BadStatus(StatusCode::SERVICE_UNAVAILABLE))));
  }

  #[tokio::test]
  async fn truncated_body_does_not_succeed() {
    let url = serve_once(ok_response("Content-Length: 10\r\n", b"abc"));
    assert!(matches!(download_mesh_file_with_limit(&url, 10).await, Err(MeshDownloadError::Request(_))));
  }

  fn ok_response(headers: &str, body: &[u8]) -> Vec<u8> {
    let mut response = format!("HTTP/1.1 200 OK\r\nConnection: close\r\n{}\r\n", headers).into_bytes();
    response.extend_from_slice(body);
    response
  }

  fn chunked_response(chunks: &[&[u8]]) -> Vec<u8> {
    let mut response = ok_response("Transfer-Encoding: chunked\r\n", b"");
    for chunk in chunks {
      write!(response, "{:x}\r\n", chunk.len()).unwrap();
      response.extend_from_slice(chunk);
      response.extend_from_slice(b"\r\n");
    }
    response.extend_from_slice(b"0\r\n\r\n");
    response
  }

  fn serve_once(response: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
      let (mut stream, _) = listener.accept().unwrap();
      stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
      let mut request = [0_u8; 2048];
      let _ = stream.read(&mut request);
      // Early size/status rejection can close the client before the body is sent.
      let _ = stream.write_all(&response);
    });
    format!("http://{address}/mesh")
  }
}
