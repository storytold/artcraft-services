use actix_cors::Cors;
use actix_http::body::{BoxBody, EitherBody};
use actix_http::StatusCode;
use actix_web::dev::{ServiceResponse, Transform};
use actix_web::http::Method;
use actix_web::test;
use actix_web::test::TestRequest;
use speculoos::asserting;

/// A config containing ONLY one module's origins, finished the same way as
/// the production config (mismatched origins get a 400). Testing modules in
/// isolation proves each module allows its own origins, rather than passing
/// because some other module happens to allow them too.
pub (crate) fn isolated_cors(add_origins: fn(Cors, bool) -> Cors, is_production: bool) -> Cors {
  add_origins(Cors::default(), is_production)
      .block_on_origin_mismatch(true)
}

/// Whether `cors` allows a simple request from `origin`, for tests that check
/// many origins and report every failure at once.
pub (crate) async fn is_origin_allowed(cors: &Cors, origin: &str) -> bool {
  let status = make_test_request(cors, origin).await.status();
  assert!(
    status == StatusCode::OK || status == StatusCode::BAD_REQUEST,
    "unexpected status {} for origin {}", status, origin);
  status == StatusCode::OK
}

/// Assert that `cors` allows the exact `https://` origin but none of its
/// lookalikes: plain http, another port, a suffix attack, or a sub-subdomain.
/// Don't use this for Netlify hosts: their matcher checks the hostname only.
pub (crate) async fn assert_exact_https_origin(cors: &Cors, origin: &str) {
  let host = origin.strip_prefix("https://")
      .unwrap_or_else(|| panic!("expected an https:// origin, got {}", origin));

  assert_origin_ok(cors, origin).await;
  assert_origin_invalid(cors, &format!("http://{}", host)).await;
  assert_origin_invalid(cors, &format!("https://{}:8443", host)).await;
  assert_origin_invalid(cors, &format!("https://{}.evil.example", host)).await;
  assert_origin_invalid(cors, &format!("https://evil.{}", host)).await;
}

/// Assert that `cors` allows a Netlify project's main, branch and
/// deploy-preview hosts, and rejects other projects' hosts.
pub (crate) async fn assert_netlify_project(cors: &Cors, netlify_hostname: &str) {
  assert_origin_ok(cors, &format!("https://{}", netlify_hostname)).await;
  assert_origin_ok(cors, &format!("https://my-branch--{}", netlify_hostname)).await;
  assert_origin_ok(cors, &format!("https://deploy-preview-123--{}", netlify_hostname)).await;

  assert_origin_invalid(cors, "https://unrelated-project.netlify.app").await;
  assert_origin_invalid(cors, "https://deploy-preview-123--unrelated-project.netlify.app").await;
  assert_origin_invalid(cors, &format!("https://{}.evil.example", netlify_hostname)).await;
  // A hyphen alone isn't a branch separator; Netlify branch hosts use "--".
  assert_origin_invalid(cors, &format!("https://evil-{}", netlify_hostname)).await;
}

/// Assert that `cors` allows a Cloudflare Pages project's main, branch and
/// preview hosts, and rejects other projects' hosts.
pub (crate) async fn assert_cloudflare_pages_project(cors: &Cors, pages_hostname: &str) {
  assert_origin_ok(cors, &format!("https://{}", pages_hostname)).await;
  assert_origin_ok(cors, &format!("https://my-branch.{}", pages_hostname)).await;
  assert_origin_ok(cors, &format!("https://deploy-preview-123.{}", pages_hostname)).await;

  assert_origin_invalid(cors, "https://unrelated-project.pages.dev").await;
  assert_origin_invalid(cors, "https://deploy-preview-123.unrelated-project.pages.dev").await;
  assert_origin_invalid(cors, &format!("https://{}.evil.example", pages_hostname)).await;
  assert_origin_invalid(cors, &format!("https://evil-{}", pages_hostname)).await;
}

pub (crate) async fn assert_origin_ok(cors: &Cors, hostname: &str) {
  let response = make_test_request(cors, hostname).await;
  asserting(&format!("Hostname {} is valid", hostname))
      .that(&response.status())
      .is_equal_to(StatusCode::OK);
}

pub (crate) async fn assert_origin_invalid(cors: &Cors, hostname: &str) {
  let response = make_test_request(cors, hostname).await;
  asserting(&format!("Hostname {} is invalid", hostname))
      .that(&response.status())
      .is_equal_to(StatusCode::BAD_REQUEST);
}

/// Assert that a CORS preflight (OPTIONS + Access-Control-Request-Method)
/// for the given method is accepted from the given origin.
pub (crate) async fn assert_preflight_method_ok(cors: &Cors, hostname: &str, method: &str) {
  let response = make_preflight_request(cors, hostname, method).await;
  asserting(&format!("Preflight for {} from {} is allowed", method, hostname))
      .that(&response.status())
      .is_equal_to(StatusCode::OK);
}

/// Assert that a CORS preflight for the given method is rejected.
pub (crate) async fn assert_preflight_method_invalid(cors: &Cors, hostname: &str, method: &str) {
  let response = make_preflight_request(cors, hostname, method).await;
  asserting(&format!("Preflight for {} from {} is rejected", method, hostname))
      .that(&response.status())
      .is_equal_to(StatusCode::BAD_REQUEST);
}

/// Assert that a request WITHOUT an Origin header (curl, server-to-server,
/// native API clients) is never blocked by CORS, regardless of
/// `block_on_origin_mismatch`.
pub (crate) async fn assert_no_origin_header_ok(cors: &Cors) {
  let cors = cors.new_transform(test::ok_service())
      .await
      .unwrap();

  let request = TestRequest::default().to_srv_request();
  let response = test::call_service(&cors, request).await;

  asserting("Requests without an Origin header are not blocked")
      .that(&response.status())
      .is_equal_to(StatusCode::OK);
}

async fn make_test_request(cors: &Cors, hostname: &str) -> ServiceResponse<EitherBody<BoxBody>> {
  let cors= cors.new_transform(test::ok_service())
      .await
      .unwrap();

  let request = TestRequest::default()
      .insert_header(("Origin", hostname))
      .to_srv_request();

  test::call_service(&cors, request).await
}

async fn make_preflight_request(
  cors: &Cors,
  hostname: &str,
  method: &str,
) -> ServiceResponse<EitherBody<BoxBody>> {
  let cors = cors.new_transform(test::ok_service())
      .await
      .unwrap();

  let request = TestRequest::default()
      .method(Method::OPTIONS)
      .insert_header(("Origin", hostname))
      .insert_header(("Access-Control-Request-Method", method))
      .to_srv_request();

  test::call_service(&cors, request).await
}
