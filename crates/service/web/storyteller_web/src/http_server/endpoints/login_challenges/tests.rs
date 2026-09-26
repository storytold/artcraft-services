//! Real HTTP handlers + signed cookies + disposable local MySQL databases.
//! No application URLs, production DB configuration, Redis, or external providers.
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

use actix_artcraft::sessions::user_sessions::http_user_session_manager::HttpUserSessionManager;
use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::{test, web, App, HttpRequest};
use artcraft_api_defs::users::login_challenges::*;
use mysql_queries::queries::user_login_challenges::expire_abandoned_challenges::{expire_abandoned_challenges, ExpireAbandonedChallengesArgs};
use mysql_queries::queries::user_login_challenges::lock_by_device::{lock_by_device, LockByDeviceArgs};
use mysql_testing::fixtures::login_challenges as fixtures;
use mysql_testing::fixtures::mcp_sessions::create_test_mcp_session;
use mysql_testing::fixtures::users::{create_test_user, TestUser};
use mysql_testing::isolated::IsolatedTestDatabase;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::MySqlPool;

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::user_lookup::user_session::require_user_session::require_user_session;
use crate::http_server::user_lookup::user_session::session_utils::session_checker::SessionChecker;
use super::{handlers, security};

const ORIGIN: &str = "https://app.getartcraft.com";
const BROWSER_IP: &str = "198.51.100.20";

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn both_browser_sites_can_review_approve_and_decline() {
  let h = Harness::create().await;
  let app = test::init_service(App::new().app_data(web::Data::new(h.db.pool.clone())).app_data(web::Data::new(h.signer.clone())).configure(handlers::configure).route("/protected", web::get().to(protected))).await;
  let mut minted = 0;
  for origin in ["https://getartcraft.com", "https://www.getartcraft.com", "https://app.getartcraft.com"] {
    for approve in [false, true] {
      let created: CreateLoginChallengeResponse = test::read_body_json(post(&app, &h, "create", json!({}), false).await).await;
      let approval = approval_token(&created);
      let request = test::TestRequest::post().uri("/v1/login_challenges/review").insert_header(("Origin", origin)).insert_header(("session", h.cookie.as_str())).set_json(json!({"approval_token": approval})).to_request();
      let review: ReviewLoginChallengeResponse = test::call_and_read_body_json(&app, request).await;
      assert_eq!(review.status, LoginChallengeState::Pending);
      assert_eq!(review.username, h.user.username);
      let request = test::TestRequest::post().uri("/v1/login_challenges/decide").insert_header(("Origin", origin)).insert_header(("session", h.cookie.as_str())).set_json(json!({"approval_token": approval, "approve": approve})).to_request();
      let decision: LoginChallengeResponse = test::call_and_read_body_json(&app, request).await;
      assert!(decision.maybe_signed_session.is_none());
      assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, minted);
      let polled = post(&app, &h, "poll", json!({"device_token": created.device_token}), false).await;
      if approve {
        let cookie = polled.response().cookies().find(|c| c.name() == "session").unwrap().into_owned();
        let downstream = test::call_service(&app, test::TestRequest::get().uri("/protected").cookie(cookie).to_request()).await;
        assert_eq!(downstream.status(), StatusCode::OK);
        let body: Value = test::read_body_json(downstream).await;
        assert_eq!(body["user_token"], h.user.user_token.as_str());
        minted += 1;
      } else {
        assert!(!polled.headers().contains_key("set-cookie"));
        let result: LoginChallengeResponse = test::read_body_json(polled).await;
        assert_eq!(result.maybe_failure_type, Some(LoginChallengeFailure::UserDeclined));
        assert!(result.maybe_signed_session.is_none());
      }
      assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, minted);
    }
  }
  h.db.destroy().await;
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn approval_mints_once_and_cookie_authenticates_downstream() {
  let h = Harness::create().await;
  let app = test::init_service(App::new().app_data(web::Data::new(h.db.pool.clone())).app_data(web::Data::new(h.signer.clone())).configure(handlers::configure).route("/protected", web::get().to(protected))).await;
  fixtures::set_passwordless(&h.db.pool, &h.user.user_token).await;
  let created: CreateLoginChallengeResponse = test::read_body_json(post(&app, &h, "create", json!({}), false).await).await;
  let approval = approval_token(&created);
  let hash = security::secret_hash(&created.device_token).unwrap();
  assert!(!created.verification_url.contains(&created.device_token));
  assert_eq!(fixtures::audit(&h.db.pool, &hash).await.lifetime_seconds, 1200);
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 0);

  let pending = post(&app, &h, "poll", json!({"device_token": created.device_token}), false).await;
  assert!(!pending.headers().contains_key("set-cookie"));
  assert_eq!(pending.headers().get("cache-control").unwrap(), "no-store");
  let pending: LoginChallengeResponse = test::read_body_json(pending).await;
  assert_eq!(pending.status, LoginChallengeState::Pending);
  assert!(pending.maybe_signed_session.is_none());
  assert_eq!(post(&app, &h, "review", json!({"approval_token": approval}), false).await.status(), StatusCode::UNAUTHORIZED);
  // Swapping approval and device credentials cannot expose or redeem a session.
  assert_eq!(post(&app, &h, "poll", json!({"device_token": approval}), false).await.status(), StatusCode::UNAUTHORIZED);
  assert_eq!(post(&app, &h, "review", json!({"approval_token": created.device_token}), true).await.status(), StatusCode::UNAUTHORIZED);
  let reviewed: ReviewLoginChallengeResponse = test::read_body_json(post(&app, &h, "review", json!({"approval_token": approval}), true).await).await;
  assert_eq!(reviewed.requesting_ip, h.ip);
  assert_eq!(reviewed.confirmation_code, created.confirmation_code);
  assert_eq!(reviewed.username, h.user.username);

  let approved = post(&app, &h, "decide", json!({"approval_token": approval, "approve": true}), true).await;
  assert!(!approved.headers().contains_key("set-cookie"));
  let approved: LoginChallengeResponse = test::read_body_json(approved).await;
  assert_eq!(approved.status, LoginChallengeState::Approved);
  assert!(approved.maybe_signed_session.is_none());
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 0, "Yes records consent only");

  // Two polls overlap. Both must observe the single committed session.
  let (first, retry) = tokio::join!(post(&app, &h, "poll", json!({"device_token": created.device_token}), false), post(&app, &h, "poll", json!({"device_token": created.device_token}), false));
  let cookie = first.response().cookies().find(|c| c.name() == "session").unwrap().into_owned();
  let first: LoginChallengeResponse = test::read_body_json(first).await;
  let retry: LoginChallengeResponse = test::read_body_json(retry).await;
  assert_eq!(first.status, LoginChallengeState::Redeemed);
  assert_eq!(first.maybe_signed_session, retry.maybe_signed_session, "response loss retries reuse the same session");
  assert_eq!(first.maybe_signed_session.as_deref(), Some(cookie.value()));
  assert_ne!(cookie.value(), h.cookie, "never copy the website's existing session");
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 1);
  assert!(fixtures::audit(&h.db.pool, &hash).await.session_id.is_some());
  let response = test::call_service(&app, test::TestRequest::get().uri("/protected").cookie(cookie.clone()).to_request()).await;
  assert_eq!(response.status(), StatusCode::OK);
  let downstream: Value = test::read_body_json(response).await;
  assert_eq!(downstream["user_token"], h.user.user_token.as_str());

  // The handoff expires in 20 minutes; a successfully issued session keeps its own lifetime.
  fixtures::set_challenge_deadline(&h.db.pool, &hash, -1).await;
  let expired: LoginChallengeResponse = test::read_body_json(post(&app, &h, "poll", json!({"device_token": created.device_token}), false).await).await;
  assert_eq!(expired.maybe_failure_type, Some(LoginChallengeFailure::Expired));
  assert!(expired.maybe_signed_session.is_none());
  assert_eq!(fixtures::audit(&h.db.pool, &hash).await.status, "redeemed");
  assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/protected").cookie(cookie).to_request()).await.status(), StatusCode::OK);
  h.db.destroy().await;
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn decline_expiry_and_invalid_consent_never_mint_sessions() {
  let h = Harness::create().await;
  let app = test::init_service(App::new().app_data(web::Data::new(h.db.pool.clone())).app_data(web::Data::new(h.signer.clone())).configure(handlers::configure)).await;
  let created: CreateLoginChallengeResponse = test::read_body_json(post(&app, &h, "create", json!({}), false).await).await;
  let approval = approval_token(&created);
  let hash = security::secret_hash(&created.device_token).unwrap();
  for origin in [None, Some("https://evil.example"), Some("null")] {
    let mut request = test::TestRequest::post().uri("/v1/login_challenges/decide").insert_header(("session", h.cookie.as_str())).set_json(json!({"approval_token": approval, "approve": true}));
    if let Some(origin) = origin {
      request = request.insert_header(("Origin", origin));
    }
    assert_eq!(test::call_service(&app, request.to_request()).await.status(), StatusCode::FORBIDDEN);
  }
  let mcp = create_test_mcp_session(&h.db.pool, &h.user.user_token).await.unwrap();
  let request = test::TestRequest::post().uri("/v1/login_challenges/decide").insert_header(("Origin", ORIGIN)).insert_header(("Authorization", format!("Bearer {}", mcp.private_session_token.as_str()))).set_json(json!({"approval_token": approval, "approve": true})).to_request();
  assert_eq!(test::call_service(&app, request).await.status(), StatusCode::UNAUTHORIZED);
  let declined: LoginChallengeResponse = test::read_body_json(post(&app, &h, "decide", json!({"approval_token": approval, "approve": false}), true).await).await;
  assert_eq!(declined.maybe_failure_type, Some(LoginChallengeFailure::UserDeclined));
  // A later Yes cannot overwrite No.
  let replay: LoginChallengeResponse = test::read_body_json(post(&app, &h, "decide", json!({"approval_token": approval, "approve": true}), true).await).await;
  assert_eq!(replay.maybe_failure_type, Some(LoginChallengeFailure::UserDeclined));
  let rejected: LoginChallengeResponse = test::read_body_json(post(&app, &h, "poll", json!({"device_token": created.device_token}), false).await).await;
  assert_eq!(rejected.status, LoginChallengeState::Failed);
  assert!(rejected.maybe_signed_session.is_none());
  let audit = fixtures::audit(&h.db.pool, &hash).await;
  assert_eq!(audit.creation_ip, h.ip);
  assert_eq!(audit.failure_ip.as_deref(), Some(BROWSER_IP));
  assert_eq!(audit.failure.as_deref(), Some("user_declined"));
  assert!(audit.session_id.is_none());

  for approve_first in [false, true] {
    let created: CreateLoginChallengeResponse = test::read_body_json(post(&app, &h, "create", json!({}), false).await).await;
    if approve_first {
      assert_eq!(post(&app, &h, "decide", json!({"approval_token": approval_token(&created), "approve": true}), true).await.status(), StatusCode::OK);
    }
    let hash = security::secret_hash(&created.device_token).unwrap();
    fixtures::set_challenge_deadline(&h.db.pool, &hash, 0).await;
    let result: LoginChallengeResponse = test::read_body_json(post(&app, &h, "poll", json!({"device_token": created.device_token}), false).await).await;
    assert_eq!(result.maybe_failure_type, Some(LoginChallengeFailure::Expired));
    assert!(result.maybe_signed_session.is_none());
    let audit = fixtures::audit(&h.db.pool, &hash).await;
    assert_eq!(audit.status, "failed");
    assert!(audit.failure_ip.is_none());
  }
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 0);
  h.db.destroy().await;
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn expired_revoked_and_banned_sessions_fail_closed() {
  let h = Harness::create().await;
  let app = test::init_service(App::new().app_data(web::Data::new(h.db.pool.clone())).app_data(web::Data::new(h.signer.clone())).configure(handlers::configure).route("/protected", web::get().to(protected))).await;
  for revoke in [false, true] {
    let created: CreateLoginChallengeResponse = test::read_body_json(post(&app, &h, "create", json!({}), false).await).await;
    post(&app, &h, "decide", json!({"approval_token": approval_token(&created), "approve": true}), true).await;
    let redeemed: LoginChallengeResponse = test::read_body_json(post(&app, &h, "poll", json!({"device_token": created.device_token}), false).await).await;
    let signed = redeemed.maybe_signed_session.unwrap();
    let request = test::TestRequest::default().insert_header(("session", signed.as_str())).to_http_request();
    let token = h.signer.decode_session_payload_from_request(&request).unwrap().unwrap().session_token;
    if revoke {
      fixtures::revoke_session(&h.db.pool, &token).await;
    } else {
      fixtures::expire_session(&h.db.pool, &token).await;
    }
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/protected").insert_header(("session", signed)).to_request()).await.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(post(&app, &h, "poll", json!({"device_token": created.device_token}), false).await.status(), StatusCode::UNAUTHORIZED);
  }
  let count = fixtures::bridge_session_count(&h.db.pool).await;
  let created: CreateLoginChallengeResponse = test::read_body_json(post(&app, &h, "create", json!({}), false).await).await;
  post(&app, &h, "decide", json!({"approval_token": approval_token(&created), "approve": true}), true).await;
  fixtures::ban_user(&h.db.pool, &h.user.user_token).await;
  assert_eq!(post(&app, &h, "poll", json!({"device_token": created.device_token}), false).await.status(), StatusCode::UNAUTHORIZED);
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, count);
  h.db.destroy().await;
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn expiry_is_checked_after_waiting_for_the_redemption_lock() {
  let h = Harness::create().await;
  let app = test::init_service(App::new().app_data(web::Data::new(h.db.pool.clone())).app_data(web::Data::new(h.signer.clone())).configure(handlers::configure)).await;
  let created: CreateLoginChallengeResponse = test::read_body_json(post(&app, &h, "create", json!({}), false).await).await;
  post(&app, &h, "decide", json!({"approval_token": approval_token(&created), "approve": true}), true).await;
  let hash = security::secret_hash(&created.device_token).unwrap();
  fixtures::set_challenge_deadline(&h.db.pool, &hash, 1).await;
  let mut lock = h.db.pool.begin().await.unwrap();
  lock_by_device(LockByDeviceArgs {
    device_hash: &hash,
    mysql_executor: &mut *lock,
  }).await.unwrap().unwrap();
  let (response, _) = tokio::join!(post(&app, &h, "poll", json!({"device_token": created.device_token}), false), async {
    tokio::time::sleep(Duration::from_millis(1500)).await;
    lock.commit().await.unwrap();
  });
  let result: LoginChallengeResponse = test::read_body_json(response).await;
  assert_eq!(result.maybe_failure_type, Some(LoginChallengeFailure::Expired));
  assert!(result.maybe_signed_session.is_none());
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 0);
  h.db.destroy().await;
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn competing_decisions_are_immutable_and_abandoned_requests_expire() {
  let h = Harness::create().await;
  let app = test::init_service(App::new().app_data(web::Data::new(h.db.pool.clone())).app_data(web::Data::new(h.signer.clone())).configure(handlers::configure)).await;
  let request = test::TestRequest::post().uri("/v1/login_challenges/create").peer_addr("[2001:db8::42]:1234".parse().unwrap()).set_json(json!({})).to_request();
  let created: CreateLoginChallengeResponse = test::read_body_json(test::call_service(&app, request).await).await;
  let hash = security::secret_hash(&created.device_token).unwrap();
  assert_eq!(fixtures::audit(&h.db.pool, &hash).await.creation_ip, "2001:db8::42");
  let approval = approval_token(&created);
  let (yes, no) = tokio::join!(post(&app, &h, "decide", json!({"approval_token": approval, "approve": true}), true), post(&app, &h, "decide", json!({"approval_token": approval, "approve": false}), true));
  let yes: LoginChallengeResponse = test::read_body_json(yes).await;
  let no: LoginChallengeResponse = test::read_body_json(no).await;
  assert_eq!(yes.status, no.status);
  assert_eq!(yes.maybe_failure_type, no.maybe_failure_type);
  assert!(matches!(yes.status, LoginChallengeState::Approved | LoginChallengeState::Failed));
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 0);
  // Sweep both pending and approved challenges even if the desktop never polls.
  for approve_first in [false, true] {
    let created: CreateLoginChallengeResponse = test::read_body_json(post(&app, &h, "create", json!({}), false).await).await;
    if approve_first {
      post(&app, &h, "decide", json!({"approval_token": approval_token(&created), "approve": true}), true).await;
    }
    let hash = security::secret_hash(&created.device_token).unwrap();
    fixtures::set_challenge_deadline(&h.db.pool, &hash, -1).await;
    assert_eq!(expire_abandoned_challenges(ExpireAbandonedChallengesArgs {
      mysql_executor: &h.db.pool,
    }).await.unwrap(), 1);
    let audit = fixtures::audit(&h.db.pool, &hash).await;
    assert_eq!(audit.failure.as_deref(), Some("expired"));
    assert!(audit.failure_ip.is_none());
    assert!(audit.session_id.is_none());
  }
  assert_eq!(expire_abandoned_challenges(ExpireAbandonedChallengesArgs {
    mysql_executor: &h.db.pool,
  }).await.unwrap(), 0);
  h.db.destroy().await;
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn impersonated_expired_and_revoked_browser_sessions_cannot_approve() {
  let h = Harness::create().await;
  let app = test::init_service(App::new().app_data(web::Data::new(h.db.pool.clone())).app_data(web::Data::new(h.signer.clone())).configure(handlers::configure)).await;
  let created: CreateLoginChallengeResponse = test::read_body_json(post(&app, &h, "create", json!({}), false).await).await;
  let approval = approval_token(&created);
  for mode in ["impersonated", "expired", "revoked"] {
    let user = create_test_user(&h.db.pool).await.unwrap();
    match mode {
      "impersonated" => fixtures::impersonate_session(&h.db.pool, user.session_token.as_str(), &h.user.user_token).await,
      "expired" => fixtures::expire_session(&h.db.pool, user.session_token.as_str()).await,
      _ => fixtures::revoke_session(&h.db.pool, user.session_token.as_str()).await,
    }
    let cookie = h.signer.create_cookie(&user.session_token, &user.user_token).unwrap();
    let request = test::TestRequest::post().uri("/v1/login_challenges/decide").insert_header(("Origin", ORIGIN)).cookie(cookie).set_json(json!({"approval_token": approval, "approve": true})).to_request();
    assert_eq!(test::call_service(&app, request).await.status(), StatusCode::UNAUTHORIZED, "{mode}");
  }
  let hash = security::secret_hash(&created.device_token).unwrap();
  assert_eq!(fixtures::audit(&h.db.pool, &hash).await.status, "pending");
  assert_eq!(fixtures::bridge_session_count(&h.db.pool).await, 0);
  h.db.destroy().await;
}

struct Harness {
  db: IsolatedTestDatabase,
  user: TestUser,
  signer: HttpUserSessionManager,
  cookie: String,
  ip: String,
}

impl Harness {
  async fn create() -> Self {
    static NEXT_IP: AtomicU16 = AtomicU16::new(1);
    let db = IsolatedTestDatabase::create().await;
    let user = create_test_user(&db.pool).await.unwrap();
    let signer = HttpUserSessionManager::new("localhost", "bridge_integration_test_secret").unwrap();
    let cookie = signer.create_cookie(&user.session_token, &user.user_token).unwrap().value().to_owned();
    let ip = format!("192.0.2.{}", NEXT_IP.fetch_add(1, Ordering::Relaxed));
    Self { db, user, signer, cookie, ip }
  }
}

async fn post<S, B>(app: &S, h: &Harness, action: &str, body: impl Serialize, authenticated: bool) -> ServiceResponse<B>
where
  S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
  B: MessageBody,
{
  let ip = if action == "review" || action == "decide" { BROWSER_IP } else { &h.ip };
  let mut request = test::TestRequest::post().uri(&format!("/v1/login_challenges/{action}")).insert_header(("Origin", ORIGIN)).peer_addr(format!("{ip}:1234").parse().unwrap()).set_json(body);
  if authenticated {
    request = request.insert_header(("session", h.cookie.as_str()));
  }
  test::call_service(app, request.to_request()).await
}

fn approval_token(created: &CreateLoginChallengeResponse) -> String {
  created.verification_url.split_once("#approval_token=").unwrap().1.to_owned()
}

/// Probe the SAME production authentication helper used by protected endpoints,
/// with the cookie signer and SQL lookups, rather than merely decoding a JWT.
async fn protected(request: HttpRequest, pool: web::Data<MySqlPool>, signer: web::Data<HttpUserSessionManager>) -> Result<web::Json<Value>, CommonWebError> {
  let checker = SessionChecker::new(&signer);
  let user = require_user_session(&request, &checker, &**pool).await?;
  Ok(web::Json(json!({"user_token": user.user_token, "username": user.username})))
}
