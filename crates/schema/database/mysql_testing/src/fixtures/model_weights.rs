//! Model weight fixtures. The bucket hash is fake; nothing downloads the file.

use anyhow::anyhow;
use enums::by_table::model_weights::weights_category::WeightsCategory;
use enums::by_table::model_weights::weights_types::WeightsType;
use enums::common::visibility::Visibility;
use sqlx::MySqlPool;
use tokens::tokens::model_weights::ModelWeightToken;
use tokens::tokens::users::UserToken;

use mysql_queries::queries::model_weights::create::create_weight::{create_weight, CreateModelWeightsArgs};

/// Insert a public RVC v2 weight owned by `owner`.
pub async fn create_test_model_weight(
  pool: &MySqlPool,
  owner: &UserToken,
) -> anyhow::Result<ModelWeightToken> {
  let token = ModelWeightToken::generate();

  create_weight(CreateModelWeightsArgs {
    token: &token,
    weights_type: WeightsType::RvcV2,
    weights_category: WeightsCategory::VoiceConversion,
    title: format!("Test weight {}", token.as_str()),
    maybe_cover_image_media_file_token: None,
    maybe_description_markdown: None,
    maybe_description_rendered_html: None,
    creator_user_token: Some(owner),
    creator_ip_address: "127.0.0.1",
    creator_set_visibility: Visibility::Public,
    maybe_last_update_user_token: None,
    original_download_url: None,
    original_filename: None,
    file_size_bytes: 1_024,
    file_checksum_sha2: "0".repeat(64),
    public_bucket_hash: token.as_str().to_lowercase(),
    maybe_public_bucket_prefix: None,
    maybe_public_bucket_extension: None,
    version: 1,
    mysql_pool: pool,
  })
  .await
  .map_err(|err| anyhow!("model weight insert failed: {err:?}"))
}
