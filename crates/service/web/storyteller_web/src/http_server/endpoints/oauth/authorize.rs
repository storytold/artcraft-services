use std::sync::Arc;

use actix_web::http::header::LOCATION;
use actix_web::{web, HttpRequest, HttpResponse};
use log::{info, warn};
use serde::Deserialize;
use tokens::tokens::oauth::{OauthCodeToken, OauthNonceToken};

use http_server_common::request::get_request_ip::get_request_ip;
use mysql_queries::queries::oauth_authorization_codes::insert_oauth_code::{insert_oauth_code, InsertCodeArgs};
use mysql_queries::queries::oauth_client_redirects::redirect_matches::{redirect_matches, RedirectMatchesArgs};
use mysql_queries::queries::oauth_clients::get_oauth_client::{get_oauth_client, GetOauthClientArgs};
use mysql_queries::queries::oauth_consent_nonces::consume_oauth_nonce::{consume_oauth_nonce, ConsumeNonceArgs};
use mysql_queries::queries::oauth_consent_nonces::insert_oauth_nonce::{insert_oauth_nonce, InsertNonceArgs};
use mysql_queries::queries::oauth_consents::save_oauth_consent::{save_oauth_consent, SaveConsentArgs};

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::oauth::crypto::token_hash;
use crate::http_server::endpoints::oauth::form::{form_value, parse_form};
use crate::http_server::endpoints::oauth::html::{consent_page, error_page, hidden_input, sign_in_page};
use crate::http_server::endpoints::oauth::pkce::code_challenge_is_valid;
use crate::http_server::endpoints::oauth::redirect::{redirect_uri_is_allowed, redirect_with_query};
use crate::http_server::endpoints::oauth::scopes::{canonicalize_scopes, scopes_from_canonical};
use crate::http_server::user_lookup::user_session::require_user_session::require_user_session;
use crate::state::server_state::ServerState;

const CODE_TTL_SECONDS: i64 = 300;
const NONCE_TTL_SECONDS: i64 = 600;

#[derive(Deserialize)]
pub struct AuthorizeQuery {
  response_type: Option<String>,
  client_id: Option<String>,
  redirect_uri: Option<String>,
  scope: Option<String>,
  state: Option<String>,
  code_challenge: Option<String>,
  code_challenge_method: Option<String>,
  audience: Option<String>,
}

struct AuthorizeRequest {
  client_id: String,
  redirect_uri: String,
  scopes: String,
  state: String,
  code_challenge: String,
  audience: String,
}

/// Render the consent page for a valid authorization-code request, or an error page when the request is not.
pub async fn oauth_authorize_get(http_request: HttpRequest, query: web::Query<AuthorizeQuery>, server_state: web::Data<Arc<ServerState>>) -> Result<HttpResponse, CommonWebError> {
  let request = match authorize_request_from_parts(parts_from_query(&query)) {
    Some(request) => request,
    None => return Ok(html_error("This authorization link is not valid.")),
  };
  render_consent(&http_request, &server_state, request).await
}

/// Approve or deny consent. Approval stores a one-time code and redirects to the client with it.
pub async fn oauth_authorize_post(http_request: HttpRequest, body: web::Bytes, server_state: web::Data<Arc<ServerState>>) -> Result<HttpResponse, CommonWebError> {
  let text = std::str::from_utf8(&body).unwrap_or("");
  let Some(pairs) = parse_form(text) else {
    return Ok(html_error("This authorization form is not valid."));
  };
  let decision = form_value(&pairs, "decision").unwrap_or("");
  if decision != "approve" && decision != "deny" {
    return Ok(html_error("This authorization form is not valid."));
  }
  let Some(request) = authorize_request_from_parts(AuthorizeParts { maybe_response_type: form_value(&pairs, "response_type"), maybe_client_id: form_value(&pairs, "client_id"), maybe_redirect_uri: form_value(&pairs, "redirect_uri"), maybe_scope: form_value(&pairs, "scope"), maybe_state: form_value(&pairs, "state"), maybe_code_challenge: form_value(&pairs, "code_challenge"), maybe_code_challenge_method: form_value(&pairs, "code_challenge_method"), maybe_audience: form_value(&pairs, "audience") }) else {
    return Ok(html_error("This authorization form is not valid."));
  };
  let Some(nonce) = form_value(&pairs, "nonce") else {
    return Ok(html_error("This authorization form is not valid."));
  };
  if !nonce.starts_with("oauth_nonce_") || nonce.len() > 80 {
    return Ok(html_error("This authorization form is not valid."));
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
  let request_hash = request_binding_hash(user_session.get_user_token().as_str(), &request)?;
  let consumed = consume_oauth_nonce(ConsumeNonceArgs { nonce_hash: &nonce_hash, user_token: user_session.get_user_token().as_str(), client_token: &request.client_id, request_hash: &request_hash, mysql_executor: &mut *tx }).await.map_err(|err| {
    warn!("OAuth nonce update failed");
    CommonWebError::from_error(err)
  })?;
  if !consumed {
    return Ok(html_error("This authorization form has expired. Open the link again."));
  }

  let allowed = redirect_matches(RedirectMatchesArgs { client_token: &request.client_id, redirect_uri: &request.redirect_uri, mysql_executor: &mut *tx }).await.map_err(|err| {
    warn!("OAuth redirect lookup failed");
    CommonWebError::from_error(err)
  })?;
  if !allowed {
    tx.commit().await.map_err(CommonWebError::from_error)?;
    return Ok(html_error("This app is not allowed to use that redirect."));
  }

  if decision == "deny" {
    tx.commit().await.map_err(CommonWebError::from_error)?;
    return Ok(redirect_response(&redirect_with_query(&request.redirect_uri, &[("error", "access_denied"), ("state", &request.state)])));
  }

  save_oauth_consent(SaveConsentArgs { user_token: user_session.get_user_token().as_str(), client_token: &request.client_id, scopes: &request.scopes, mysql_executor: &mut *tx }).await.map_err(|err| {
    warn!("OAuth consent save failed");
    CommonWebError::from_error(err)
  })?;

  let code = OauthCodeToken::generate();
  let code_hash = token_hash(code.as_str()).map_err(CommonWebError::from_anyhow_error)?;
  insert_oauth_code(InsertCodeArgs { code_hash: &code_hash, client_token: &request.client_id, user_token: user_session.get_user_token().as_str(), redirect_uri: &request.redirect_uri, scopes: &request.scopes, code_challenge: &request.code_challenge, audience: &request.audience, ttl_seconds: CODE_TTL_SECONDS, mysql_executor: &mut *tx }).await.map_err(|err| {
    warn!("OAuth code insert failed");
    CommonWebError::from_error(err)
  })?;
  tx.commit().await.map_err(CommonWebError::from_error)?;

  info!("OAuth authorization code issued for client {} from {}", request.client_id, get_request_ip(&http_request),);
  Ok(redirect_response(&redirect_with_query(&request.redirect_uri, &[("code", code.as_str()), ("state", &request.state)])))
}

/// Load the client, confirm the redirect is registered, and render consent for the signed-in user.
async fn render_consent(http_request: &HttpRequest, server_state: &ServerState, request: AuthorizeRequest) -> Result<HttpResponse, CommonWebError> {
  let mut connection = server_state.mysql_pool.acquire().await.map_err(|err| {
    warn!("MySQL pool error");
    CommonWebError::from_error(err)
  })?;

  let client = get_oauth_client(GetOauthClientArgs { token: &request.client_id, mysql_executor: &mut *connection }).await.map_err(|err| {
    warn!("OAuth client lookup failed");
    CommonWebError::from_error(err)
  })?;
  let Some(client) = client else {
    return Ok(html_error("This app is not registered."));
  };
  let allowed = redirect_matches(RedirectMatchesArgs { client_token: &request.client_id, redirect_uri: &request.redirect_uri, mysql_executor: &mut *connection }).await.map_err(|err| {
    warn!("OAuth redirect lookup failed");
    CommonWebError::from_error(err)
  })?;
  if !allowed {
    return Ok(html_error("This app is not allowed to use that redirect."));
  }

  let user_session = match require_user_session(http_request, &server_state.session_checker, &mut *connection).await {
    Ok(session) => session,
    Err(CommonWebError::NotAuthorized) => return Ok(sign_in_required()),
    Err(err) => return Err(err),
  };

  let nonce = OauthNonceToken::generate();
  let nonce_hash = token_hash(nonce.as_str()).map_err(CommonWebError::from_anyhow_error)?;
  let request_hash = request_binding_hash(user_session.get_user_token().as_str(), &request)?;
  insert_oauth_nonce(InsertNonceArgs { nonce_hash: &nonce_hash, user_token: user_session.get_user_token().as_str(), client_token: &request.client_id, request_hash: &request_hash, ttl_seconds: NONCE_TTL_SECONDS, mysql_executor: &mut *connection }).await.map_err(|err| {
    warn!("OAuth nonce insert failed");
    CommonWebError::from_error(err)
  })?;

  let items = scopes_from_canonical(&request.scopes).into_iter().map(|scope| format!("<li>{}</li>", crate::http_server::endpoints::oauth::html::html_escape(scope.description()))).collect::<String>();
  let fields = [hidden_input("response_type", "code"), hidden_input("client_id", &request.client_id), hidden_input("redirect_uri", &request.redirect_uri), hidden_input("scope", &request.scopes), hidden_input("state", &request.state), hidden_input("code_challenge", &request.code_challenge), hidden_input("code_challenge_method", "S256"), hidden_input("audience", &request.audience), hidden_input("nonce", nonce.as_str())].concat();

  Ok(html_ok(consent_page(&client.client_name, &items, &fields)))
}

struct AuthorizeParts<'a> {
  maybe_response_type: Option<&'a str>,
  maybe_client_id: Option<&'a str>,
  maybe_redirect_uri: Option<&'a str>,
  maybe_scope: Option<&'a str>,
  maybe_state: Option<&'a str>,
  maybe_code_challenge: Option<&'a str>,
  maybe_code_challenge_method: Option<&'a str>,
  maybe_audience: Option<&'a str>,
}

/// Copy the authorization query into the shape shared with the consent form.
fn parts_from_query(query: &AuthorizeQuery) -> AuthorizeParts<'_> {
  AuthorizeParts { maybe_response_type: query.response_type.as_deref(), maybe_client_id: query.client_id.as_deref(), maybe_redirect_uri: query.redirect_uri.as_deref(), maybe_scope: query.scope.as_deref(), maybe_state: query.state.as_deref(), maybe_code_challenge: query.code_challenge.as_deref(), maybe_code_challenge_method: query.code_challenge_method.as_deref(), maybe_audience: query.audience.as_deref() }
}

/// Accept only `response_type=code`, S256 PKCE, an allowlisted redirect, and a known audience.
fn authorize_request_from_parts(parts: AuthorizeParts<'_>) -> Option<AuthorizeRequest> {
  if parts.maybe_response_type != Some("code") {
    return None;
  }
  let client_id = parts.maybe_client_id?;
  if !client_id.starts_with("oauth_client_") || client_id.len() > 64 {
    return None;
  }
  let redirect_uri = parts.maybe_redirect_uri?;
  if !redirect_uri_is_allowed(redirect_uri) {
    return None;
  }
  let state = parts.maybe_state?;
  if !state_is_valid(state) {
    return None;
  }
  if parts.maybe_code_challenge_method != Some("S256") {
    return None;
  }
  let code_challenge = parts.maybe_code_challenge?;
  if !code_challenge_is_valid(code_challenge) {
    return None;
  }
  let audience = parts.maybe_audience?;
  let scopes = canonicalize_scopes(parts.maybe_scope?, audience)?;
  Some(AuthorizeRequest { client_id: client_id.to_string(), redirect_uri: redirect_uri.to_string(), scopes, state: state.to_string(), code_challenge: code_challenge.to_string(), audience: audience.to_string() })
}

/// OAuth `state` is required, ASCII, and at most 512 characters.
fn state_is_valid(state: &str) -> bool {
  (1..=512).contains(&state.len()) && state.chars().all(|character| character.is_ascii() && !character.is_control())
}

/// Bind a consent nonce to this user and the exact authorization request.
fn request_binding_hash(user_token: &str, request: &AuthorizeRequest) -> Result<String, CommonWebError> {
  token_hash(&format!("{user_token}\n{}\n{}\n{}\n{}\n{}\n{}", request.client_id, request.redirect_uri, request.scopes, request.state, request.code_challenge, request.audience,)).map_err(CommonWebError::from_anyhow_error)
}

/// HTML 200 that refuses to be framed.
fn html_ok(body: String) -> HttpResponse {
  framed(HttpResponse::Ok(), body)
}

/// HTML 400 that refuses to be framed.
fn html_error(message: &str) -> HttpResponse {
  framed(HttpResponse::BadRequest(), error_page(message))
}

/// HTML 401 telling the user to sign in on the website and retry.
fn sign_in_required() -> HttpResponse {
  framed(HttpResponse::Unauthorized(), sign_in_page())
}

/// Consent and error pages send `X-Frame-Options: DENY` and `frame-ancestors 'none'`.
fn framed(mut builder: actix_web::HttpResponseBuilder, body: String) -> HttpResponse {
  builder.content_type("text/html; charset=utf-8").insert_header(("cache-control", "no-store")).insert_header(("x-frame-options", "DENY")).insert_header(("content-security-policy", "frame-ancestors 'none'"));
  builder.body(body)
}

/// 302 back to the client's redirect URI. The location is not cached.
fn redirect_response(location: &str) -> HttpResponse {
  HttpResponse::Found().insert_header((LOCATION, location)).insert_header(("cache-control", "no-store")).finish()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn s256_request_with_a_state_is_accepted() {
    assert!(authorize_request_from_parts(AuthorizeParts { maybe_response_type: Some("code"), maybe_client_id: Some("oauth_client_example"), maybe_redirect_uri: Some("https://example.com/callback"), maybe_scope: Some("generate"), maybe_state: Some("state"), maybe_code_challenge: Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"), maybe_code_challenge_method: Some("S256"), maybe_audience: Some("https://api.storyteller.ai") }).is_some());
  }

  #[test]
  fn plain_pkce_and_a_missing_state_are_rejected() {
    assert!(authorize_request_from_parts(AuthorizeParts { maybe_response_type: Some("code"), maybe_client_id: Some("oauth_client_example"), maybe_redirect_uri: Some("https://example.com/callback"), maybe_scope: Some("generate"), maybe_state: Some("state"), maybe_code_challenge: Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"), maybe_code_challenge_method: Some("plain"), maybe_audience: Some("https://api.storyteller.ai") }).is_none());
    assert!(authorize_request_from_parts(AuthorizeParts { maybe_response_type: Some("code"), maybe_client_id: Some("oauth_client_example"), maybe_redirect_uri: Some("https://example.com/callback"), maybe_scope: Some("generate"), maybe_state: None, maybe_code_challenge: Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"), maybe_code_challenge_method: Some("S256"), maybe_audience: Some("https://api.storyteller.ai") }).is_none());
  }
}
