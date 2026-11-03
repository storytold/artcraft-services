use sqlx::{Executor, MySql, QueryBuilder};

use enums::by_table::entity_stats::stats_entity_type::StatsEntityType;
use super::user_media_file_filters::UserMediaFileFilters;
use super::user_media_file_list_item::{MediaFileListItem, MediaFileListItemInternal};

const RESULT_FIELDS: &str = r#"
    m.id,
    m.token,

    m.media_class,
    m.media_type,

    m.maybe_engine_category,
    m.maybe_animation_type,

    m.origin_category,
    m.origin_product_category,

    m.maybe_origin_model_type,
    m.maybe_origin_model_token,
    
    w.title as maybe_origin_model_title,

    m.public_bucket_directory_hash,
    m.maybe_public_bucket_prefix,
    m.maybe_public_bucket_extension,
    m.maybe_thumbnail_version,

    m.maybe_prompt_token,

    m.creator_set_visibility,

    m.is_user_upload,
    m.is_intermediate_system_file,

    media_file_cover_image.public_bucket_directory_hash as maybe_file_cover_image_public_bucket_hash,
    media_file_cover_image.maybe_public_bucket_prefix as maybe_file_cover_image_public_bucket_prefix,
    media_file_cover_image.maybe_public_bucket_extension as maybe_file_cover_image_public_bucket_extension,

    m.maybe_title,
    m.maybe_text_transcript,
    prompts.maybe_other_args as maybe_other_prompt_args,
    m.maybe_duration_millis,

    entity_stats.ratings_positive_count as maybe_ratings_positive_count,
    entity_stats.ratings_negative_count as maybe_ratings_negative_count,
    entity_stats.bookmark_count as maybe_bookmark_count,

    m.created_at,
    m.updated_at
  "#;

pub struct ListMediaFileForUserArgs<'a, T> {
  pub filters: UserMediaFileFilters<'a>,
  pub limit: usize,
  pub offset: usize,
  pub sort_ascending: bool,
  pub mysql_executor: T,
}

/// Select a bounded page before joining potentially large prompt/metadata rows.
/// `LIMIT` prevents MySQL from merging the derived table into the outer query.
pub async fn list_media_files_for_user<'c, T>(
  args: ListMediaFileForUserArgs<'_, T>,
) -> Result<Vec<MediaFileListItem>, sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  let order = if args.sort_ascending { " ASC" } else { " DESC" };
  let mut query = QueryBuilder::<MySql>::new(format!("SELECT {RESULT_FIELDS} FROM (SELECT m.id "));
  // This index is in the July 2026 migrations. Restricting the scan to this user
  // avoids index-merge scans of the global deletion/class indexes on large tables.
  query.push("FROM media_files m FORCE INDEX (idx_creator_created_at) ");
  args.filters.push_predicates(&mut query);
  query.push(" ORDER BY m.created_at").push(order).push(", m.id").push(order);
  query.push(" LIMIT ").push_bind(args.limit as u64);
  query.push(" OFFSET ").push_bind(args.offset as u64);
  query.push(
    r#") AS page
JOIN media_files m ON m.id = page.id
LEFT JOIN model_weights w ON m.maybe_origin_model_token = w.token
LEFT JOIN media_files media_file_cover_image ON media_file_cover_image.token = m.maybe_cover_image_media_file_token
  AND media_file_cover_image.user_deleted_at IS NULL
  AND media_file_cover_image.mod_deleted_at IS NULL
  AND (media_file_cover_image.creator_set_visibility <> 'private' OR media_file_cover_image.maybe_creator_user_token = m.maybe_creator_user_token)
LEFT JOIN entity_stats ON entity_stats.entity_type = "#,
  );
  query.push_bind(StatsEntityType::MediaFile.to_str());
  query
    .push(
      r#" AND entity_stats.entity_token = m.token
LEFT JOIN prompts ON prompts.token = m.maybe_prompt_token
ORDER BY m.created_at"#,
    )
    .push(order)
    .push(", m.id")
    .push(order);
  let records = query.build_query_as::<MediaFileListItemInternal>().fetch_all(args.mysql_executor).await?;
  Ok(records.into_iter().map(MediaFileListItem::from).collect())
}
