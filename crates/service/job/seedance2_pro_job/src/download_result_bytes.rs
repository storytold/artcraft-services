use anyhow::anyhow;

use errors::AnyhowResult;

/// Download a finished result file (eg. a generated image) from the provider's CDN.
///
/// Non-2xx responses are errors: otherwise an expired link's error page would be
/// stored as the user's result and the job marked successful.
pub async fn download_result_bytes(client: &reqwest::Client, url: &str) -> AnyhowResult<Vec<u8>> {
  let bytes = client.get(url)
    .send()
    .await
    .map_err(|err| anyhow!("reqwest error downloading {}: {:?}", url, err))?
    .error_for_status()
    .map_err(|err| anyhow!("HTTP error downloading {}: {:?}", url, err))?
    .bytes()
    .await
    .map_err(|err| anyhow!("error reading bytes from {}: {:?}", url, err))?;

  Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
  use std::io::{Read, Write};
  use std::net::TcpListener;
  use std::thread;

  use super::*;

  #[tokio::test]
  async fn returns_body_on_success() {
    let url = serve_one_response("200 OK", b"image-bytes");
    let bytes = download_result_bytes(&test_client(), &url).await.unwrap();
    assert_eq!(bytes, b"image-bytes");
  }

  #[tokio::test]
  async fn rejects_http_error_status() {
    // An expired or missing CDN object must not be saved as the result file.
    let url = serve_one_response("404 Not Found", b"<Error>NoSuchKey</Error>");
    assert!(download_result_bytes(&test_client(), &url).await.is_err());
  }

  fn test_client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
  }

  /// Serve a single canned HTTP response on a random local port and return its URL.
  fn serve_one_response(status: &'static str, body: &'static [u8]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/result.png", listener.local_addr().unwrap());
    thread::spawn(move || {
      let (mut stream, _) = listener.accept().unwrap();
      let mut request = [0u8; 4096];
      let _ = stream.read(&mut request);
      let head = format!("HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", status, body.len());
      stream.write_all(head.as_bytes()).unwrap();
      stream.write_all(body).unwrap();
    });
    url
  }
}
