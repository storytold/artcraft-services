use enums::common::visibility::Visibility;
use mysql_queries::queries::media_files::get::batch_get_media_files_by_tokens::MediaReadScope;
use mysql_queries::queries::users::user_sessions::get_user_session_by_token::SessionUserRecord;
use tokens::tokens::users::UserToken;

use crate::http_server::common_responses::common_web_error::CommonWebError;

/// The read scope of a caller authenticated without a cookie session (API key, SSO), for media
/// lookups. Those callers have no moderator ability, so private records stay restricted to their
/// creators.
pub fn scope_for_user_token(user_token: &UserToken, is_mod: bool) -> MediaReadScope<'_> {
  MediaReadScope { viewer_user_token: Some(user_token.as_str()), is_mod }
}

/// The read scope of the requester behind `maybe_user_session`, for media lookups.
pub fn scope_for_session(maybe_user_session: Option<&SessionUserRecord>) -> MediaReadScope<'_> {
  match maybe_user_session {
    Some(session) => MediaReadScope {
      viewer_user_token: Some(session.user_token.as_str()),
      is_mod: session.can_delete_other_users_tts_results,
    },
    None => MediaReadScope::ANONYMOUS,
  }
}

/// Private media is readable only by its creator and by moderators. `NotFound` is returned rather
/// than `NotAuthorized` so that a non-owner cannot confirm that a private token exists.
///
/// Callers that return many records should drop the ones this rejects instead of failing the whole
/// request, which also keeps a mixed batch from disclosing which tokens were unreadable.
pub fn ensure_may_read_media(
  visibility: &Visibility,
  maybe_creator_user_token: Option<&str>,
  viewer_user_token: Option<&str>,
  is_mod: bool,
) -> Result<(), CommonWebError> {
  if *visibility != Visibility::Private || is_mod {
    return Ok(());
  }

  match (viewer_user_token, maybe_creator_user_token) {
    (Some(viewer), Some(creator)) if viewer == creator => Ok(()),
    _ => Err(CommonWebError::NotFound),
  }
}
