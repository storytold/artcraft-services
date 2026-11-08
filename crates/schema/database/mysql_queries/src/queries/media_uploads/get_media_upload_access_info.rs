use sqlx::{Executor, MySql};

use enums::common::visibility::Visibility;
use tokens::tokens::media_uploads::MediaUploadToken;
use tokens::tokens::users::UserToken;

/// The fields needed to decide who may use a (non-deleted) media upload.
#[derive(Debug, Clone)]
pub struct MediaUploadAccessInfo {
  pub maybe_creator_user_token: Option<UserToken>,
  pub creator_set_visibility: Visibility,
}

/// Inputs for [`get_media_upload_access_info`].
pub struct GetMediaUploadAccessInfoArgs<'a, T> {
  pub media_upload_token: &'a MediaUploadToken,
  pub mysql_executor: T,
}

/// Look up the creator and visibility of a media upload. Excludes soft-deleted rows.
pub async fn get_media_upload_access_info<'c, T>(
  args: GetMediaUploadAccessInfoArgs<'_, T>,
) -> Result<Option<MediaUploadAccessInfo>, sqlx::Error>
  where T: Executor<'c, Database = MySql>
{
  sqlx::query_as!(MediaUploadAccessInfo, r#"
SELECT
  mu.maybe_creator_user_token as `maybe_creator_user_token: tokens::tokens::users::UserToken`,
  mu.creator_set_visibility as `creator_set_visibility: enums::common::visibility::Visibility`
FROM media_uploads as mu
WHERE mu.token = ?
  AND mu.user_deleted_at IS NULL
  AND mu.mod_deleted_at IS NULL
LIMIT 1
    "#, args.media_upload_token.as_str())
    .fetch_optional(args.mysql_executor)
    .await
}
