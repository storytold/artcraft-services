use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use sqlx::{Executor, MySql, QueryBuilder};

use enums::by_table::media_files::media_file_class::MediaFileClass;
use enums::by_table::media_files::media_file_type::MediaFileType;
use enums::common::visibility::Visibility;
use tokens::tokens::media_files::MediaFileToken;
use tokens::tokens::users::UserToken;

/// Minimal columns the folder-thumbnail builder needs from `media_files`.
#[derive(Debug, Clone)]
pub struct MediaFileThumbnailRow {
  pub token: MediaFileToken,
  pub media_class: MediaFileClass,
  pub media_type: MediaFileType,
  pub public_bucket_directory_hash: String,
  pub maybe_public_bucket_prefix: Option<String>,
  pub maybe_public_bucket_extension: Option<String>,
  /// Which video preview files exist. See `video_thumbnail_suffixes` in `bucket_paths`.
  pub maybe_thumbnail_version: Option<u8>,
  pub created_at: DateTime<Utc>,
}

pub struct BatchGetMediaFileThumbnailsByTokensArgs<'e, 'c, E>
where
  E: 'e + Executor<'c, Database = MySql>,
{
  pub candidate_tokens: &'e [MediaFileToken],
  /// When set, private media files are only returned if this user created
  /// them. Pass the viewer whenever the result is shown to a user (e.g.
  /// folder thumbnails); `None` skips the visibility check.
  pub maybe_restrict_private_to_creator: Option<&'e UserToken>,
  pub mysql_executor: E,
  pub phantom: PhantomData<&'c E>,
}

/// Look up the bucket-path / class / type for each of the given media
/// file tokens. Silently drops rows that don't exist or are soft-deleted.
/// Caller is responsible for de-duplicating the input set if needed; the
/// SQL doesn't care about duplicates.
///
/// Uses runtime `QueryBuilder` because the IN-list size is dynamic.
pub async fn batch_get_media_file_thumbnails_by_tokens<'e, 'c: 'e, E>(
  args: BatchGetMediaFileThumbnailsByTokensArgs<'e, 'c, E>,
) -> Result<Vec<MediaFileThumbnailRow>, sqlx::Error>
where
  E: 'e + Executor<'c, Database = MySql>,
{
  if args.candidate_tokens.is_empty() {
    return Ok(Vec::new());
  }

  let mut builder = QueryBuilder::<MySql>::new(
    "SELECT \
       token, \
       media_class, \
       media_type, \
       public_bucket_directory_hash, \
       maybe_public_bucket_prefix, \
       maybe_public_bucket_extension, \
       maybe_thumbnail_version, \
       created_at \
     FROM media_files \
     WHERE user_deleted_at IS NULL \
       AND mod_deleted_at IS NULL ",
  );

  if let Some(viewer_user_token) = args.maybe_restrict_private_to_creator {
    builder.push(" AND (creator_set_visibility <> ");
    builder.push_bind(Visibility::Private.to_str());
    builder.push(" OR maybe_creator_user_token = ");
    builder.push_bind(viewer_user_token.as_str());
    builder.push(") ");
  }

  builder.push(" AND token IN (");

  let mut separated = builder.separated(", ");
  for token in args.candidate_tokens {
    separated.push_bind(token.as_str());
  }
  separated.push_unseparated(")");

  let rows = builder
    .build_query_as::<RawRow>()
    .fetch_all(args.mysql_executor)
    .await?;

  Ok(rows.into_iter().map(|r| MediaFileThumbnailRow {
    token: MediaFileToken::new(r.token),
    media_class: MediaFileClass::from_str(&r.media_class).unwrap_or(MediaFileClass::Image),
    media_type: MediaFileType::from_str(&r.media_type).unwrap_or(MediaFileType::Image),
    public_bucket_directory_hash: r.public_bucket_directory_hash,
    maybe_public_bucket_prefix: r.maybe_public_bucket_prefix,
    maybe_public_bucket_extension: r.maybe_public_bucket_extension,
    maybe_thumbnail_version: r.maybe_thumbnail_version,
    created_at: r.created_at,
  }).collect())
}

/// Internal row shape used by the dynamic `QueryBuilder` call. We hand-roll
/// the deserialization here because `QueryBuilder` can't ride the
/// compile-time type checks of `sqlx::query!`/`query_as!`.
#[derive(Debug, sqlx::FromRow)]
struct RawRow {
  token: String,
  media_class: String,
  media_type: String,
  public_bucket_directory_hash: String,
  maybe_public_bucket_prefix: Option<String>,
  maybe_public_bucket_extension: Option<String>,
  maybe_thumbnail_version: Option<u8>,
  created_at: DateTime<Utc>,
}
