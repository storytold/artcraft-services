use std::collections::HashSet;
use std::time::Duration;

use log::info;
use sqlx::{MySql, Pool};

use enums::by_table::media_files::media_file_type::MediaFileType;
use enums::common::view_as::ViewAs;
use errors::AnyhowResult;
use mysql_queries::queries::media_files::delete::delete_media_file::delete_media_file_as_mod;
use mysql_queries::queries::media_files::list::list_media_files::{list_media_files, ListMediaFilesArgs};

use crate::cli_args::Args;
use crate::util::constants::ECHELON_USER_TOKEN;

pub async fn delete_all_anonymous_user_images(_args: &Args, mysql: &Pool<MySql>) -> AnyhowResult<()> {
  info!("delete anonymous user images");

  // Uploads are stored with a concrete, MIME-derived type (Jpg, Png, ...);
  // only old records use the coarse `Image` type. Select all of them.
  let image_media_types = image_media_types();

  let mut page_index = 0;
  let mut maybe_offset = None;

  loop {
    info!("Querying page {page_index} of files...");

    let media_files = list_media_files(ListMediaFilesArgs {
      limit: 100,
      maybe_filter_media_types: Some(&image_media_types),
      maybe_filter_media_classes: None,
      maybe_offset,
      cursor_is_reversed: false,
      sort_ascending: false,
      view_as: ViewAs::Moderator,
      mysql_pool: &mysql,
      maybe_filter_engine_categories: None,
      include_user_uploads: true,
    }).await?;

    if media_files.records.is_empty() {
      break;
    }

    for file in media_files.records {
      if file.maybe_creator_user_token.is_some() {
        info!("Skipping file: {:?} , which has author: {:?}", &file.token, &file.maybe_creator_user_token);
        continue;
      }

      if !file.media_type.is_image() {
        info!("Skipping file: {:?} , which is not an image", &file.token);
        continue;
      }

      info!("Deleting image file: {:?}", &file.token);
      delete_media_file_as_mod(&file.token, ECHELON_USER_TOKEN, mysql).await?;
    }

    page_index += 1;

    maybe_offset = media_files.last_id.map(|id| id as usize);

    tokio::time::sleep(Duration::from_millis(1000)).await;
  }

  Ok(())
}

/// Every media type that `MediaFileType::is_image` accepts, so that the
/// listing filter and the per-record check agree. The legacy `Image` type
/// is deprecated but still present on old rows, which this must cover.
#[allow(deprecated)]
fn image_media_types() -> HashSet<MediaFileType> {
  HashSet::from([
    MediaFileType::Image,
    MediaFileType::Jpg,
    MediaFileType::Png,
    MediaFileType::Gif,
    MediaFileType::Webp,
  ])
}

#[cfg(test)]
mod tests {
  use super::image_media_types;

  #[test]
  fn selection_covers_every_image_type() {
    let types = image_media_types();
    assert!(types.iter().all(|t| t.is_image()));
    assert_eq!(types.len(), 5);
  }
}
