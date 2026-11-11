use std::collections::HashMap;
use std::marker::PhantomData;

use log::warn;
use sqlx::MySqlPool;

use mysql_queries::queries::media_files::get::batch_get_media_file_thumbnails_by_tokens::{batch_get_media_file_thumbnails_by_tokens, BatchGetMediaFileThumbnailsByTokensArgs};
use tokens::tokens::media_files::MediaFileToken;

use crate::http_server::common_responses::media::media_links_builder::VideoThumbnailInfo;

/// Look up the video thumbnail info (`maybe_thumbnail_version`, `created_at`)
/// for media files that didn't come from MySQL, eg. Elasticsearch search
/// results, whose documents don't carry the thumbnail version.
///
/// Callers should pass only the tokens of `.mp4` files (the only files with
/// video previews). Tokens missing from the result (deleted rows, or a failed
/// query) should fall back to `VideoThumbnailInfo::new(None, created_at)`,
/// which links the same files as a video the thumbnail job hasn't reached yet.
pub async fn lookup_video_thumbnail_info_by_tokens(
  tokens: &[MediaFileToken],
  mysql_pool: &MySqlPool,
) -> HashMap<MediaFileToken, VideoThumbnailInfo> {
  if tokens.is_empty() {
    return HashMap::new();
  }

  let result = batch_get_media_file_thumbnails_by_tokens(BatchGetMediaFileThumbnailsByTokensArgs {
    candidate_tokens: tokens,
    // Only the thumbnail version and timestamp are returned, for files the
    // caller already resolved; no URLs come out of this lookup.
    maybe_restrict_private_to_creator: None,
    mysql_executor: mysql_pool,
    phantom: PhantomData,
  }).await;

  match result {
    Ok(rows) => rows.into_iter()
        .map(|row| (row.token, VideoThumbnailInfo::new(row.maybe_thumbnail_version, row.created_at)))
        .collect(),
    Err(err) => {
      warn!("Error looking up video thumbnail info; falling back to NULL versions: {:?}", err);
      HashMap::new()
    }
  }
}
