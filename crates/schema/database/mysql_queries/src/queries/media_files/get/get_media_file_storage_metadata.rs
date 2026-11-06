use sqlx::{Executor, MySql};
use tokens::tokens::media_files::MediaFileToken;

pub struct MediaFileStorageMetadata {
  pub file_size_bytes: i32,
  pub checksum_sha2: String,
}

pub struct GetMediaFileStorageMetadataArgs<'a, T> {
  pub media_file_token: &'a MediaFileToken,
  pub mysql_executor: T,
}

/// Stored byte count and checksum, without changing the public media-file API.
pub async fn get_media_file_storage_metadata<'c, T>(args: GetMediaFileStorageMetadataArgs<'_, T>) -> Result<MediaFileStorageMetadata, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query_as!(MediaFileStorageMetadata, "SELECT file_size_bytes, checksum_sha2 FROM media_files WHERE token = ?", args.media_file_token.as_str(),).fetch_one(args.mysql_executor).await
}
