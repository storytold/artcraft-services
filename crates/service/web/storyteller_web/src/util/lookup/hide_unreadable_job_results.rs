use std::collections::HashMap;

use enums::common::visibility::Visibility;
use errors::AnyhowResult;
use mysql_queries::queries::generic_inference::web::job_status::GenericInferenceJobStatus;
use log::warn;
use mysql_queries::queries::media_files::get::batch_get_media_files_by_tokens::{batch_get_media_files_by_tokens, MediaReadScope};
use sqlx::MySqlPool;
use tokens::tokens::media_files::MediaFileToken;
use tokens::tokens::users::UserToken;

/// Job status is public to anyone holding the job token, but a media result keeps its own
/// visibility. Drop `maybe_result_details` for media results the viewer may not read right now:
/// deleted (unless moderator), or private and not the viewer's own.
pub async fn hide_unreadable_job_results(
  records: &mut [GenericInferenceJobStatus],
  maybe_viewer_user_token: Option<&UserToken>,
  is_moderator: bool,
  mysql_pool: &MySqlPool,
) -> AnyhowResult<()> {
  let media_tokens = records.iter()
      .filter_map(|record| record.maybe_result_details.as_ref())
      .filter(|details| details.entity_type == "media_file")
      .map(|details| MediaFileToken::new_from_str(&details.entity_token))
      .collect::<Vec<_>>();

  if media_tokens.is_empty() {
    return Ok(());
  }

  // NB: Soft-deleted media is not returned unless the viewer is a moderator.
  let scope = MediaReadScope {
    viewer_user_token: maybe_viewer_user_token.map(|token| token.as_str()),
    is_mod: is_moderator,
  };
  let media = match batch_get_media_files_by_tokens(mysql_pool, &media_tokens, is_moderator, &scope).await {
    Ok(media) => media,
    Err(err) => {
      // Fail closed: the status stays, the results go.
      warn!("Error looking up media for job results: {:?}", err);
      for record in records.iter_mut() {
        record.maybe_result_details = None;
      }
      return Ok(());
    }
  };
  let media_by_token = media
      .into_iter()
      .map(|media| (media.token.as_str().to_string(), (media.creator_set_visibility, media.maybe_creator_user_token)))
      .collect::<HashMap<_, _>>();

  for record in records.iter_mut() {
    let readable = match record.maybe_result_details.as_ref() {
      Some(details) if details.entity_type == "media_file" => match media_by_token.get(&details.entity_token) {
        None => false,
        Some((visibility, maybe_creator)) =>
          is_result_readable(*visibility, maybe_creator.as_ref(), maybe_viewer_user_token, is_moderator),
      },
      _ => true,
    };
    if !readable {
      record.maybe_result_details = None;
    }
  }

  Ok(())
}

fn is_result_readable(
  visibility: Visibility,
  maybe_creator_user_token: Option<&UserToken>,
  maybe_viewer_user_token: Option<&UserToken>,
  is_moderator: bool,
) -> bool {
  if visibility != Visibility::Private || is_moderator {
    return true;
  }
  matches!((maybe_creator_user_token, maybe_viewer_user_token), (Some(creator), Some(viewer)) if creator == viewer)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn private_results_are_only_readable_by_creator_or_moderator() {
    let creator = UserToken::new_from_str("U:creator");
    let other = UserToken::new_from_str("U:other");

    assert!(is_result_readable(Visibility::Public, Some(&creator), None, false));
    assert!(is_result_readable(Visibility::Hidden, Some(&creator), None, false));
    assert!(is_result_readable(Visibility::Private, Some(&creator), Some(&creator), false));
    assert!(is_result_readable(Visibility::Private, Some(&creator), None, true));
    assert!(!is_result_readable(Visibility::Private, Some(&creator), Some(&other), false));
    assert!(!is_result_readable(Visibility::Private, Some(&creator), None, false));
    assert!(!is_result_readable(Visibility::Private, None, None, false));
  }
}
