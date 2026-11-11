use std::marker::PhantomData;

use sqlx::{Executor, MySql, QueryBuilder, Row};

use enums::common::visibility::Visibility;
use tokens::tokens::media_files::MediaFileToken;
use tokens::tokens::users::UserToken;

pub struct FindMediaFilesWithUnreadableCoverArgs<'e, 'c, E>
where
  E: 'e + Executor<'c, Database = MySql>,
{
  pub media_file_tokens: &'e [MediaFileToken],
  /// Covers that are private are readable only by their creator.
  pub viewer_user_token: &'e UserToken,
  pub mysql_executor: E,
  pub phantom: PhantomData<&'c E>,
}

/// Return the input media files whose cover image is private and was not created by the viewer.
/// A media file's cover can belong to a different user than the file itself.
pub async fn find_media_files_with_unreadable_cover<'e, 'c: 'e, E>(
  args: FindMediaFilesWithUnreadableCoverArgs<'e, 'c, E>,
) -> Result<Vec<MediaFileToken>, sqlx::Error>
where
  E: 'e + Executor<'c, Database = MySql>,
{
  if args.media_file_tokens.is_empty() {
    return Ok(Vec::new());
  }

  let mut builder = QueryBuilder::<MySql>::new(
    "SELECT mf.token FROM media_files AS mf \
       JOIN media_files AS cover ON cover.token = mf.maybe_cover_image_media_file_token \
       WHERE cover.creator_set_visibility = ",
  );
  builder.push_bind(Visibility::Private.to_str());
  builder.push(" AND NOT (cover.maybe_creator_user_token <=> ");
  builder.push_bind(args.viewer_user_token.as_str());
  builder.push(") AND mf.token IN (");

  let mut separated = builder.separated(", ");
  for token in args.media_file_tokens {
    separated.push_bind(token.as_str());
  }
  separated.push_unseparated(")");

  let rows = builder.build().fetch_all(args.mysql_executor).await?;

  Ok(rows.into_iter()
    .map(|row| MediaFileToken::new(row.get::<String, _>(0)))
    .collect())
}
