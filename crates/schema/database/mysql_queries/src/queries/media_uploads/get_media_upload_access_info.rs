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

/// Look up the creator and visibility of a media upload. Excludes soft-deleted rows.
///
/// NB: Runtime query because `maybe_creator_user_token` on `media_uploads` has no offline cache
/// entry yet; replace with `sqlx::query_as!` after the next `cargo sqlx prepare`.
pub async fn get_media_upload_access_info<'e, 'c: 'e, E>(
  media_upload_token: &MediaUploadToken,
  mysql_executor: E,
) -> Result<Option<MediaUploadAccessInfo>, sqlx::Error>
  where E: 'e + Executor<'c, Database = MySql>
{
  let maybe_row: Option<(Option<String>, Visibility)> = sqlx::query_as(r#"
SELECT
  maybe_creator_user_token,
  creator_set_visibility
FROM media_uploads
WHERE token = ?
  AND user_deleted_at IS NULL
  AND mod_deleted_at IS NULL
LIMIT 1
    "#)
      .bind(media_upload_token.as_str())
      .fetch_optional(mysql_executor)
      .await?;

  Ok(maybe_row.map(|(maybe_creator_user_token, creator_set_visibility)| MediaUploadAccessInfo {
    maybe_creator_user_token: maybe_creator_user_token.map(|t| UserToken::new_from_str(&t)),
    creator_set_visibility,
  }))
}
