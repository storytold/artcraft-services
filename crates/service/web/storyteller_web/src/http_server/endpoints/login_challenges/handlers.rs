use actix_artcraft::sessions::user_sessions::http_user_session_manager::HttpUserSessionManager;
use actix_web::middleware::DefaultHeaders;
use actix_web::{web, HttpRequest, HttpResponse};
use artcraft_api_defs::users::login_challenges::*;
use mysql_queries::queries::user_login_challenges::create_challenge::{create_challenge, CreateChallengeArgs};
use mysql_queries::queries::user_login_challenges::decide_challenge::{decide_challenge, DecideChallengeArgs};
use mysql_queries::queries::user_login_challenges::expire_challenge::{expire_challenge, ExpireChallengeArgs};
use mysql_queries::queries::user_login_challenges::is_challenge_expired::{is_challenge_expired, IsChallengeExpiredArgs};
use mysql_queries::queries::user_login_challenges::lock_by_approval::{lock_by_approval, LockByApprovalArgs};
use mysql_queries::queries::user_login_challenges::lock_by_device::{lock_by_device, LockByDeviceArgs};
use mysql_queries::queries::user_login_challenges::login_challenge::LoginChallenge;
use mysql_queries::queries::user_login_challenges::mark_redeemed::{mark_redeemed, MarkRedeemedArgs};
use mysql_queries::queries::users::user_sessions::bridge_session::BridgeSession;
use mysql_queries::queries::users::user_sessions::create_bridge_session::{create_bridge_session, CreateBridgeSessionArgs};
use mysql_queries::queries::users::user_sessions::find_approving_session::{find_approving_session, FindApprovingSessionArgs};
use mysql_queries::queries::users::user_sessions::find_redeemed_session::{find_redeemed_session, FindRedeemedSessionArgs};
use sqlx::{Executor, MySql, MySqlPool, Transaction};
use tokens::tokens::user_login_challenges::UserLoginChallengeToken;
use tokens::tokens::user_sessions::UserSessionToken;
use tokens::tokens::users::UserToken;

use crate::http_server::common_responses::common_web_error::CommonWebError;
use super::security::{new_confirmation_code, new_secret, rate_limit, request_ip, require_approval_origin, secret_hash};

const VERIFICATION_PAGE: &str = "https://app.getartcraft.com/login/desktop";


pub fn configure(cfg: &mut web::ServiceConfig) {
  cfg.service(web::scope("/v1/login_challenges").wrap(DefaultHeaders::new().add(("Cache-Control", "no-store")).add(("Referrer-Policy", "no-referrer"))).app_data(web::JsonConfig::default().limit(1024)).route("/create", web::post().to(create)).route("/review", web::post().to(review)).route("/decide", web::post().to(decide)).route("/poll", web::post().to(poll)));
}

#[utoipa::path(post, path = "/v1/login_challenges/create", tag = "Users", request_body = CreateLoginChallengeRequest,
  responses((status = 200, body = CreateLoginChallengeResponse), (status = 429, body = CommonWebError)))]
pub async fn create(request: HttpRequest, _body: web::Json<CreateLoginChallengeRequest>, pool: web::Data<MySqlPool>) -> Result<web::Json<CreateLoginChallengeResponse>, CommonWebError> {
  let ip = request_ip(&request);
  rate_limit(&ip, true)?;
  let approval_token = new_secret()?;
  let device_token = new_secret()?;
  let approval_hash = secret_hash(&approval_token)?;
  let device_hash = secret_hash(&device_token)?;
  let code = new_confirmation_code()?;
  let token = UserLoginChallengeToken::generate();
  let mut conn = pool.acquire().await?;
  create_challenge(CreateChallengeArgs {
    token: &token,
    approval_hash: &approval_hash,
    device_hash: &device_hash,
    confirmation_code: &code,
    ip_address: &ip,
    mysql_executor: &mut *conn,
  }).await?;
  let challenge = lock_by_device(LockByDeviceArgs {
    device_hash: &device_hash,
    mysql_executor: &mut *conn,
  }).await?.ok_or(CommonWebError::NotFound)?;
  Ok(web::Json(CreateLoginChallengeResponse {
    success: true,
    device_token,
    // A fragment avoids sending the approval credential to the website's HTTP logs.
    verification_url: format!("{}#approval_token={approval_token}", verification_page(&request)),
    confirmation_code: code,
    expires_at: challenge.expires_at,
    poll_interval_seconds: 5,
  }))
}

#[utoipa::path(post, path = "/v1/login_challenges/review", tag = "Users", request_body = ReviewLoginChallengeRequest,
  responses((status = 200, body = ReviewLoginChallengeResponse), (status = 401, body = CommonWebError), (status = 403, body = CommonWebError)))]
pub async fn review(request: HttpRequest, body: web::Json<ReviewLoginChallengeRequest>, pool: web::Data<MySqlPool>, signer: web::Data<HttpUserSessionManager>) -> Result<web::Json<ReviewLoginChallengeResponse>, CommonWebError> {
  require_approval_origin(&request)?;
  rate_limit(&request_ip(&request), false)?;
  let hash = secret_hash(&body.approval_token)?;
  let mut tx = pool.begin().await?;
  let approver = approving_session(&request, &signer, &mut *tx).await?;
  let mut challenge = lock_by_approval(LockByApprovalArgs {
    approval_hash: &hash,
    mysql_executor: &mut *tx,
  }).await?.ok_or(CommonWebError::NotAuthorized)?;
  let expired = expire_locked_challenge(&mut challenge, &mut tx).await?;
  let result = outcome(&challenge, expired)?;
  tx.commit().await?;
  Ok(web::Json(ReviewLoginChallengeResponse { success: true, status: result.status, maybe_failure_type: result.maybe_failure_type, confirmation_code: challenge.confirmation_code, requesting_ip: challenge.ip_address_creation, expires_at: challenge.expires_at, username: approver.username }))
}

#[utoipa::path(post, path = "/v1/login_challenges/decide", tag = "Users", request_body = DecideLoginChallengeRequest,
  responses((status = 200, body = LoginChallengeResponse), (status = 401, body = CommonWebError), (status = 403, body = CommonWebError)))]
pub async fn decide(request: HttpRequest, body: web::Json<DecideLoginChallengeRequest>, pool: web::Data<MySqlPool>, signer: web::Data<HttpUserSessionManager>) -> Result<web::Json<LoginChallengeResponse>, CommonWebError> {
  require_approval_origin(&request)?;
  let ip = request_ip(&request);
  rate_limit(&ip, false)?;
  let hash = secret_hash(&body.approval_token)?;
  let mut tx = pool.begin().await?;
  let approver = approving_session(&request, &signer, &mut *tx).await?;
  let mut challenge = lock_by_approval(LockByApprovalArgs {
    approval_hash: &hash,
    mysql_executor: &mut *tx,
  }).await?.ok_or(CommonWebError::NotAuthorized)?;
  let mut expired = expire_locked_challenge(&mut challenge, &mut tx).await?;
  if !expired && challenge.status == "pending" {
    if decide_challenge(DecideChallengeArgs {
      challenge_id: challenge.id,
      user_token: &approver.user_token,
      ip_address: &ip,
      approve: body.approve,
      mysql_executor: &mut *tx,
    }).await? {
      challenge.status = if body.approve { "approved" } else { "failed" }.into();
      challenge.maybe_failure_type = if body.approve { None } else { Some("user_declined".into()) };
    } else {
      expired = expire_locked_challenge(&mut challenge, &mut tx).await?;
    }
  }
  // No session INSERT occurs here, including on approval. Terminal decisions are immutable.
  let result = outcome(&challenge, expired)?;
  tx.commit().await?;
  Ok(web::Json(result))
}

#[utoipa::path(post, path = "/v1/login_challenges/poll", tag = "Users", request_body = PollLoginChallengeRequest,
  responses((status = 200, body = LoginChallengeResponse), (status = 401, body = CommonWebError), (status = 429, body = CommonWebError)))]
pub async fn poll(request: HttpRequest, body: web::Json<PollLoginChallengeRequest>, pool: web::Data<MySqlPool>, signer: web::Data<HttpUserSessionManager>) -> Result<HttpResponse, CommonWebError> {
  let ip = request_ip(&request);
  rate_limit(&ip, false)?;
  let hash = secret_hash(&body.device_token)?;
  let mut tx = pool.begin().await?;
  let mut challenge = lock_by_device(LockByDeviceArgs {
    device_hash: &hash,
    mysql_executor: &mut *tx,
  }).await?.ok_or(CommonWebError::NotAuthorized)?;
  let expired = expire_locked_challenge(&mut challenge, &mut tx).await?;
  let mut result = outcome(&challenge, expired)?;
  if expired || challenge.status == "pending" || challenge.status == "failed" {
    tx.commit().await?;
    return Ok(HttpResponse::Ok().json(result));
  }
  let user = challenge.maybe_deciding_user_token.as_deref().ok_or(CommonWebError::NotAuthorized)?;
  let session = if challenge.status == "approved" {
    let session_id = create_bridge_session(CreateBridgeSessionArgs {
      user_token: user,
      ip_address: &ip,
      mysql_executor: &mut *tx,
    }).await?.ok_or(CommonWebError::NotAuthorized)?;
    let session = find_redeemed_session(FindRedeemedSessionArgs {
      session_id,
      user_token: user,
      mysql_executor: &mut *tx,
    }).await?.ok_or(CommonWebError::NotAuthorized)?;
    if !mark_redeemed(MarkRedeemedArgs {
      challenge_id: challenge.id,
      session_id: session.id,
      ip_address: &ip,
      mysql_executor: &mut *tx,
    }).await? {
      // Includes expiry during redemption. Roll back the INSERT as well.
      tx.rollback().await?;
      return Ok(HttpResponse::Ok().json(outcome(&challenge, true)?));
    }
    session
  } else if challenge.status == "redeemed" {
    let id = challenge.maybe_redeemed_user_session_id.ok_or(CommonWebError::NotAuthorized)?;
    find_redeemed_session(FindRedeemedSessionArgs {
      session_id: id,
      user_token: user,
      mysql_executor: &mut *tx,
    }).await?.ok_or(CommonWebError::NotAuthorized)?
  } else {
    return Err(CommonWebError::NotAuthorized);
  };
  let cookie = signer.create_cookie(&UserSessionToken::new(session.token), &UserToken::new(session.user_token))?;
  result.status = LoginChallengeState::Redeemed;
  result.maybe_signed_session = Some(cookie.value().to_owned());
  // The new session, challenge transition, and signing must ALL succeed first.
  tx.commit().await?;
  Ok(HttpResponse::Ok().cookie(cookie).json(result))
}

/// Both queries must run in the transaction that already holds the challenge lock.
async fn expire_locked_challenge(challenge: &mut LoginChallenge, tx: &mut Transaction<'_, MySql>) -> Result<bool, sqlx::Error> {
  let expired = is_challenge_expired(IsChallengeExpiredArgs {
    challenge_id: challenge.id,
    mysql_executor: &mut **tx,
  }).await?;
  if expired && (challenge.status == "pending" || challenge.status == "approved") {
    expire_challenge(ExpireChallengeArgs {
      challenge_id: challenge.id,
      mysql_executor: &mut **tx,
    }).await?;
    challenge.status = "failed".into();
    challenge.maybe_failure_type = Some("expired".into());
  }
  Ok(expired)
}

async fn approving_session<'c, T>(request: &HttpRequest, signer: &HttpUserSessionManager, mysql_executor: T) -> Result<BridgeSession, CommonWebError>
where
  T: Executor<'c, Database = MySql>,
{
  let payload = signer.decode_session_payload_from_request(request).map_err(|_| CommonWebError::NotAuthorized)?.ok_or(CommonWebError::NotAuthorized)?;
  find_approving_session(FindApprovingSessionArgs {
    session_token: &payload.session_token,
    mysql_executor,
  }).await?.ok_or(CommonWebError::NotAuthorized)
}

fn verification_page(request: &HttpRequest) -> &'static str {
  let host = request.connection_info().host().to_owned();
  if host.starts_with("localhost:") || host.starts_with("127.0.0.1:") {
    "http://localhost:4201/login/desktop"
  } else {
    VERIFICATION_PAGE
  }
}

fn outcome(challenge: &LoginChallenge, expired: bool) -> Result<LoginChallengeResponse, CommonWebError> {
  // Keep an explicit decline even when its deadline has since elapsed.
  let declined = challenge.maybe_failure_type.as_deref() == Some("user_declined");
  let (status, failure) = if expired && !declined {
    (LoginChallengeState::Failed, Some(LoginChallengeFailure::Expired))
  } else {
    let status = match challenge.status.as_str() {
      "pending" => LoginChallengeState::Pending,
      "approved" => LoginChallengeState::Approved,
      "redeemed" => LoginChallengeState::Redeemed,
      "failed" => LoginChallengeState::Failed,
      _ => return Err(CommonWebError::NotAuthorized),
    };
    let failure = match challenge.maybe_failure_type.as_deref() {
      Some("user_declined") => Some(LoginChallengeFailure::UserDeclined),
      Some("expired") => Some(LoginChallengeFailure::Expired),
      None => None,
      _ => return Err(CommonWebError::NotAuthorized),
    };
    (status, failure)
  };
  Ok(LoginChallengeResponse { success: true, status, maybe_failure_type: failure, maybe_signed_session: None })
}

#[cfg(test)]
mod website_origin_tests {
  use super::verification_page;
  use actix_web::test::TestRequest;

  #[test]
  fn local_challenges_point_at_the_local_approval_site() {
    let request = TestRequest::default().insert_header(("Host", "localhost:12345")).to_http_request();
    assert_eq!(verification_page(&request), "http://localhost:4201/login/desktop");
  }

  #[test]
  fn production_challenges_point_at_the_production_approval_site() {
    let request = TestRequest::default().insert_header(("Host", "api.storyteller.ai")).to_http_request();
    assert_eq!(verification_page(&request), "https://app.getartcraft.com/login/desktop");
  }
}
