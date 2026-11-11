use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use actix_web::web::Json;
use actix_web::{web, HttpRequest};
use actix_web_lab::extract::Query;
use artcraft_api_defs::prompts::batch_get_prompts::{
  BatchGetPromptsQuery, BatchGetPromptsResponse, BatchPromptInfo,
};
use artcraft_api_defs::prompts::get_prompt::GetPromptImageContextItem;
use bucket_paths::legacy::typified_paths::public::media_files::bucket_file_path::MediaFileBucketPath;
use enums::by_table::prompt_context_items::prompt_context_semantic_type::PromptContextSemanticType;
use enums::common::visibility::Visibility;
use log::warn;
use mysql_queries::queries::prompt_context_items::batch_list_prompt_context_items::batch_list_prompt_context_items;
use mysql_queries::queries::prompts::batch_get_prompts::batch_get_prompts;
use tokens::tokens::prompts::PromptToken;
use tokens::tokens::users::UserToken;

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::common_responses::media::media_links_builder::MediaLinksBuilder;
use crate::http_server::common_responses::media::media_links_builder::VideoThumbnailInfo;
use crate::http_server::endpoints::media_files::helpers::get_media_domain::get_media_domain;
use crate::state::server_state::ServerState;

const MAX_BATCH_SIZE: usize = 100;

/// Batch get details on multiple prompts.
///
/// Called, eg. {{api_host}}/v1/prompt/batch?tokens={val}&tokens={val}&tokens=...
#[utoipa::path(
  get,
  tag = "Prompts",
  path = "/v1/prompt/batch",
  params(BatchGetPromptsQuery),
  responses(
    (status = 200, description = "Found", body = BatchGetPromptsResponse),
    (status = 400, description = "Bad request"),
    (status = 500, description = "Server error"),
  ),
)]
pub async fn batch_get_prompts_handler(
  http_request: HttpRequest,
  query: Query<BatchGetPromptsQuery>,
  server_state: web::Data<Arc<ServerState>>,
) -> Result<Json<BatchGetPromptsResponse>, CommonWebError> {
  if query.tokens.len() > MAX_BATCH_SIZE {
    return Err(CommonWebError::BadInputWithSimpleMessage(
      format!("tokens must contain at most {} items", MAX_BATCH_SIZE),
    ));
  }

  // Deduplicate and trim whitespace
  let unique_tokens: Vec<PromptToken> = query.tokens
    .iter()
    .map(|t| PromptToken::new_from_str(t.trim()))
    .filter(|t| !t.as_str().is_empty())
    .collect::<HashSet<_>>()
    .into_iter()
    .collect();

  if unique_tokens.is_empty() {
    return Ok(Json(BatchGetPromptsResponse {
      success: true,
      prompts: Vec::new(),
    }));
  }

  let mut mysql_connection = server_state.mysql_pool.acquire().await?;

  let maybe_user_session = server_state
    .session_checker
    .maybe_get_user_session_from_connection(&http_request, &mut mysql_connection)
    .await
    .map_err(|e| {
      warn!("Session checker error: {:?}", e);
      CommonWebError::from(e)
    })?;

  // NB: Same moderator convention as `get_prompt_handler`.
  let is_moderator = maybe_user_session
    .as_ref()
    .map(|session| session.can_ban_users)
    .unwrap_or(false);

  let maybe_viewer_user_token = maybe_user_session.as_ref().map(|session| &session.user_token);

  // Batch query: fetch all prompts in one query
  let results = batch_get_prompts(&unique_tokens, &mut mysql_connection)
    .await
    .map_err(|err| {
      warn!("Batch get prompts query error: {:?}", err);
      CommonWebError::from_error(err)
    })?;

  // Batch query: fetch all context items for all prompts in one query
  let all_context_items = batch_list_prompt_context_items(&unique_tokens, &mut mysql_connection)
    .await
    .map_err(|err| {
      warn!("Batch list prompt context items query error: {:?}", err);
      CommonWebError::from_error(err)
    })?;

  let media_domain = get_media_domain(&http_request);

  // Group context items by prompt token
  let mut context_items_map: HashMap<String, Vec<GetPromptImageContextItem>> = HashMap::new();

  for item in all_context_items {
    // Filter to only image-like context types
    match item.context_semantic_type {
      PromptContextSemanticType::VidStartFrame
      | PromptContextSemanticType::VidEndFrame
      | PromptContextSemanticType::Imgref
      | PromptContextSemanticType::ImgrefCharacter
      | PromptContextSemanticType::ImgrefStyle
      | PromptContextSemanticType::ImgrefBg
      | PromptContextSemanticType::VidRef => {} // Include
      _ => continue, // Ignore
    }

    // NB: A prompt records the media it was built from, but that media keeps its own visibility:
    // a private reference image must not be reachable through a prompt that used it.
    if !may_read_context_media(
      item.creator_set_visibility,
      item.maybe_creator_user_token.as_ref(),
      maybe_viewer_user_token,
      is_moderator,
    ) {
      continue;
    }

    let bucket_path = MediaFileBucketPath::from_object_hash(
      &item.public_bucket_directory_hash,
      item.maybe_public_bucket_prefix.as_deref(),
      item.maybe_public_bucket_extension.as_deref(),
    );

    let context_item = GetPromptImageContextItem {
      media_token: item.media_token,
      semantic: item.context_semantic_type,
      media_links: MediaLinksBuilder::from_media_path_and_env(
        media_domain,
        server_state.server_environment,
        &bucket_path,
        VideoThumbnailInfo::new(item.maybe_thumbnail_version, item.created_at),
      ),
    };

    context_items_map
      .entry(item.prompt_token.as_str().to_string())
      .or_default()
      .push(context_item);
  }

  // Assemble final response
  let prompts: Vec<BatchPromptInfo> = results
    .into_iter()
    .map(|result| {
      let maybe_context_images = context_items_map
        .remove(result.token.as_str())
        .filter(|items| !items.is_empty());

      BatchPromptInfo {
        token: result.token,
        maybe_model_class: result.maybe_model_type.map(|ty| ty.get_model_class()),
        maybe_model_type: result.maybe_model_type,
        maybe_generation_provider: result.maybe_generation_provider,
        maybe_positive_prompt: result.maybe_positive_prompt,
        maybe_negative_prompt: result.maybe_negative_prompt,
        maybe_generation_mode: result.maybe_generation_mode,
        maybe_aspect_ratio: result.maybe_aspect_ratio,
        maybe_resolution: result.maybe_resolution,
        maybe_bitrate: result.maybe_bitrate,
        maybe_batch_count: result.maybe_batch_count,
        maybe_generate_audio: result.maybe_generate_audio,
        maybe_duration_seconds: result.maybe_duration_seconds,
        maybe_context_images,
        created_at: result.created_at,
      }
    })
    .collect();

  Ok(Json(BatchGetPromptsResponse {
    success: true,
    prompts,
  }))
}

/// Private context media is readable only by its creator and by moderators.
fn may_read_context_media(
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
  fn private_media_is_only_readable_by_creator_or_moderator() {
    let creator = UserToken::new_from_str("U:creator");
    let other = UserToken::new_from_str("U:other");

    assert!(may_read_context_media(Visibility::Public, Some(&creator), None, false));
    assert!(may_read_context_media(Visibility::Private, Some(&creator), Some(&creator), false));
    assert!(may_read_context_media(Visibility::Private, Some(&creator), Some(&other), true));
    assert!(!may_read_context_media(Visibility::Private, Some(&creator), Some(&other), false));
    assert!(!may_read_context_media(Visibility::Private, Some(&creator), None, false));
    // No creator on record and no viewer must not count as a match.
    assert!(!may_read_context_media(Visibility::Private, None, None, false));
  }
}
