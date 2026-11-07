use std::sync::Arc;

use actix_web::web::{Json, Path, Query};
use actix_web::{web, HttpRequest};
use artcraft_api_defs::media_file::list::user_media_file_list_item::MediaFileForUserListItem;
use enums::common::view_as::ViewAs;
use log::info;
use std::time::Instant;
use mysql_queries::queries::media_files::list::list_media_files_for_user::{list_media_files_for_user, ListMediaFileForUserArgs};
use mysql_queries::queries::media_files::list::count_media_files_for_user::{count_media_files_for_user, CountMediaFilesForUserArgs};
use mysql_queries::queries::media_files::list::user_media_file_filters::UserMediaFileFilters;
use super::user_media_file_response::{build_user_media_file_response, UserMediaFileResponseArgs};
use utoipa::{IntoParams, ToSchema};
use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::common_responses::pagination_page::PaginationPage;
use crate::http_server::endpoints::media_files::helpers::get_media_domain::get_media_domain;
use crate::http_server::endpoints::media_files::helpers::get_scoped_engine_categories::get_scoped_engine_categories;
use crate::http_server::endpoints::media_files::helpers::get_scoped_media_classes::get_scoped_media_classes;
use crate::http_server::endpoints::media_files::helpers::get_scoped_media_types::get_scoped_media_types;
use crate::state::server_state::ServerState;
use crate::util::allowed_studio_access::allowed_studio_access;

#[derive(Deserialize, ToSchema)]
pub struct ListMediaFilesForUserPathInfo {
  username: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct ListMediaFilesForUserQueryParams {
  pub sort_ascending: Option<bool>,
  pub page_size: Option<usize>,
  pub page_index: Option<usize>,

  /// NB: This can be one (or more comma-separated values) from `MediaFileClass`,
  /// which are the broad category of media files: image, video, etc.
  ///
  /// Usage:
  ///   - `?filter_media_classes=audio`
  ///   - `?filter_media_classes=image,video`
  ///   - `?filter_media_classes=dimensional`
  ///   - etc.
  pub filter_media_classes: Option<String>,

  /// NB: This can be one (or more comma-separated values) from `MediaFileType`,
  /// which are mimetype-like / format-like categories of media files: glb, gltf,
  /// scene_json, jpg, png, mp4, wav, etc.
  ///
  /// Usage:
  ///   - `?filter_media_type=scene_json`
  ///   - `?filter_media_type=glb,gltf,fbx`
  ///   - `?filter_media_type=pmd,vmd,pmx`
  ///   - `?filter_media_type=jpg,png,gif`
  ///   - `?filter_media_type=wav,mp3`
  ///   - `?filter_media_type=mp4`
  ///   - etc.
  pub filter_media_type: Option<String>,

  /// NB: This can be one (or more comma-separated values) from `MediaFileEngineCategory`.
  ///
  /// Usage:
  ///   - `?filter_engine_categories=scene`
  ///   - `?filter_engine_categories=animation,character,object`
  ///   - etc.
  pub filter_engine_categories: Option<String>,

  /// Include user uploaded files in the results.
  /// By default, we do not return them unless this flag is set to true.
  pub include_user_uploads: Option<bool>,
}

#[derive(Serialize, ToSchema)]
pub struct ListMediaFilesForUserSuccessResponse {
  pub success: bool,
  pub results: Vec<MediaFileForUserListItem>,
  pub pagination: PaginationPage,
}

/// List all of a user's media files (paginated).
///
/// This endpoint uses the session to automatically show all files for the given user, but only
/// show "public" files for external users.
#[utoipa::path(
  get,
  tag = "Media Files",
  path = "/v1/media_files/list/user/{username}",
  params(
     ListMediaFilesForUserQueryParams,
  ),
  responses(
    (status = 200, description = "List Featured Media Files", body = ListMediaFilesForUserSuccessResponse),
    (status = 500, description = "Server error"),
  ),
)]
pub async fn list_media_files_for_user_handler(
  http_request: HttpRequest,
  path: Path<ListMediaFilesForUserPathInfo>,
  query: Query<ListMediaFilesForUserQueryParams>,
  server_state: web::Data<Arc<ServerState>>
) -> Result<Json<ListMediaFilesForUserSuccessResponse>, CommonWebError>
{
  let maybe_user_session = server_state
      .session_checker
      .maybe_get_user_session(&http_request, &server_state.mysql_pool)
      .await?;

  let mut is_author = false;
  let mut is_mod = false;

  // NB: Temporary rollout flag for certain file types (BVH, etc).
  let mut is_allowed_studio_access = allowed_studio_access(
    maybe_user_session.as_ref(),
    &server_state.flags
  );

  match maybe_user_session {
    None => {},
    Some(session) => {
      is_author = session.username == path.username;
      is_mod = session.can_ban_users;
    },
  };

  // TODO(bt,2023-12-04): Enforce real maximums and defaults
  let sort_ascending = query.sort_ascending.unwrap_or(false);
  let page_size = query.page_size.unwrap_or_else(|| 25);
  let page_index = query.page_index.unwrap_or_else(|| 0);

  let view_as = if is_author {
    ViewAs::Author
  } else if is_mod {
    ViewAs::Moderator
  } else {
    ViewAs::AnotherUser
  };

  let mut maybe_filter_media_types = get_scoped_media_types(query.filter_media_type.as_deref());
  let mut maybe_filter_media_classes  = get_scoped_media_classes(query.filter_media_classes.as_deref());
  let mut maybe_filter_engine_categories = get_scoped_engine_categories(query.filter_engine_categories.as_deref());

  let filters = UserMediaFileFilters {
    username: &path.username,
    maybe_filter_media_types: maybe_filter_media_types.as_ref(),
    maybe_filter_media_classes: maybe_filter_media_classes.as_ref(),
    maybe_filter_engine_categories: maybe_filter_engine_categories.as_ref(),
    include_user_uploads: query.include_user_uploads.unwrap_or(false),
    view_as,
  };
  let count_started = Instant::now();
  let total_count = count_media_files_for_user(CountMediaFilesForUserArgs {
    filters,
    mysql_executor: &server_state.mysql_pool,
  }).await?;
  let count_ms = count_started.elapsed().as_millis();
  let page_started = Instant::now();
  let records = list_media_files_for_user(ListMediaFileForUserArgs {
    filters,
    limit: page_size,
    offset: page_index * page_size,
    sort_ascending,
    mysql_executor: &server_state.mysql_pool,
  }).await?;
  info!("Listed legacy user media: username={:?} page={} count_ms={} page_ms={}",
    path.username, page_index, count_ms, page_started.elapsed().as_millis());

  let media_domain = get_media_domain(&http_request);

  let results = build_user_media_file_response(UserMediaFileResponseArgs {
    records,
    is_allowed_studio_access,
    media_domain,
    server_environment: server_state.server_environment,
  });

  Ok(Json(ListMediaFilesForUserSuccessResponse {
    success: true,
    results,
    pagination: PaginationPage {
      current: page_index,
      // Preserve v1's historical calculation, including exact multiples.
      // A pagination behavior change belongs in a new endpoint version.
      total_page_count: 1 + (total_count / page_size as i64) as usize,
    }
  }))
}
