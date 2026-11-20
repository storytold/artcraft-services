use actix_web::http::header;
use actix_web::HttpRequest;
use sqlx::{Executor, MySql};

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::user_lookup::api_or_web_session::require_any_session_or_key::require_any_session_or_key;
use crate::state::server_state::ServerState;

/// The user token behind an `Authorization` header credential (API key or MCP session), for
/// public read endpoints that only have the cookie session otherwise. No header, or a credential
/// that does not authenticate, yields `None` (the caller stays an anonymous viewer, never more
/// privileged); only lookup failures are errors.
///
/// Pass the handler's open connection (`&mut *connection`) when it holds one, so the lookup does
/// not take a second connection from the pool; otherwise `&server_state.mysql_pool`.
pub async fn header_viewer_user_token<'c, E>(
  http_request: &HttpRequest,
  server_state: &ServerState,
  mysql_executor: E,
) -> Result<Option<String>, CommonWebError>
  where E: 'c + Executor<'c, Database = MySql>
{
  if !http_request.headers().contains_key(header::AUTHORIZATION) {
    return Ok(None);
  }

  match require_any_session_or_key(
    http_request,
    &server_state.session_checker,
    &server_state.avt_cookie_manager,
    mysql_executor,
  ).await {
    Ok(session) => Ok(Some(session.user_token.as_str().to_string())),
    Err(CommonWebError::NotAuthorized) => Ok(None),
    Err(err) => Err(err),
  }
}
