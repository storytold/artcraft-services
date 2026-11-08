//! Decides whether a caller may feed a stored file into an inference job.
//!
//! The legacy job workers download whatever token the job names with the service's own bucket
//! credentials and publish the result under the requester. Possession of a token is therefore not
//! enough: a private file may only be used by its creator (or a moderator), and deleted files may
//! not be used at all. Enqueue handlers check every input token here before inserting the job.

use log::warn;
use sqlx::MySqlPool;

use enums::common::visibility::Visibility;
use mysql_queries::queries::media_files::get::get_media_file::get_media_file;
use mysql_queries::queries::media_uploads::get_media_upload_access_info::{get_media_upload_access_info, GetMediaUploadAccessInfoArgs};
use mysql_queries::queries::voice_conversion::results::get_voice_conversion_result::get_voice_conversion_result;
use tokens::tokens::media_files::MediaFileToken;
use tokens::tokens::media_uploads::MediaUploadToken;
use tokens::tokens::users::UserToken;
use tokens::tokens::voice_conversion_results::VoiceConversionResultToken;

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::user_lookup::user_session::session_utils::lookup::user_session_extended::UserSessionExtended;

/// Deleted inputs are never usable, so lookups exclude them.
const CAN_SEE_DELETED: bool = false;

/// Who is submitting the job.
#[derive(Clone, Copy)]
pub struct JobInputRequester<'a> {
  pub maybe_user_token: Option<&'a str>,
  pub is_mod: bool,
}

impl<'a> JobInputRequester<'a> {
  pub fn from_session(maybe_user_session: Option<&'a UserSessionExtended>) -> Self {
    Self {
      maybe_user_token: maybe_user_session.map(|session| session.user_token.as_str()),
      is_mod: maybe_user_session.is_some_and(|session| session.role.can_ban_users),
    }
  }
}

/// True when `requester` may use the media file as a job input. Missing, deleted, and other users'
/// private files all return false, so callers should answer "not found" in every case.
pub async fn can_use_media_file_input(
  token: &MediaFileToken,
  requester: JobInputRequester<'_>,
  mysql_pool: &MySqlPool,
) -> anyhow::Result<bool> {
  let maybe_record = get_media_file(token, CAN_SEE_DELETED, mysql_pool).await?;
  Ok(maybe_record.is_some_and(|record| may_use_record(
    record.creator_set_visibility,
    record.maybe_creator_user_token.as_ref(),
    requester,
  )))
}

/// Require every non-empty media file token to be usable by `requester`, answering `NotFound`
/// otherwise so that other users' private files are indistinguishable from missing ones.
pub async fn require_media_file_inputs(
  tokens: &[Option<&MediaFileToken>],
  requester: JobInputRequester<'_>,
  mysql_pool: &MySqlPool,
) -> Result<(), CommonWebError> {
  for token in tokens.iter().flatten() {
    if token.as_str().trim().is_empty() {
      continue;
    }
    let can_use_input = can_use_media_file_input(token, requester, mysql_pool)
        .await
        .map_err(|err| {
          warn!("Error checking job input access for {:?}: {:?}", token, err);
          CommonWebError::from_anyhow_error(err)
        })?;
    if !can_use_input {
      warn!("Job input not usable by requester: {:?}", token);
      return Err(CommonWebError::NotFound);
    }
  }
  Ok(())
}

/// Same as `can_use_media_file_input`, for legacy media uploads.
pub async fn can_use_media_upload_input(
  token: &MediaUploadToken,
  requester: JobInputRequester<'_>,
  mysql_pool: &MySqlPool,
) -> anyhow::Result<bool> {
  let maybe_record = get_media_upload_access_info(GetMediaUploadAccessInfoArgs { media_upload_token: token, mysql_executor: mysql_pool }).await?;
  Ok(maybe_record.is_some_and(|record| may_use_record(
    record.creator_set_visibility,
    record.maybe_creator_user_token.as_ref(),
    requester,
  )))
}

/// Same as `can_use_media_file_input`, for legacy voice conversion results.
pub async fn can_use_voice_conversion_result_input(
  token: &VoiceConversionResultToken,
  requester: JobInputRequester<'_>,
  mysql_pool: &MySqlPool,
) -> anyhow::Result<bool> {
  let maybe_record = get_voice_conversion_result(token.as_str(), CAN_SEE_DELETED, mysql_pool).await?;
  Ok(maybe_record.is_some_and(|record| may_use_record(
    record.creator_set_visibility,
    record.maybe_creator_user_token.as_ref(),
    requester,
  )))
}

/// The visibility rule for a record that exists and is not deleted.
///
/// A private record belongs to its creator. Records with no creator user are system
/// intermediates or anonymous uploads, which the batch media read policy also allows.
fn may_use_record(
  visibility: Visibility,
  maybe_creator_user_token: Option<&UserToken>,
  requester: JobInputRequester<'_>,
) -> bool {
  if visibility != Visibility::Private || requester.is_mod {
    return true;
  }
  match maybe_creator_user_token {
    None => true,
    Some(creator) => requester.maybe_user_token == Some(creator.as_str()),
  }
}

#[cfg(test)]
mod tests {
  use enums::common::visibility::Visibility;
  use tokens::tokens::users::UserToken;

  use super::{may_use_record, JobInputRequester};

  const OWNER: JobInputRequester<'static> = JobInputRequester { maybe_user_token: Some("U:OWNER"), is_mod: false };
  const OTHER: JobInputRequester<'static> = JobInputRequester { maybe_user_token: Some("U:OTHER"), is_mod: false };
  const ANON: JobInputRequester<'static> = JobInputRequester { maybe_user_token: None, is_mod: false };
  const MOD: JobInputRequester<'static> = JobInputRequester { maybe_user_token: Some("U:MOD"), is_mod: true };

  #[test]
  fn non_private_records_are_usable_by_anyone() {
    let owner = UserToken::new_from_str("U:OWNER");
    for visibility in [Visibility::Public, Visibility::Hidden] {
      for requester in [OWNER, OTHER, ANON, MOD] {
        assert!(may_use_record(visibility, Some(&owner), requester));
      }
    }
  }

  #[test]
  fn private_records_are_usable_by_creator_and_mods_only() {
    let owner = UserToken::new_from_str("U:OWNER");
    assert!(may_use_record(Visibility::Private, Some(&owner), OWNER));
    assert!(may_use_record(Visibility::Private, Some(&owner), MOD));
    assert!(!may_use_record(Visibility::Private, Some(&owner), OTHER));
    assert!(!may_use_record(Visibility::Private, Some(&owner), ANON));
  }

  #[test]
  fn private_records_without_a_creator_follow_the_read_policy() {
    assert!(may_use_record(Visibility::Private, None, OTHER));
    assert!(may_use_record(Visibility::Private, None, ANON));
  }
}
