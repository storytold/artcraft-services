use chrono::{DateTime, Utc};
use enums::by_table::media_files::media_file_class::MediaFileClass;
use sqlx::{Executor, MySql};
use tokens::tokens::media_files::MediaFileToken;

const DEFAULT_PAGE_SIZE: i64 = 100;

const DEFAULT_MAX_LOOKBACK_HOURS: i32 = 4;

pub struct ListVideoMediaFilesWithoutThumbnailsArgs<E> {
  /// Override the maximum number of hours in our lookback window.
  pub custom_max_lookback_hours: Option<i32>,
  /// Override how many results to fetch (given a full page of results)
  pub custom_page_size: Option<i64>,
  /// Cursor to continue paginating with
  pub maybe_id_cursor: Option<i64>,
  /// The MySQL executor to run the query on.
  pub executor: E,
}

pub struct VideoMediaFilesWithoutThumbnails {
  pub media_files: Vec<VideoMediaFileWithoutThumbnail>,
  pub next_cursor: Option<i64>,
}

pub struct VideoMediaFileWithoutThumbnail {
  pub id: i64,
  pub token: MediaFileToken,
  pub created_at: DateTime<Utc>,
  /// Database statement-start time, shared by every row in this page.
  pub database_read_at: DateTime<Utc>,
  /// Our job's successful finalization time, not the provider's completion time.
  /// Uploads, legacy records, and jobs still being finalized can have no timestamp.
  pub maybe_generation_completed_at: Option<DateTime<Utc>>,
  pub maybe_thumbnail_version: Option<u8>,
  pub public_bucket_directory_hash: String,
  pub maybe_public_bucket_prefix: Option<String>,
  pub maybe_public_bucket_extension: Option<String>,
}

pub async fn list_video_media_files_without_thumbnails_for_job<'e, 'c, E>(
  args: ListVideoMediaFilesWithoutThumbnailsArgs<E>,
) -> Result<VideoMediaFilesWithoutThumbnails, sqlx::Error>
  where E: 'e + Executor<'c, Database = MySql>
{
  let cursor = args.maybe_id_cursor.unwrap_or(i64::MAX);

  let page_size = args.custom_page_size.unwrap_or(DEFAULT_PAGE_SIZE);
  let max_lookback_hours = args.custom_max_lookback_hours.unwrap_or(DEFAULT_MAX_LOOKBACK_HOURS);

  const MEDIA_CLASS_VIDEO: &str = MediaFileClass::Video.to_str();

  // NB(1): `COALESCE ...` helps us force the index for a more performant query plan (otherwise it explodes into a table scan).
  // NB(2): `NOW() - INTERVAL ? HOUR` uses the database clock to avoid client/server clock skew.
  let media_files = sqlx::query_as!(
    VideoMediaFileWithoutThumbnail,
    r#"
SELECT
    media.id,
    media.token as `token: MediaFileToken`,
    media.created_at as `created_at: DateTime<Utc>`,
    NOW(6) as `database_read_at!: DateTime<Utc>`,
    jobs.successfully_completed_at as `maybe_generation_completed_at?: DateTime<Utc>`,
    media.maybe_thumbnail_version as `maybe_thumbnail_version: u8`,
    media.public_bucket_directory_hash,
    media.maybe_public_bucket_prefix,
    media.maybe_public_bucket_extension
FROM media_files AS media
LEFT JOIN generic_inference_jobs AS jobs ON jobs.token = media.maybe_source_job_token
WHERE
    media.id >= (SELECT COALESCE(MIN(id), 0) FROM media_files WHERE created_at >= NOW() - INTERVAL ? HOUR)
    AND media.id < ?
    AND media.media_class = ?
    AND media.maybe_thumbnail_version IS NULL
    AND media.user_deleted_at IS NULL
    AND media.mod_deleted_at IS NULL
ORDER BY media.id DESC
LIMIT ?
    "#,
    max_lookback_hours,
    cursor,
    MEDIA_CLASS_VIDEO,
    page_size,
  )
    .fetch_all(args.executor)
    .await?;

  let next_cursor = if media_files.len() as i64 == page_size {
    media_files.last().map(|f| f.id)
  } else {
    None
  };

  Ok(VideoMediaFilesWithoutThumbnails { media_files, next_cursor })
}
