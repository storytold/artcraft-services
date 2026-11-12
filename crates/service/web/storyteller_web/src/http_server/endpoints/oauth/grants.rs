use std::sync::Arc;

use actix_web::http::header::LOCATION;
use actix_web::{web, HttpRequest, HttpResponse};
use log::warn;
use tokens::tokens::oauth::OauthNonceToken;

use mysql_queries::queries::oauth_authorization_codes::consume_unused_oauth_codes::{consume_unused_oauth_codes, ConsumeUnusedOauthCodesArgs};
use mysql_queries::queries::oauth_consent_nonces::consume_oauth_nonce::{consume_oauth_nonce, ConsumeNonceArgs};
use mysql_queries::queries::oauth_consent_nonces::insert_oauth_nonce::{insert_oauth_nonce, InsertNonceArgs};
use mysql_queries::queries::oauth_consents::list_oauth_grants::{list_oauth_grants, ListGrantsArgs};
use mysql_queries::queries::oauth_consents::revoke_oauth_grant::{revoke_oauth_grant, RevokeGrantArgs};
use mysql_queries::queries::oauth_tokens::revoke_oauth_tokens_for_client::{revoke_oauth_tokens_for_client, RevokeOauthTokensForClientArgs};

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::oauth::crypto::token_hash;
use crate::http_server::endpoints::oauth::form::{form_value, parse_form};
use crate::http_server::endpoints::oauth::html::{grants_page, hidden_input, html_escape, sign_in_page};
use crate::http_server::user_lookup::user_session::require_user_session::require_user_session;
use crate::state::server_state::ServerState;

const NONCE_TTL_SECONDS: i64 = 600;

/// List the signed-in user's live grants. Each row carries a one-time revoke nonce.
pub async fn oauth_grants_get(http_request: HttpRequest, server_state: web::Data<Arc<ServerState>>) -> Result<HttpResponse, CommonWebError> {
  let mut connection = server_state.mysql_pool.acquire().await.map_err(|err| {
    warn!("MySQL pool error");
    CommonWebError::from_error(err)
  })?;
  let user_session = match require_user_session(&http_request, &server_state.session_checker, &mut *connection).await {
    Ok(session) => session,
    Err(CommonWebError::NotAuthorized) => return Ok(sign_in_required()),
    Err(err) => return Err(err),
  };

  let grants = list_oauth_grants(ListGrantsArgs { user_token: user_session.get_user_token().as_str(), mysql_executor: &mut *connection }).await.map_err(|err| {
    warn!("OAuth grant list failed");
    CommonWebError::from_error(err)
  })?;

  let mut rows = String::new();
  for grant in grants {
    let nonce = OauthNonceToken::generate();
    let nonce_hash = token_hash(nonce.as_str()).map_err(CommonWebError::from_anyhow_error)?;
    let request_hash = token_hash(&format!("revoke\n{}\n{}", user_session.get_user_token().as_str(), grant.client_token)).map_err(CommonWebError::from_anyhow_error)?;
    insert_oauth_nonce(InsertNonceArgs { nonce_hash: &nonce_hash, user_token: user_session.get_user_token().as_str(), client_token: &grant.client_token, request_hash: &request_hash, ttl_seconds: NONCE_TTL_SECONDS, mysql_executor: &mut *connection }).await.map_err(|err| {
      warn!("OAuth grant nonce insert failed");
      CommonWebError::from_error(err)
    })?;
    rows.push_str(&format!("<li>{}<br>{}<form method=\"post\" action=\"/v1/oauth/grants/{}/revoke\">{}<button type=\"submit\">Revoke</button></form></li>", html_escape(&grant.client_name), html_escape(&grant.scopes), html_escape(&grant.client_token), hidden_input("nonce", nonce.as_str()),));
  }

  Ok(framed(HttpResponse::Ok(), grants_page(&rows)))
}

/// Revoke one grant: the consent row, every live token, and any unused authorization code.
pub async fn oauth_grant_revoke(http_request: HttpRequest, path: web::Path<String>, body: web::Bytes, server_state: web::Data<Arc<ServerState>>) -> Result<HttpResponse, CommonWebError> {
  let client_token = path.into_inner();
  if !client_token.starts_with("oauth_client_") || client_token.len() > 64 {
    return Ok(html_message("That app was not found."));
  }
  let text = std::str::from_utf8(&body).unwrap_or("");
  let Some(pairs) = parse_form(text) else {
    return Ok(html_message("This revoke form is not valid."));
  };
  let Some(nonce) = form_value(&pairs, "nonce") else {
    return Ok(html_message("This revoke form is not valid."));
  };
  if !nonce.starts_with("oauth_nonce_") || nonce.len() > 80 {
    return Ok(html_message("This revoke form is not valid."));
  }

  let mut tx = server_state.mysql_pool.begin().await.map_err(|err| {
    warn!("MySQL pool error");
    CommonWebError::from_error(err)
  })?;
  let user_session = match require_user_session(&http_request, &server_state.session_checker, &mut *tx).await {
    Ok(session) => session,
    Err(CommonWebError::NotAuthorized) => return Ok(sign_in_required()),
    Err(err) => return Err(err),
  };
  let nonce_hash = token_hash(nonce).map_err(CommonWebError::from_anyhow_error)?;
  let request_hash = token_hash(&format!("revoke\n{}\n{client_token}", user_session.get_user_token().as_str())).map_err(CommonWebError::from_anyhow_error)?;
  let consumed = consume_oauth_nonce(ConsumeNonceArgs { nonce_hash: &nonce_hash, user_token: user_session.get_user_token().as_str(), client_token: &client_token, request_hash: &request_hash, mysql_executor: &mut *tx }).await.map_err(|err| {
    warn!("OAuth grant nonce update failed");
    CommonWebError::from_error(err)
  })?;
  if !consumed {
    return Ok(html_message("This revoke form has expired. Reload the page."));
  }

  revoke_oauth_grant(RevokeGrantArgs { user_token: user_session.get_user_token().as_str(), client_token: &client_token, mysql_executor: &mut *tx }).await.map_err(|err| {
    warn!("OAuth grant revoke failed");
    CommonWebError::from_error(err)
  })?;
  revoke_oauth_tokens_for_client(RevokeOauthTokensForClientArgs { user_token: user_session.get_user_token().as_str(), client_token: &client_token, mysql_executor: &mut *tx }).await.map_err(|err| {
    warn!("OAuth token revoke failed");
    CommonWebError::from_error(err)
  })?;
  consume_unused_oauth_codes(ConsumeUnusedOauthCodesArgs { user_token: user_session.get_user_token().as_str(), client_token: &client_token, mysql_executor: &mut *tx }).await.map_err(|err| {
    warn!("OAuth code revoke failed");
    CommonWebError::from_error(err)
  })?;
  tx.commit().await.map_err(CommonWebError::from_error)?;

  Ok(HttpResponse::Found().insert_header((LOCATION, "/v1/oauth/grants")).insert_header(("cache-control", "no-store")).finish())
}

/// HTML 401 telling the user to sign in on the website and retry.
fn sign_in_required() -> HttpResponse {
  framed(HttpResponse::Unauthorized(), sign_in_page())
}

/// HTML 400 that refuses to be framed.
fn html_message(message: &str) -> HttpResponse {
  framed(HttpResponse::BadRequest(), crate::http_server::endpoints::oauth::html::error_page(message))
}

/// Grant pages send `X-Frame-Options: DENY` and `frame-ancestors 'none'`.
fn framed(mut builder: actix_web::HttpResponseBuilder, body: String) -> HttpResponse {
  builder.content_type("text/html; charset=utf-8").insert_header(("cache-control", "no-store")).insert_header(("x-frame-options", "DENY")).insert_header(("content-security-policy", "frame-ancestors 'none'"));
  builder.body(body)
}
