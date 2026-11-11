use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::pool::PoolConnection;
use sqlx::{FromRow, MySql, QueryBuilder, Row};

use enums::by_table::prompt_context_items::prompt_context_semantic_type::PromptContextSemanticType;
use enums::common::visibility::Visibility;
use enums::traits::mysql_from_row::MySqlFromRow;
use tokens::tokens::media_files::MediaFileToken;
use tokens::tokens::prompts::PromptToken;
use tokens::tokens::users::UserToken;
use tokens::traits::mysql_token_from_row::MySqlTokenFromRow;

/// A prompt context item with its owning prompt token, for batch queries.
pub struct BatchPromptContextItem {
  pub prompt_token: PromptToken,
  pub media_token: MediaFileToken,
  pub context_semantic_type: PromptContextSemanticType,
  pub public_bucket_directory_hash: String,
  pub maybe_public_bucket_prefix: Option<String>,
  pub maybe_public_bucket_extension: Option<String>,
  /// Which video preview files exist. See `video_thumbnail_suffixes` in `bucket_paths`.
  pub maybe_thumbnail_version: Option<u8>,
  pub created_at: DateTime<Utc>,
  /// The media keeps its own visibility: callers must hide private items from non-creators.
  pub creator_set_visibility: Visibility,
  pub maybe_creator_user_token: Option<UserToken>,
}

impl FromRow<'_, MySqlRow> for BatchPromptContextItem {
  fn from_row(row: &MySqlRow) -> Result<Self, sqlx::Error> {
    Ok(Self {
      prompt_token: PromptToken::try_from_mysql_row(row, "prompt_token")?,
      media_token: MediaFileToken::try_from_mysql_row(row, "media_token")?,
      context_semantic_type: PromptContextSemanticType::try_from_mysql_row(row, "context_semantic_type")?,
      public_bucket_directory_hash: row.try_get("public_bucket_directory_hash")?,
      maybe_public_bucket_prefix: row.try_get("maybe_public_bucket_prefix")?,
      maybe_public_bucket_extension: row.try_get("maybe_public_bucket_extension")?,
      maybe_thumbnail_version: row.try_get("maybe_thumbnail_version")?,
      created_at: row.try_get("created_at")?,
      creator_set_visibility: Visibility::try_from_mysql_row(row, "creator_set_visibility")?,
      maybe_creator_user_token: UserToken::try_from_mysql_row_nullable(row, "maybe_creator_user_token")?,
    })
  }
}

/// Batch fetch all context items for multiple prompts in a single query.
/// Returns items grouped by their prompt_token (via the prompt_token field).
pub async fn batch_list_prompt_context_items(
  prompt_tokens: &[PromptToken],
  mysql_connection: &mut PoolConnection<MySql>,
) -> Result<Vec<BatchPromptContextItem>, sqlx::Error> {
  if prompt_tokens.is_empty() {
    return Ok(Vec::new());
  }

  let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(r#"
SELECT
    pci.prompt_token,
    pci.media_token,
    pci.context_semantic_type,

    m.public_bucket_directory_hash,
    m.maybe_public_bucket_prefix,
    m.maybe_public_bucket_extension,
    m.maybe_thumbnail_version,
    m.created_at,
    m.creator_set_visibility,
    m.maybe_creator_user_token

FROM prompt_context_items pci
JOIN media_files m ON pci.media_token = m.token
WHERE pci.prompt_token IN (
"#);

  let mut separated = query_builder.separated(", ");
  for token in prompt_tokens {
    separated.push_bind(token.to_string());
  }
  separated.push_unseparated(") ");
  // NB: Deleted media must not stay reachable through the prompts that used it.
  query_builder.push(" AND m.user_deleted_at IS NULL AND m.mod_deleted_at IS NULL ");

  let query = query_builder.build_query_as::<BatchPromptContextItem>();

  let results = query.fetch_all(&mut **mysql_connection).await?;

  Ok(results)
}
