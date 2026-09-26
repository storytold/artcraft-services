//! End-to-end HTTP endpoint tests: real TCP, production handlers/signers, real MySQL fixtures.
//! The API binds only 127.0.0.1:0; MySQL uses the isolated fixed-local-socket harness.
//! No mocks of queries, login handlers, session responses, or cookies.
use std::net::TcpListener;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;

use actix_artcraft::sessions::anonymous_visitor_tracking::avt_cookie_manager::AvtCookieManager;
use actix_artcraft::sessions::user_sessions::http_user_session_manager::HttpUserSessionManager;
use actix_web::dev::ServerHandle;
use actix_web::{web, App, HttpServer};
use artcraft_api_defs::users::login_challenges::{CreateLoginChallengeResponse, LoginChallengeFailure, LoginChallengeResponse, LoginChallengeState, ReviewLoginChallengeResponse};
use enums::by_table::users::user_feature_flag::UserFeatureFlag;
use mysql_testing::fixtures::login_challenges as fixtures;
use mysql_testing::fixtures::users::{create_test_user, TestUser};
use mysql_testing::isolated::IsolatedTestDatabase;
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderValue, SET_COOKIE};
use reqwest::{Client, Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::http_server::endpoints::users::session_info_handler::session_info_handler;
use crate::http_server::user_lookup::user_session::session_utils::session_checker::SessionChecker;
use super::{handlers, security};

const BROWSER_ORIGIN: &str = "https://getartcraft.com";
const TEST_SIGNING_KEY: &str = "local_endpoint_integration_test_key";

struct EndpointHarness {
  db: IsolatedTestDatabase,
  user: TestUser,
  signer: HttpUserSessionManager,
  server: ServerHandle,
  base_url: Url,
  desktop: Client,
  browser: Client,
  desktop_ip: String,
  browser_ip: String,
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn approved_passwordless_login_redeems_once_and_authenticates_real_session_endpoint() {
  let h = EndpointHarness::start().await;
  assert_anonymous(h.session(&h.desktop).await);
  let browser_session = h.session(&h.browser).await;
  assert_eq!(browser_session["user"]["user_token"], h.user.user_token.as_str());

  let created = h.create().await;
  let hash = security::secret_hash(&created.device_token).unwrap();
  let initial = fixtures::audit(&h.db.pool, &hash).await;
  assert_eq!(initial.lifetime_seconds, 1200);
  assert_eq!(initial.creation_ip, h.desktop_ip);
  assert!(initial.deciding_user.is_none());
  assert!(initial.decided_at.is_none() && initial.redeemed_at.is_none() && initial.failed_at.is_none());
  assert!(initial.session_id.is_none());
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 0);

  let pending = h.poll(&created).await;
  assert_private_response(&pending);
  assert_no_session_cookie(&pending);
  let pending: LoginChallengeResponse = read_ok(pending).await;
  assert_eq!(pending.status, LoginChallengeState::Pending);
  assert!(pending.maybe_signed_session.is_none());
  assert_eq!(h.review(&h.desktop, &created).await.status(), StatusCode::UNAUTHORIZED);

  let review: ReviewLoginChallengeResponse = read_ok(h.review(&h.browser, &created).await).await;
  assert_eq!(review.status, LoginChallengeState::Pending);
  assert_eq!(review.requesting_ip, h.desktop_ip);
  assert_eq!(review.confirmation_code, created.confirmation_code);
  assert_eq!(review.username, h.user.username);
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 0);

  let approved = h.decide(&h.browser, &created, true).await;
  assert_private_response(&approved);
  assert_no_session_cookie(&approved);
  let approved: LoginChallengeResponse = read_ok(approved).await;
  assert_eq!(approved.status, LoginChallengeState::Approved);
  assert!(approved.maybe_signed_session.is_none());
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 0, "approval must not mint a session");
  assert_anonymous(h.session(&h.desktop).await);
  let approved_audit = fixtures::audit(&h.db.pool, &hash).await;
  assert_eq!(approved_audit.deciding_user.as_deref(), Some(h.user.user_token.as_str()));
  assert_eq!(approved_audit.decision_ip.as_deref(), Some(h.browser_ip.as_str()));
  assert!(approved_audit.decided_at.is_some());
  assert!(approved_audit.redeemed_at.is_none() && approved_audit.session_id.is_none());

  // Overlapping real HTTP requests serialize through the production row lock.
  let (first, retry) = tokio::join!(h.poll(&created), h.poll(&created));
  assert_private_response(&first);
  let cookie = session_cookie(&first).expect("redemption Set-Cookie");
  assert_eq!(cookie.path(), Some("/"));
  let first: LoginChallengeResponse = read_ok(first).await;
  let retry: LoginChallengeResponse = read_ok(retry).await;
  assert_eq!(first.status, LoginChallengeState::Redeemed);
  assert_eq!(first.maybe_signed_session, retry.maybe_signed_session);
  assert_eq!(first.maybe_signed_session.as_deref(), Some(cookie.value()));
  assert_ne!(cookie.value(), h.signer.create_cookie(&h.user.session_token, &h.user.user_token).unwrap().value());
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 1);

  // No injected cookie or canned response: reqwest sends the Set-Cookie it received.
  let session = h.session(&h.desktop).await;
  assert_eq!(session["success"], true);
  assert_eq!(session["logged_in"], true);
  assert_eq!(session["user"]["user_token"], h.user.user_token.as_str());
  assert_eq!(session["user"]["core_info"]["user_token"], h.user.user_token.as_str());
  assert_eq!(session["user"]["username"], h.user.username);
  assert_eq!(session["user"]["onboarding"]["password_not_set"], true);
  assert_eq!(session["user"]["onboarding"]["email_not_confirmed"], false);
  assert_eq!(session["user"]["maybe_feature_flags"], json!(["studio", "use_qt"]));
  assert_eq!(session["user"]["can_access_studio"], true);
  assert_eq!(session["user"]["can_ban_users"], false);
  assert!(session.get("signed_session").is_none());
  let audit = fixtures::audit(&h.db.pool, &hash).await;
  assert_eq!(audit.status, "redeemed");
  assert_eq!(audit.redemption_ip.as_deref(), Some(h.desktop_ip.as_str()));
  assert!(audit.session_id.is_some());
  assert!(audit.redeemed_at.unwrap() >= audit.decided_at.unwrap());
  assert!(audit.failed_at.is_none());

  // Losing the first response is recoverable from a fresh client with no cookies.
  let fresh = h.client("192.0.2.250", None);
  let replay: LoginChallengeResponse = read_ok(h.post(&fresh, "poll", json!({"device_token":created.device_token})).await).await;
  assert_eq!(replay.maybe_signed_session, first.maybe_signed_session);
  assert_eq!(h.session(&fresh).await["user"]["user_token"], h.user.user_token.as_str());
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 1);

  fixtures::set_challenge_deadline(&h.db.pool, &hash, -1).await;
  let expired = h.poll(&created).await;
  assert_no_session_cookie(&expired);
  let expired: LoginChallengeResponse = read_ok(expired).await;
  assert_eq!(expired.maybe_failure_type, Some(LoginChallengeFailure::Expired));
  assert!(expired.maybe_signed_session.is_none());
  assert_eq!(h.session(&h.desktop).await["logged_in"], true, "session outlives the handoff");
  h.finish().await;
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn rejected_and_expired_requests_never_authenticate_the_desktop() {
  let h = EndpointHarness::start().await;
  for mode in ["declined", "expired_pending", "expired_approved"] {
    let created = h.create().await;
    let hash = security::secret_hash(&created.device_token).unwrap();
    let review: ReviewLoginChallengeResponse = read_ok(h.review(&h.browser, &created).await).await;
    assert_eq!(review.status, LoginChallengeState::Pending);
    if mode == "declined" {
      let declined = h.decide(&h.browser, &created, false).await;
      assert_no_session_cookie(&declined);
      let declined: LoginChallengeResponse = read_ok(declined).await;
      assert_eq!(declined.maybe_failure_type, Some(LoginChallengeFailure::UserDeclined));
      let replay: LoginChallengeResponse = read_ok(h.decide(&h.browser, &created, true).await).await;
      assert_eq!(replay.maybe_failure_type, Some(LoginChallengeFailure::UserDeclined));
    } else {
      if mode == "expired_approved" {
        let approved: LoginChallengeResponse = read_ok(h.decide(&h.browser, &created, true).await).await;
        assert_eq!(approved.status, LoginChallengeState::Approved);
      }
      fixtures::set_challenge_deadline(&h.db.pool, &hash, -1).await;
    }
    let response = h.poll(&created).await;
    assert_private_response(&response);
    assert_no_session_cookie(&response);
    let result: LoginChallengeResponse = read_ok(response).await;
    assert_eq!(result.status, LoginChallengeState::Failed);
    assert_eq!(result.maybe_failure_type, Some(if mode == "declined" { LoginChallengeFailure::UserDeclined } else { LoginChallengeFailure::Expired }));
    assert!(result.maybe_signed_session.is_none());
    assert_anonymous(h.session(&h.desktop).await);
    let audit = fixtures::audit(&h.db.pool, &hash).await;
    assert!(audit.session_id.is_none() && audit.redeemed_at.is_none());
    assert!(audit.failed_at.is_some());
    assert_eq!(audit.creation_ip, h.desktop_ip);
    assert_eq!(audit.failure_ip.as_deref(), if mode == "declined" { Some(h.browser_ip.as_str()) } else { None });
    assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 0);
  }
  assert_eq!(h.session(&h.browser).await["logged_in"], true);
  h.finish().await;
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn revoked_expired_and_banned_sessions_fail_real_downstream_requests_and_cannot_be_reminted() {
  let h = EndpointHarness::start().await;
  for (index, mode) in ["revoked", "expired", "banned"].iter().copied().enumerate() {
    let created = h.create().await;
    let _: LoginChallengeResponse = read_ok(h.decide(&h.browser, &created, true).await).await;
    let result: LoginChallengeResponse = read_ok(h.poll(&created).await).await;
    assert_eq!(h.session(&h.desktop).await["logged_in"], true);
    let signed = result.maybe_signed_session.unwrap();
    let request = actix_web::test::TestRequest::default().insert_header(("session", signed)).to_http_request();
    let token = h.signer.decode_session_payload_from_request(&request).unwrap().unwrap().session_token;
    match mode {
      "revoked" => fixtures::revoke_session(&h.db.pool, &token).await,
      "expired" => fixtures::expire_session(&h.db.pool, &token).await,
      _ => fixtures::ban_user(&h.db.pool, &h.user.user_token).await,
    }
    assert_anonymous(h.session(&h.desktop).await);
    let retry = h.poll(&created).await;
    assert_eq!(retry.status(), StatusCode::UNAUTHORIZED);
    assert_no_session_cookie(&retry);
    assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, (index + 1) as i64);
  }
  h.finish().await;
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn a_second_account_cannot_replace_the_user_who_approved() {
  let h = EndpointHarness::start().await;
  let other = create_test_user(&h.db.pool).await.unwrap();
  let other_browser = h.client("198.51.100.250", Some(&other));
  let created = h.create().await;
  let _: LoginChallengeResponse = read_ok(h.decide(&h.browser, &created, true).await).await;
  for approve in [true, false] {
    let replay: LoginChallengeResponse = read_ok(h.decide(&other_browser, &created, approve).await).await;
    assert_eq!(replay.status, LoginChallengeState::Approved);
    assert!(replay.maybe_signed_session.is_none());
  }
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 0);
  let _: LoginChallengeResponse = read_ok(h.poll(&created).await).await;
  assert_eq!(h.session(&h.desktop).await["user"]["user_token"], h.user.user_token.as_str());
  assert_eq!(h.session(&other_browser).await["user"]["user_token"], other.user_token.as_str());
  let audit = fixtures::audit(&h.db.pool, &security::secret_hash(&created.device_token).unwrap()).await;
  assert_eq!(audit.deciding_user.as_deref(), Some(h.user.user_token.as_str()));
  assert_eq!(audit.decision_ip.as_deref(), Some(h.browser_ip.as_str()));
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 1);
  h.finish().await;
}

impl EndpointHarness {
  async fn start() -> Self {
    static NEXT_IP: AtomicU16 = AtomicU16::new(128);
    let suffix = NEXT_IP.fetch_add(1, Ordering::Relaxed);
    let desktop_ip = format!("192.0.2.{suffix}");
    let browser_ip = format!("198.51.100.{suffix}");
    let db = IsolatedTestDatabase::create().await;
    let user = fixtures::create_passwordless_user(&db.pool, &[UserFeatureFlag::Studio, UserFeatureFlag::CanUseQuicktime]).await;
    let signer = HttpUserSessionManager::new("localhost", TEST_SIGNING_KEY).unwrap();
    let checker = SessionChecker::new(&signer);
    let avt = AvtCookieManager::new("localhost", TEST_SIGNING_KEY).unwrap();
    let pool = db.pool.clone();
    let app_signer = signer.clone();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let address = listener.local_addr().unwrap();
    assert!(address.ip().is_loopback());
    let base_url = Url::parse(&format!("http://{address}")).unwrap();
    let server = HttpServer::new(move || {
      App::new()
        .app_data(web::Data::new(pool.clone()))
        .app_data(web::Data::new(app_signer.clone()))
        .app_data(web::Data::new(checker.clone()))
        .app_data(web::Data::new(avt.clone()))
        .configure(handlers::configure)
        .route("/v1/session", web::get().to(session_info_handler))
    }).workers(1).disable_signals().listen(listener).unwrap().run();
    let handle = server.handle();
    actix_web::rt::spawn(server);
    let desktop = client(&base_url, &desktop_ip, None);
    let cookie = signer.create_cookie(&user.session_token, &user.user_token).unwrap().to_string();
    let browser = client(&base_url, &browser_ip, Some(&cookie));
    Self { db, user, signer, server: handle, base_url, desktop, browser, desktop_ip, browser_ip }
  }

  fn client(&self, ip: &str, user: Option<&TestUser>) -> Client {
    let cookie = user.map(|u| self.signer.create_cookie(&u.session_token, &u.user_token).unwrap().to_string());
    client(&self.base_url, ip, cookie.as_deref())
  }

  async fn create(&self) -> CreateLoginChallengeResponse {
    let response = self.post(&self.desktop, "create", json!({})).await;
    assert_private_response(&response);
    assert_no_session_cookie(&response);
    let created: CreateLoginChallengeResponse = read_ok(response).await;
    assert!(created.success);
    assert!(!created.verification_url.contains(&created.device_token));
    created
  }

  async fn review(&self, client: &Client, created: &CreateLoginChallengeResponse) -> Response {
    self.post(client, "review", json!({"approval_token": approval_token(created)})).await
  }

  async fn decide(&self, client: &Client, created: &CreateLoginChallengeResponse, approve: bool) -> Response {
    self.post(client, "decide", json!({"approval_token": approval_token(created), "approve": approve})).await
  }

  async fn poll(&self, created: &CreateLoginChallengeResponse) -> Response {
    self.post(&self.desktop, "poll", json!({"device_token": created.device_token})).await
  }

  async fn post(&self, client: &Client, action: &str, body: Value) -> Response {
    client.post(self.base_url.join(&format!("/v1/login_challenges/{action}")).unwrap()).json(&body).send().await.unwrap()
  }

  async fn session(&self, client: &Client) -> Value {
    read_ok(client.get(self.base_url.join("/v1/session").unwrap()).send().await.unwrap()).await
  }

  async fn finish(self) {
    self.server.stop(true).await;
    self.db.destroy().await;
  }
}

fn client(base_url: &Url, ip: &str, cookie: Option<&str>) -> Client {
  let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
  let jar = Arc::new(Jar::default());
  if let Some(cookie) = cookie { jar.add_cookie_str(cookie, base_url); }
  let mut headers = HeaderMap::new();
  headers.insert("Origin", HeaderValue::from_static(BROWSER_ORIGIN));
  // Model the production gateway forwarding different browser and desktop IPs.
  headers.insert("X-Forwarded-For", HeaderValue::from_str(ip).unwrap());
  Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none())
    .timeout(Duration::from_secs(10)).default_headers(headers).cookie_provider(jar).build().unwrap()
}

async fn read_ok<T: DeserializeOwned>(response: Response) -> T {
  assert_eq!(response.status(), StatusCode::OK);
  response.json().await.unwrap()
}

fn approval_token(created: &CreateLoginChallengeResponse) -> &str {
  created.verification_url.split_once("#approval_token=").unwrap().1
}

fn session_cookie(response: &Response) -> Option<actix_web::cookie::Cookie<'static>> {
  response.headers().get_all(SET_COOKIE).iter()
    .filter_map(|value| actix_web::cookie::Cookie::parse(value.to_str().unwrap()).ok())
    .find(|cookie| cookie.name() == "session").map(|cookie| cookie.into_owned())
}

fn assert_no_session_cookie(response: &Response) {
  assert!(session_cookie(response).is_none());
}

fn assert_private_response(response: &Response) {
  assert_eq!(response.headers()["cache-control"], "no-store");
  assert_eq!(response.headers()["referrer-policy"], "no-referrer");
}

fn assert_anonymous(session: Value) {
  assert_eq!(session["success"], true);
  assert_eq!(session["logged_in"], false);
  assert!(session["user"].is_null());
}
