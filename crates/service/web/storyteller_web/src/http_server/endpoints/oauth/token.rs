use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse};
use log::warn;
use serde_json::json;
use tokens::tokens::oauth::{OauthAccessToken, OauthRefreshToken};

use mysql_queries::queries::oauth_authorization_codes::consume_oauth_code::{consume_oauth_code, ConsumeCodeArgs};
use mysql_queries::queries::oauth_authorization_codes::read_used_oauth_code::{read_used_oauth_code, ReadCodeArgs};
use mysql_queries::queries::oauth_clients::get_oauth_client::{get_oauth_client, GetOauthClientArgs};
use mysql_queries::queries::oauth_spent_refresh_hashes::insert_spent_refresh::{insert_spent_refresh, InsertSpentRefreshArgs};
use mysql_queries::queries::oauth_tokens::insert_oauth_token::{insert_oauth_token, InsertTokenArgs};
use mysql_queries::queries::oauth_tokens::revoke_reused_refresh::{revoke_reused_refresh, ReusedRefreshArgs};
use mysql_queries::queries::oauth_tokens::rotate_oauth_refresh::{rotate_oauth_refresh, RotateRefreshArgs};
use mysql_queries::queries::oauth_tokens::take_oauth_refresh::{take_oauth_refresh, TakeRefreshArgs};

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::oauth::crypto::token_hash;
use crate::http_server::endpoints::oauth::form::{form_value, parse_form};
use crate::http_server::endpoints::oauth::pkce::verify_s256;
use crate::state::server_state::ServerState;

const ACCESS_TTL_SECONDS: i64 = 3600;
const REFRESH_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;

pub async fn oauth_token_handler(http_request: HttpRequest, body: web::Bytes, server_state: web::Data<Arc<ServerState>>) -> Result<HttpResponse, CommonWebError> {
  if !form_content_type(&http_request) {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_request"));
  }
  let text = std::str::from_utf8(&body).unwrap_or("");
  let Some(pairs) = parse_form(text) else {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_request"));
  };
  let Some(grant_type) = form_value(&pairs, "grant_type") else {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_request"));
  };
  let Some(client_id) = form_value(&pairs, "client_id") else {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_request"));
  };
  if !client_id.starts_with("oauth_client_") || client_id.len() > 64 {
    return Ok(oauth_error(StatusCode::UNAUTHORIZED, "invalid_client"));
  }

  match grant_type {
    "authorization_code" => authorization_code(&server_state, client_id, &pairs).await,
    "refresh_token" => refresh_token(&server_state, client_id, &pairs).await,
    _ => Ok(oauth_error(StatusCode::BAD_REQUEST, "unsupported_grant_type")),
  }
}

async fn authorization_code(server_state: &ServerState, client_id: &str, pairs: &[(String, String)]) -> Result<HttpResponse, CommonWebError> {
  let Some(code) = form_value(pairs, "code") else {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_request"));
  };
  let Some(redirect_uri) = form_value(pairs, "redirect_uri") else {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_request"));
  };
  let Some(verifier) = form_value(pairs, "code_verifier") else {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_request"));
  };
  if !code.starts_with("oauth_code_") || code.len() > 128 {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_grant"));
  }

  let mut tx = server_state.mysql_pool.begin().await.map_err(pool_error)?;
  let client = get_oauth_client(GetOauthClientArgs { token: client_id, mysql_executor: &mut *tx }).await.map_err(query_error)?;
  if client.is_none() {
    return Ok(oauth_error(StatusCode::UNAUTHORIZED, "invalid_client"));
  }

  let code_hash = token_hash(code).map_err(CommonWebError::from_anyhow_error)?;
  let consumed = consume_oauth_code(ConsumeCodeArgs { code_hash: &code_hash, mysql_executor: &mut *tx }).await.map_err(query_error)?;
  if !consumed {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_grant"));
  }
  let row = read_used_oauth_code(ReadCodeArgs { code_hash: &code_hash, mysql_executor: &mut *tx }).await.map_err(query_error)?;
  let Some(row) = row else {
    tx.commit().await.map_err(CommonWebError::from_error)?;
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_grant"));
  };

  let pkce_ok = verify_s256(verifier, &row.code_challenge).map_err(CommonWebError::from_anyhow_error)?;
  let binding_ok = row.client_token == client_id && row.redirect_uri == redirect_uri;
  if !pkce_ok || !binding_ok {
    tx.commit().await.map_err(CommonWebError::from_error)?;
    warn!("OAuth authorization code rejected");
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_grant"));
  }

  let access = OauthAccessToken::generate();
  let refresh = OauthRefreshToken::generate();
  let access_hash = token_hash(access.as_str()).map_err(CommonWebError::from_anyhow_error)?;
  let refresh_hash = token_hash(refresh.as_str()).map_err(CommonWebError::from_anyhow_error)?;
  insert_oauth_token(InsertTokenArgs { access_hash: &access_hash, refresh_hash: &refresh_hash, client_token: &row.client_token, user_token: &row.user_token, scopes: &row.scopes, audience: &row.audience, access_ttl_seconds: ACCESS_TTL_SECONDS, refresh_ttl_seconds: REFRESH_TTL_SECONDS, mysql_executor: &mut *tx }).await.map_err(query_error)?;
  tx.commit().await.map_err(CommonWebError::from_error)?;
  Ok(token_response(access.as_str(), refresh.as_str(), &row.scopes))
}

async fn refresh_token(server_state: &ServerState, client_id: &str, pairs: &[(String, String)]) -> Result<HttpResponse, CommonWebError> {
  let Some(refresh) = form_value(pairs, "refresh_token") else {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_request"));
  };
  if !refresh.starts_with("oauth_refresh_") || refresh.len() > 128 {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_grant"));
  }

  let mut tx = server_state.mysql_pool.begin().await.map_err(pool_error)?;
  let client = get_oauth_client(GetOauthClientArgs { token: client_id, mysql_executor: &mut *tx }).await.map_err(query_error)?;
  if client.is_none() {
    return Ok(oauth_error(StatusCode::UNAUTHORIZED, "invalid_client"));
  }

  let refresh_hash = token_hash(refresh).map_err(CommonWebError::from_anyhow_error)?;
  let row = take_oauth_refresh(TakeRefreshArgs { refresh_hash: &refresh_hash, mysql_executor: &mut *tx }).await.map_err(query_error)?;
  let Some(row) = row else {
    let reused = revoke_reused_refresh(ReusedRefreshArgs { spent_refresh_hash: &refresh_hash, mysql_executor: &mut *tx }).await.map_err(query_error)?;
    if reused {
      tx.commit().await.map_err(CommonWebError::from_error)?;
      warn!("OAuth refresh token reuse revoked the grant");
    }
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_grant"));
  };
  if row.client_token != client_id {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_grant"));
  }

  let access = OauthAccessToken::generate();
  let next_refresh = OauthRefreshToken::generate();
  let new_access_hash = token_hash(access.as_str()).map_err(CommonWebError::from_anyhow_error)?;
  let new_refresh_hash = token_hash(next_refresh.as_str()).map_err(CommonWebError::from_anyhow_error)?;
  let spent = insert_spent_refresh(InsertSpentRefreshArgs { refresh_hash: &refresh_hash, mysql_executor: &mut *tx }).await.map_err(query_error)?;
  if !spent {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_grant"));
  }
  let rotated = rotate_oauth_refresh(RotateRefreshArgs { old_refresh_hash: &refresh_hash, client_token: client_id, new_access_hash: &new_access_hash, new_refresh_hash: &new_refresh_hash, access_ttl_seconds: ACCESS_TTL_SECONDS, mysql_executor: &mut *tx }).await.map_err(query_error)?;
  if !rotated {
    return Ok(oauth_error(StatusCode::BAD_REQUEST, "invalid_grant"));
  }
  tx.commit().await.map_err(CommonWebError::from_error)?;
  Ok(token_response(access.as_str(), next_refresh.as_str(), &row.scopes))
}

fn form_content_type(http_request: &HttpRequest) -> bool {
  http_request.headers().get("content-type").and_then(|value| value.to_str().ok()).map(|value| value.split(';').next().unwrap_or("").trim().eq_ignore_ascii_case("application/x-www-form-urlencoded")).unwrap_or(false)
}

fn token_response(access: &str, refresh: &str, scopes: &str) -> HttpResponse {
  HttpResponse::Ok().insert_header(("cache-control", "no-store")).insert_header(("pragma", "no-cache")).json(json!({
    "access_token": access,
    "token_type": "Bearer",
    "expires_in": ACCESS_TTL_SECONDS,
    "refresh_token": refresh,
    "scope": scopes,
  }))
}

fn oauth_error(status: StatusCode, error: &str) -> HttpResponse {
  HttpResponse::build(status).insert_header(("cache-control", "no-store")).json(json!({ "error": error }))
}

fn pool_error(err: sqlx::Error) -> CommonWebError {
  warn!("MySQL pool error");
  CommonWebError::from_error(err)
}

fn query_error(err: sqlx::Error) -> CommonWebError {
  warn!("OAuth token query failed");
  CommonWebError::from_error(err)
}
