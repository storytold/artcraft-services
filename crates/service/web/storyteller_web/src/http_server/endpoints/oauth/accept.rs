use actix_web::HttpRequest;
use log::warn;
use sqlx::MySqlConnection;
use tokens::tokens::users::UserToken;

use mysql_queries::queries::oauth_tokens::get_live_oauth_access::{get_live_oauth_access, GetAccessArgs};
use mysql_queries::queries::users::oauth_owner_is_banned::{oauth_owner_is_banned, OwnerBannedArgs};

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::oauth::crypto::token_hash;
use crate::http_server::endpoints::oauth::scopes::{scope_granted, API_AUDIENCE, Scope};
use crate::http_server::web_utils::get_authorization_header_api_key::get_authorization_header_api_key;

const OAUTH_ACCESS_PREFIX: &str = "oauth_access_";
const MAX_PRESENTED_TOKEN_LEN: usize = 200;

pub struct PresentedOauthAccess {
  pub user_token: UserToken,
  pub scopes: String,
  pub audience: String,
}

/// `Ok(None)` when the header is absent or is not an OAuth credential.
/// An `oauth_` credential that is missing, expired, revoked, or not an access token is 401.
pub async fn presented_oauth_access(http_request: &HttpRequest, mysql_connection: &mut MySqlConnection) -> Result<Option<PresentedOauthAccess>, CommonWebError> {
  let Some(credential) = get_authorization_header_api_key(http_request) else {
    return Ok(None);
  };
  let raw = credential.as_str_be_careful();
  if !raw.starts_with("oauth_") {
    return Ok(None);
  }
  if !raw.starts_with(OAUTH_ACCESS_PREFIX) || raw.len() > MAX_PRESENTED_TOKEN_LEN {
    warn!("Rejected an OAuth Authorization header that is not an access token");
    return Err(CommonWebError::NotAuthorized);
  }

  let access_hash = token_hash(raw).map_err(CommonWebError::from_anyhow_error)?;
  let row = get_live_oauth_access(GetAccessArgs { access_hash: &access_hash, mysql_executor: &mut *mysql_connection }).await.map_err(|err| {
    warn!("OAuth access lookup failed");
    CommonWebError::from_error(err)
  })?;
  let Some(row) = row else {
    warn!("Rejected an OAuth access token");
    return Err(CommonWebError::NotAuthorized);
  };

  let banned = oauth_owner_is_banned(OwnerBannedArgs { user_token: &row.user_token, mysql_executor: &mut *mysql_connection }).await.map_err(|err| {
    warn!("OAuth owner lookup failed");
    CommonWebError::from_error(err)
  })?;
  if banned != Some(false) {
    warn!("Rejected an OAuth token for a missing or banned user");
    return Err(CommonWebError::NotAuthorized);
  }

  Ok(Some(PresentedOauthAccess { user_token: UserToken::new(row.user_token), scopes: row.scopes, audience: row.audience }))
}

/// Library and job reads. A bad OAuth header stays 401 and does not fall through to a cookie.
pub async fn oauth_user_for_scope(http_request: &HttpRequest, required: Scope, mysql_connection: &mut MySqlConnection) -> Result<Option<UserToken>, CommonWebError> {
  let Some(access) = presented_oauth_access(http_request, mysql_connection).await? else {
    return Ok(None);
  };
  if access.audience != API_AUDIENCE || !scope_granted(&access.scopes, required) {
    warn!("Rejected an OAuth token for {}", required.as_str());
    return Err(CommonWebError::NotAuthorized);
  }
  Ok(Some(access.user_token))
}

pub fn oauth_access_satisfies(access: &PresentedOauthAccess, required: Scope) -> bool {
  access.audience == API_AUDIENCE && scope_granted(&access.scopes, required)
}
