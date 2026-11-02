use std::sync::Arc;

use actix_web::web::{Data, Json};
use actix_web::HttpRequest;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tokens::tokens::oauth::OauthClientToken;

use http_server_common::request::get_request_ip::get_request_ip;
use mysql_queries::queries::oauth::{insert_oauth_client, insert_oauth_redirect, InsertOauthClientArgs, InsertOauthRedirectArgs};

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::oauth::crypto::token_hash;
use crate::http_server::endpoints::oauth::redirect::redirect_uri_is_allowed;
use crate::http_server::user_lookup::user_session::require_user_session::require_user_session;
use crate::state::server_state::ServerState;

const MAX_NAME_LEN: usize = 80;
const MAX_REDIRECTS: usize = 8;

#[derive(Deserialize)]
pub struct RegisterOauthClientRequest {
  client_name: String,
  redirect_uris: Vec<String>,
}

#[derive(Serialize)]
pub struct RegisterOauthClientResponse {
  client_id: String,
  client_name: String,
  redirect_uris: Vec<String>,
}

/// Register a public client for the signed-in user. There is no client secret; PKCE is required.
pub async fn oauth_register_client_handler(http_request: HttpRequest, request: Json<RegisterOauthClientRequest>, server_state: Data<Arc<ServerState>>) -> Result<Json<RegisterOauthClientResponse>, CommonWebError> {
  let client_name = request.client_name.trim();
  if client_name.is_empty() || client_name.len() > MAX_NAME_LEN || client_name.chars().any(|character| character.is_control()) {
    return Err(CommonWebError::BadInputWithSimpleMessage("client_name is not valid".to_string()));
  }
  if request.redirect_uris.is_empty() || request.redirect_uris.len() > MAX_REDIRECTS {
    return Err(CommonWebError::BadInputWithSimpleMessage("redirect_uris must contain 1 to 8 https or loopback urls".to_string()));
  }
  let mut redirect_uris = Vec::new();
  for redirect_uri in &request.redirect_uris {
    if !redirect_uri_is_allowed(redirect_uri) || redirect_uris.iter().any(|existing| existing == redirect_uri) {
      return Err(CommonWebError::BadInputWithSimpleMessage("redirect_uri is not allowed".to_string()));
    }
    redirect_uris.push(redirect_uri.clone());
  }

  let mut tx = server_state.mysql_pool.begin().await.map_err(|err| {
    warn!("MySQL pool error");
    CommonWebError::from_error(err)
  })?;
  let user_session = require_user_session(&http_request, &server_state.session_checker, &mut *tx).await?;
  let client = OauthClientToken::generate();
  insert_oauth_client(InsertOauthClientArgs { token: client.as_str(), client_name, owner_user_token: user_session.get_user_token().as_str(), mysql_executor: &mut *tx }).await.map_err(|err| {
    warn!("OAuth client insert failed");
    CommonWebError::from_error(err)
  })?;
  for redirect_uri in &redirect_uris {
    let redirect_hash = token_hash(redirect_uri).map_err(CommonWebError::from_anyhow_error)?;
    insert_oauth_redirect(InsertOauthRedirectArgs { client_token: client.as_str(), redirect_uri, redirect_hash: &redirect_hash, mysql_executor: &mut *tx }).await.map_err(|err| {
      warn!("OAuth redirect insert failed");
      CommonWebError::from_error(err)
    })?;
  }
  tx.commit().await.map_err(CommonWebError::from_error)?;
  info!("OAuth client registered by {} from {}", user_session.get_user_token().as_str(), get_request_ip(&http_request),);

  Ok(Json(RegisterOauthClientResponse { client_id: client.as_str().to_string(), client_name: client_name.to_string(), redirect_uris }))
}
