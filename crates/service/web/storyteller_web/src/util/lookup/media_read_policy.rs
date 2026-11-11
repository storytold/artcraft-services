use enums::common::visibility::Visibility;

use crate::http_server::common_responses::common_web_error::CommonWebError;

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
