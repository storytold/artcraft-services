use sqlx::pool::PoolConnection;
use sqlx::MySql;

use enums::common::visibility::Visibility;
use tokens::tokens::media_uploads::MediaUploadToken;
use tokens::tokens::users::UserToken;

/// The fields needed to decide whether a requester may use a media upload
/// as an input to their own job.
#[derive(Debug, Clone)]
pub struct MediaUploadAccessInfo {
  pub maybe_creator_user_token: Option<UserToken>,
  pub creator_set_visibility: Visibility,
}

/// Look up the creator and visibility of a non-deleted media upload.
/// Returns `None` if the upload doesn't exist or has been deleted.
pub async fn get_media_upload_access_info(
  media_upload_token: &MediaUploadToken,
  mysql_connection: &mut PoolConnection<MySql>,
) -> Result<Option<MediaUploadAccessInfo>, sqlx::Error> {
  // NB: Runtime query (not `query!`) so no offline metadata is needed.
  let maybe_row: Option<(Option<String>, Visibility)> = sqlx::query_as(
    r#"
SELECT
    mu.maybe_creator_user_token,
    mu.creator_set_visibility
FROM media_uploads as mu
WHERE
    mu.token = ?
    AND mu.user_deleted_at IS NULL
    AND mu.mod_deleted_at IS NULL
LIMIT 1
    "#)
      .bind(media_upload_token.as_str())
      .fetch_optional(&mut **mysql_connection)
      .await?;

  Ok(maybe_row.map(|(maybe_creator_user_token, creator_set_visibility)| MediaUploadAccessInfo {
    maybe_creator_user_token: maybe_creator_user_token.map(UserToken::new),
    creator_set_visibility,
  }))
}
