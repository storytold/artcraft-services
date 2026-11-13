use std::sync::Arc;

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::common_responses::media::media_file_cover_image_details_builder::MediaFileCoverImageDetailsBuilder;
use crate::http_server::common_responses::media::media_links_builder::MediaLinksBuilder;
use crate::http_server::common_responses::media::media_links_builder::VideoThumbnailInfo;
use crate::http_server::common_responses::user_details_lite_builder::UserDetailsLightBuilder;
use crate::http_server::endpoints::media_files::get::get_media_file_handler::GetMediaFilePathInfo;
use crate::http_server::endpoints::media_files::helpers::get_media_domain::get_media_domain;
use crate::http_server::user_lookup::api_or_web_session::header_viewer_user_token::header_viewer_user_token;
use crate::state::server_state::ServerState;
use actix_web::web::{Json, Path};
use actix_web::{web, HttpRequest};
use actix_web_lab::extract::Query;
use artcraft_api_defs::common::responses::media_links::MediaLinks;
use artcraft_api_defs::common::responses::simple_entity_stats::SimpleEntityStats;
use artcraft_api_defs::media_file::list_batch_generated_media_files::{BatchGeneratedReduxMediaFileInfo, ListBatchGeneratedReduxMediaFilesPathInfo, ListBatchGeneratedReduxMediaFilesSuccessResponse};
use bucket_paths::legacy::typified_paths::public::media_files::bucket_file_path::MediaFileBucketPath;
use chrono::{DateTime, Utc};
use enums::common::visibility::Visibility;
use log::warn;
use mysql_queries::queries::media_files::get::batch_get_media_files::batch_get_media_files;
use mysql_queries::queries::media_files::list::list_batch_generated_redux_media_files::list_batch_generated_redux_media_files_with_connection;
use tokens::tokens::batch_generations::BatchGenerationToken;
use tokens::tokens::media_files::MediaFileToken;
use tokens::tokens::model_weights::ModelWeightToken;
use tokens::tokens::prompts::PromptToken;
use utoipa::{IntoParams, ToSchema};

/// List media files generated as part of the same batch (redux endpoint)
#[utoipa::path(
  get,
  tag = "Media Files",
  path = "/v1/media_files/batch_gen_redux/{token}",
  responses(
    (status = 200, description = "Success", body = ListBatchGeneratedReduxMediaFilesSuccessResponse),
  ),
  params(
    ("request" = ListBatchGeneratedReduxMediaFilesPathInfo, description = "Payload for Request"),
  )
)]
pub async fn list_batch_generated_redux_media_files_handler(
  http_request: HttpRequest,
  path: Path<ListBatchGeneratedReduxMediaFilesPathInfo>,
  server_state: web::Data<Arc<ServerState>>
) -> Result<Json<ListBatchGeneratedReduxMediaFilesSuccessResponse>, CommonWebError> {
  let mut mysql_connection = server_state.mysql_pool
      .acquire()
      .await?;

  let maybe_user_session = server_state
      .session_checker
      .maybe_get_user_session_from_connection(&http_request, &mut mysql_connection)
      .await?;

  // NB: Moderators can see deleted results.
  // Original creators cannot see them (unless they're moderators!)
  let is_mod = maybe_user_session
      .as_ref()
      .map(|session| session.can_delete_other_users_tts_results)
      .unwrap_or(false);

  let show_deleted_results = is_mod;

  let viewer_user_token = match maybe_user_session.as_ref() {
    Some(session) => Some(session.user_token.as_str().to_string()),
    // API-key and MCP clients authenticate by header, not cookie.
    None => header_viewer_user_token(&http_request, &server_state).await?,
  };

  // NB(bt,2024-03-24): I'm sorry, this is gross. We're not respecting sorting, input ordering,
  // de-duplication, (if we swap types), etc. Gotta go fast.
  let result = list_batch_generated_redux_media_files_with_connection(
    &path.token,
    show_deleted_results,
    &mut mysql_connection,
  ).await;

  let media_files = match result {
    Ok(batch) => batch.media_files,
    Err(e) => {
      warn!("query error: {:?}", e);
      return Err(CommonWebError::from_anyhow_error(e));
    }
  };

  let media_domain = get_media_domain(&http_request);

  // NB: A batch token is not a read grant: public files share it with private ones, so
  // private files are listed only to their creator and to moderators (as for `/batch/{token}`).
  let media_files = media_files.into_iter()
      .filter(|result| {
        if result.creator_set_visibility != Visibility::Private || is_mod {
          return true;
        }
        match (viewer_user_token.as_deref(), result.maybe_creator_user_token.as_ref()) {
          (Some(viewer), Some(creator)) => viewer == creator.as_str(),
          _ => false,
        }
      })
      .map(|result| {
        let public_bucket_path = MediaFileBucketPath::from_object_hash(
          &result.public_bucket_directory_hash,
          result.maybe_public_bucket_prefix.as_deref(),
          result.maybe_public_bucket_extension.as_deref());

        BatchGeneratedReduxMediaFileInfo {
          token: result.token.clone(),
          media_class: result.media_class,
          media_type: result.media_type,
          maybe_batch_token: result.maybe_batch_token,
          media_links: MediaLinksBuilder::from_media_path_and_env(
            media_domain,
            server_state.server_environment,
            &public_bucket_path,
            VideoThumbnailInfo::new(result.maybe_thumbnail_version, result.created_at)
          ),
          cover_image: MediaFileCoverImageDetailsBuilder::from_optional_db_fields(
            &result.token,
            media_domain,
            server_state.server_environment,
            result.maybe_file_cover_image_public_bucket_hash.as_deref(),
            result.maybe_file_cover_image_public_bucket_prefix.as_deref(),
            result.maybe_file_cover_image_public_bucket_extension.as_deref(),
          ),
          maybe_title: result.maybe_title,
          maybe_creator_user: UserDetailsLightBuilder::from_optional_db_fields_owned(
            result.maybe_creator_user_token,
            result.maybe_creator_username,
            result.maybe_creator_display_name,
            result.maybe_creator_gravatar_hash,
          ),
          creator_set_visibility: result.creator_set_visibility,
          maybe_prompt_token: result.maybe_prompt_token,
          maybe_original_filename: result.maybe_origin_filename,
          maybe_duration_millis: result.maybe_duration_millis,
          created_at: result.created_at,
          updated_at: result.updated_at,
        }
      })
      .collect();

  Ok(Json(ListBatchGeneratedReduxMediaFilesSuccessResponse {
    success: true,
    media_files,
  }))
}
