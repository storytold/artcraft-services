/// FAL's queue URLs for a request submitted to `endpoint`: `(status_url, response_url)`.
///
/// FAL addresses queued requests by app (`owner/app`), not by the endpoint's full path: a request
/// sent to `fal-ai/flux/dev` is polled at `{queue}/fal-ai/flux/requests/{id}/status`. FAL's own
/// clients build the URLs the same way. Prefer the URLs a submit response returns; this is for
/// responses that leave them out.
pub fn fal_queue_urls(queue_base_url: &str, endpoint: &str, request_id: &str) -> (String, String) {
  let app: Vec<&str> = endpoint.trim_matches('/').split('/').take(2).collect();
  let response_url = format!("{}/{}/requests/{}", queue_base_url.trim_end_matches('/'), app.join("/"), request_id);
  let status_url = format!("{}/status", response_url);
  (status_url, response_url)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn uses_the_app_not_the_endpoints_full_path() {
    let (status_url, response_url) = fal_queue_urls("https://queue.fal.run", "fal-ai/flux-2-lora-gallery/multiple-angles", "abc");

    assert_eq!(status_url, "https://queue.fal.run/fal-ai/flux-2-lora-gallery/requests/abc/status");
    assert_eq!(response_url, "https://queue.fal.run/fal-ai/flux-2-lora-gallery/requests/abc");
  }

  #[test]
  fn matches_what_fal_returns_for_a_two_segment_endpoint() {
    // The status and response URLs in a real queue submit response for `fal-ai/nano-banana-pro`.
    let (status_url, response_url) = fal_queue_urls("https://queue.fal.run", "fal-ai/nano-banana-pro", "01a082d7");

    assert_eq!(status_url, "https://queue.fal.run/fal-ai/nano-banana-pro/requests/01a082d7/status");
    assert_eq!(response_url, "https://queue.fal.run/fal-ai/nano-banana-pro/requests/01a082d7");
  }

  #[test]
  fn tolerates_stray_slashes() {
    let (status_url, _) = fal_queue_urls("https://queue.fal.run/", "/fal-ai/flux/dev/", "abc");

    assert_eq!(status_url, "https://queue.fal.run/fal-ai/flux/requests/abc/status");
  }
}
