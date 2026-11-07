use std::sync::Arc;
use std::time::Instant;

use actix_web::web::{Json, Path, Query};
use actix_web::{web, HttpRequest};
use artcraft_api_defs::media_file::list::list_media_files_for_user_v2::{
  ListMediaFilesForUserV2PathInfo, ListMediaFilesForUserV2QueryParams,
  ListMediaFilesForUserV2SuccessResponse, UserMediaFileV2Pagination,
};
use enums::common::view_as::ViewAs;
use log::{info, warn};
use mysql_queries::queries::users::user::get::get_user_token_by_username::get_user_token_by_username;
use mysql_queries::queries::media_files::list::list_media_files_for_user::{
  list_media_files_for_user, ListMediaFileForUserArgs,
};
use mysql_queries::queries::media_files::list::user_media_file_filters::UserMediaFileFilters;

use super::user_media_file_response::{build_user_media_file_response, UserMediaFileResponseArgs};
use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::media_files::helpers::get_media_domain::get_media_domain;
use crate::http_server::endpoints::media_files::helpers::get_scoped_engine_categories::get_scoped_engine_categories;
use crate::http_server::endpoints::media_files::helpers::get_scoped_media_classes::get_scoped_media_classes;
use crate::http_server::endpoints::media_files::helpers::get_scoped_media_types::get_scoped_media_types;
use crate::state::server_state::ServerState;
use crate::util::allowed_studio_access::allowed_studio_access;

/// Faster user-media listing for infinite scroll.
///
/// Unlike `/v1/media_files/list/user/{username}`, this endpoint skips the exact
/// `COUNT(*)` over the user's entire matching library. It fetches `page_size + 1`
/// records to return `pagination.has_more`, without a total-page count. Removing
/// that scan makes loading pages faster for large libraries.
///
/// Both endpoints use the same optimized per-user indexes and load related
/// metadata only after selecting a page. The original endpoint retains its
/// request/response contract and pagination behavior for deployed clients.
///
/// Page indexes remain offset based; very deep pages still scan preceding
/// matching records. Page size defaults to 25 and must be between 1 and 100.
#[utoipa::path(
  get,
  tag = "Media Files",
  path = "/v1/media_files/list_v2/user/{username}",
  params(ListMediaFilesForUserV2QueryParams),
  responses(
    (status = 200, description = "Faster user-media page with has_more; no full-library count", body = ListMediaFilesForUserV2SuccessResponse),
    (status = 400, description = "Invalid pagination"),
    (status = 500, description = "Server error"),
  ),
)]
pub async fn list_media_files_for_user_v2_handler(
  http_request: HttpRequest,
  path: Path<ListMediaFilesForUserV2PathInfo>,
  query: Query<ListMediaFilesForUserV2QueryParams>,
  server_state: web::Data<Arc<ServerState>>,
) -> Result<Json<ListMediaFilesForUserV2SuccessResponse>, CommonWebError> {
  let maybe_user_session = server_state
    .session_checker
    .maybe_get_user_session(&http_request, &server_state.mysql_pool)
    .await?;

  let mut is_author = false;
  let mut is_mod = false;

  // NB: Temporary rollout flag for certain file types (BVH, etc).
  let is_allowed_studio_access = allowed_studio_access(maybe_user_session.as_ref(), &server_state.flags);

  // NB: Usernames can be released and reassigned while a cached session still carries the old
  // name, so author access is bound to the immutable user token of the account being listed.
  let maybe_target_user_token = get_user_token_by_username(&path.username, &server_state.mysql_pool)
      .await
      .map_err(|err| {
        warn!("Error resolving username {:?}: {:?}", path.username, err);
        CommonWebError::from_anyhow_error(err)
      })?;

  match maybe_user_session {
    None => {}
    Some(session) => {
      is_author = maybe_target_user_token.as_ref() == Some(&session.user_token);
      is_mod = session.can_ban_users;
    }
  };

  let sort_ascending = query.sort_ascending.unwrap_or(false);
  let page_size = query.page_size.unwrap_or_else(|| 25);
  let page_index = query.page_index.unwrap_or_else(|| 0);
  if !(1..=100).contains(&page_size) {
    return Err(CommonWebError::BadInputWithSimpleMessage(
      "page_size must be between 1 and 100".to_string(),
    ));
  }
  let offset = page_index
    .checked_mul(page_size)
    .ok_or_else(|| CommonWebError::BadInputWithSimpleMessage("page_index is too large".to_string()))?;

  let view_as = if is_author {
    ViewAs::Author
  } else if is_mod {
    ViewAs::Moderator
  } else {
    ViewAs::AnotherUser
  };

  let maybe_filter_media_types = get_scoped_media_types(query.filter_media_type.as_deref());
  let maybe_filter_media_classes = get_scoped_media_classes(query.filter_media_classes.as_deref());
  let maybe_filter_engine_categories = get_scoped_engine_categories(query.filter_engine_categories.as_deref());

  let filters = UserMediaFileFilters {
    username: &path.username,
    maybe_filter_media_types: maybe_filter_media_types.as_ref(),
    maybe_filter_media_classes: maybe_filter_media_classes.as_ref(),
    maybe_filter_engine_categories: maybe_filter_engine_categories.as_ref(),
    include_user_uploads: query.include_user_uploads.unwrap_or(false),
    view_as,
  };
  let page_started = Instant::now();
  let mut records = list_media_files_for_user(ListMediaFileForUserArgs {
    filters,
    limit: page_size + 1,
    offset,
    sort_ascending,
    mysql_executor: &server_state.mysql_pool,
  })
  .await?;
  let has_more = records.len() > page_size;
  records.truncate(page_size);
  info!(
    "Listed list_v2 user media: username={} page={} page_ms={}",
    path.username,
    page_index,
    page_started.elapsed().as_millis()
  );
  let results = build_user_media_file_response(UserMediaFileResponseArgs {
    records,
    is_allowed_studio_access,
    media_domain: get_media_domain(&http_request),
    server_environment: server_state.server_environment,
  });
  Ok(Json(ListMediaFilesForUserV2SuccessResponse {
    success: true,
    results,
    pagination: UserMediaFileV2Pagination {
      current: page_index,
      has_more,
    },
  }))
}
