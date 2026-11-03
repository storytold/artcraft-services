//! Cover-image fallback for mesh/splat results whose webhook payload carries
//! no thumbnail or preview image (e.g. Hunyuan 3D 2.0 / 2.1): attach the
//! original image the user generated the model from (the prompt's imageref).
//!
//! A cover is shown wherever the result is shown, so a private imageref is
//! only attached to a result that is itself private and has the same creator.

use crate::state::server_state::ServerState;
use enums::common::visibility::Visibility;
use log::{info, warn};
use mysql_queries::queries::generic_inference::api_providers::fal::get_inference_job_by_fal_id::FalJobDetails;
use mysql_queries::queries::media_files::edit::set_media_file_cover_image::{set_media_file_cover_image, UpdateArgs};
use mysql_queries::queries::media_files::get::get_media_file::get_media_file;
use mysql_queries::queries::prompt_context_items::get_first_imageref_for_prompt::{get_first_imageref_for_prompt, GetFirstImagerefForPromptArgs};
use tokens::tokens::media_files::MediaFileToken;
use tokens::tokens::users::UserToken;

/// Attach the prompt's first non-deleted imageref as the cover image of
/// `target_media_token`. The imageref is already a media file in our bucket,
/// so no download/upload is needed — just the cover-image foreign key.
///
/// NB: Fail open at every step — a missing cover is never worth failing the
/// webhook over.
pub(crate) async fn try_to_attach_prompt_imageref_cover(
  job: &FalJobDetails,
  server_state: &ServerState,
  target_media_token: &MediaFileToken,
) {
  let Some(prompt_token) = job.maybe_prompt_token.as_ref() else {
    info!(
      "No prompt token on job {:?}; skipping imageref cover fallback for {}",
      job.job_token, target_media_token,
    );
    return;
  };

  let query_result = get_first_imageref_for_prompt(GetFirstImagerefForPromptArgs {
    prompt_token,
    mysql_executor: &server_state.mysql_pool,
    phantom: Default::default(),
  }).await;

  let maybe_imageref_token = match query_result {
    Ok(maybe_token) => maybe_token,
    Err(err) => {
      warn!(
        "Failed to look up imageref for prompt {} (cover fallback for {}): {:?}",
        prompt_token, target_media_token, err,
      );
      return;
    }
  };

  let Some(imageref_token) = maybe_imageref_token else {
    info!(
      "No imageref on prompt {}; {} gets no cover image",
      prompt_token, target_media_token,
    );
    return;
  };

  if !imageref_may_be_cover(&imageref_token, target_media_token, server_state).await {
    info!(
      "Imageref {} is not visible to everyone who can see {}; skipping cover fallback",
      imageref_token, target_media_token,
    );
    return;
  }

  let update_result = set_media_file_cover_image(UpdateArgs {
    media_file_token: target_media_token,
    maybe_cover_image_media_file_token: Some(&imageref_token),
    mysql_pool: &server_state.mysql_pool,
  }).await;

  if let Err(err) = update_result {
    warn!(
      "Failed to set imageref {} as cover image on {}: {:?}",
      imageref_token, target_media_token, err,
    );
    return;
  }

  info!(
    "Attached prompt imageref {} as cover image of {}",
    imageref_token, target_media_token,
  );
}

/// Look up both files and apply [`cover_visibility_permits`]. Fails closed:
/// any lookup error or missing row means "don't attach".
async fn imageref_may_be_cover(
  imageref_token: &MediaFileToken,
  target_media_token: &MediaFileToken,
  server_state: &ServerState,
) -> bool {
  let imageref = match get_media_file(imageref_token, false, &server_state.mysql_pool).await {
    Ok(Some(file)) => file,
    Ok(None) => return false,
    Err(err) => {
      warn!("Failed to look up imageref {} for cover check: {:?}", imageref_token, err);
      return false;
    }
  };

  if imageref.creator_set_visibility != Visibility::Private {
    return true;
  }

  let target = match get_media_file(target_media_token, false, &server_state.mysql_pool).await {
    Ok(Some(file)) => file,
    Ok(None) => return false,
    Err(err) => {
      warn!("Failed to look up {} for cover check: {:?}", target_media_token, err);
      return false;
    }
  };

  cover_visibility_permits(
    imageref.creator_set_visibility,
    imageref.maybe_creator_user_token.as_ref(),
    target.creator_set_visibility,
    target.maybe_creator_user_token.as_ref(),
  )
}

/// A non-private cover can go anywhere. A private cover may only be
/// attached to a private result owned by the same user, so that it is never
/// shown to someone who couldn't already see the original image.
fn cover_visibility_permits(
  cover_visibility: Visibility,
  maybe_cover_creator: Option<&UserToken>,
  target_visibility: Visibility,
  maybe_target_creator: Option<&UserToken>,
) -> bool {
  if cover_visibility != Visibility::Private {
    return true;
  }
  if target_visibility != Visibility::Private {
    return false;
  }
  match (maybe_cover_creator, maybe_target_creator) {
    (Some(cover_creator), Some(target_creator)) => cover_creator.as_str() == target_creator.as_str(),
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use enums::common::visibility::Visibility;
  use tokens::tokens::users::UserToken;

  use super::cover_visibility_permits;

  #[test]
  fn non_private_cover_is_always_allowed() {
    let a = UserToken::new_from_str("U:A");
    let b = UserToken::new_from_str("U:B");
    for cover in [Visibility::Public, Visibility::Hidden] {
      for target in [Visibility::Public, Visibility::Hidden, Visibility::Private] {
        assert!(cover_visibility_permits(cover, Some(&a), target, Some(&b)));
      }
    }
  }

  #[test]
  fn private_cover_is_not_attached_to_shared_result() {
    let a = UserToken::new_from_str("U:A");
    assert!(!cover_visibility_permits(Visibility::Private, Some(&a), Visibility::Public, Some(&a)));
    assert!(!cover_visibility_permits(Visibility::Private, Some(&a), Visibility::Hidden, Some(&a)));
  }

  #[test]
  fn private_cover_needs_private_result_with_same_creator() {
    let a = UserToken::new_from_str("U:A");
    let b = UserToken::new_from_str("U:B");
    assert!(cover_visibility_permits(Visibility::Private, Some(&a), Visibility::Private, Some(&a)));
    assert!(!cover_visibility_permits(Visibility::Private, Some(&a), Visibility::Private, Some(&b)));
    assert!(!cover_visibility_permits(Visibility::Private, None, Visibility::Private, Some(&a)));
    assert!(!cover_visibility_permits(Visibility::Private, Some(&a), Visibility::Private, None));
  }
}
