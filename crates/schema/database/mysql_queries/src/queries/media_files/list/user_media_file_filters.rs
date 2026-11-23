use std::collections::HashSet;

use sqlx::{MySql, QueryBuilder};

use enums::by_table::media_files::media_file_class::MediaFileClass;
use enums::by_table::media_files::media_file_engine_category::MediaFileEngineCategory;
use enums::by_table::media_files::media_file_type::MediaFileType;
use enums::common::view_as::ViewAs;
use enums::common::visibility::Visibility;

/// Shared predicates keep the optional total and the returned page consistent.
#[derive(Clone, Copy)]
pub struct UserMediaFileFilters<'a> {
  pub username: &'a str,
  pub maybe_filter_media_types: Option<&'a HashSet<MediaFileType>>,
  pub maybe_filter_media_classes: Option<&'a HashSet<MediaFileClass>>,
  pub maybe_filter_engine_categories: Option<&'a HashSet<MediaFileEngineCategory>>,
  pub include_user_uploads: bool,
  pub view_as: ViewAs,
}

impl<'a> UserMediaFileFilters<'a> {
  pub(super) fn push_predicates(&self, query: &mut QueryBuilder<MySql>) {
    // Resolve the unique username once; subsequent scans are scoped to its token.
    query.push(" WHERE m.maybe_creator_user_token = (SELECT token FROM users WHERE username = ");
    query.push_bind(self.username).push(")");
    if let Some(values) = self.maybe_filter_media_types.filter(|values| !values.is_empty()) {
      query.push(" AND m.media_type IN (");
      let mut separated = query.separated(", ");
      for value in values {
        separated.push_bind(value.to_str());
      }
      separated.push_unseparated(")");
    }
    if let Some(values) = self.maybe_filter_media_classes.filter(|values| !values.is_empty()) {
      query.push(" AND m.media_class IN (");
      let mut separated = query.separated(", ");
      for value in values {
        separated.push_bind(value.to_str());
      }
      separated.push_unseparated(")");
    }
    if let Some(values) = self.maybe_filter_engine_categories.filter(|values| !values.is_empty()) {
      query.push(" AND m.maybe_engine_category IN (");
      let mut separated = query.separated(", ");
      for value in values {
        separated.push_bind(value.to_str());
      }
      separated.push_unseparated(")");
    }
    if !self.include_user_uploads {
      query.push(" AND NOT m.is_user_upload");
    }
    query.push(" AND NOT m.is_intermediate_system_file AND m.user_deleted_at IS NULL AND m.mod_deleted_at IS NULL");
    if self.view_as == ViewAs::AnotherUser {
      query.push(" AND m.creator_set_visibility = ").push_bind(Visibility::Public.to_str());
    }
  }
}
