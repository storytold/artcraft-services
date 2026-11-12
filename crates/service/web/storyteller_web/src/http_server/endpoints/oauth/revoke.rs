use std::sync::Arc;

use actix_web::{web, HttpRequest, HttpResponse};
use log::warn;
use serde_json::json;

use mysql_queries::queries::oauth_tokens::revoke_oauth_token_hash::{revoke_oauth_token_hash, RevokePresentedArgs};

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::oauth::crypto::token_hash;
use crate::http_server::endpoints::oauth::form::{form_value, parse_form};
use crate::state::server_state::ServerState;

/// Revoke an access or refresh token. The response is 200 even when the token is unknown.
pub async fn oauth_revoke_handler(http_request: HttpRequest, body: web::Bytes, server_state: web::Data<Arc<ServerState>>) -> Result<HttpResponse, CommonWebError> {
  let content_type = http_request.headers().get("content-type").and_then(|value| value.to_str().ok()).unwrap_or("");
  if !content_type.split(';').next().unwrap_or("").trim().eq_ignore_ascii_case("application/x-www-form-urlencoded") {
    return Ok(revoked());
  }
  let text = std::str::from_utf8(&body).unwrap_or("");
  let Some(pairs) = parse_form(text) else {
    return Ok(revoked());
  };
  let Some(token) = form_value(&pairs, "token") else {
    return Ok(revoked());
  };
  if token.len() > 200 || !(token.starts_with("oauth_access_") || token.starts_with("oauth_refresh_")) {
    return Ok(revoked());
  }

  let mut connection = server_state.mysql_pool.acquire().await.map_err(|err| {
    warn!("MySQL pool error");
    CommonWebError::from_error(err)
  })?;
  let token_hash = token_hash(token).map_err(CommonWebError::from_anyhow_error)?;
  revoke_oauth_token_hash(RevokePresentedArgs { token_hash: &token_hash, mysql_executor: &mut *connection }).await.map_err(|err| {
    warn!("OAuth revoke failed");
    CommonWebError::from_error(err)
  })?;
  Ok(revoked())
}

/// RFC 7009 success body. Unknown tokens get the same response, and it is not cached.
fn revoked() -> HttpResponse {
  HttpResponse::Ok().insert_header(("cache-control", "no-store")).json(json!({}))
}
