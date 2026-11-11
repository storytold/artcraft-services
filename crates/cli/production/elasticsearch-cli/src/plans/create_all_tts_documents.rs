use tokio::fs::read_to_string;

use elasticsearch::{BulkOperation, BulkParts, Elasticsearch};
use log::{error, info};
use serde_json::Value;
use sqlx::{MySql, Pool};

use elasticsearch_schema::documents::tts_model_document::{TTS_MODEL_INDEX, TtsModelDocument};
use elasticsearch_schema::traits::document::Document;
use elasticsearch_schema::utils::create_index_if_not_exists::{create_index_if_not_exists, CreateIndexArgs};
use errors::AnyhowResult;
use mysql_queries::queries::tts::tts_models::list_tts_models::{list_tts_models, TtsModelRecordForList};
use storyteller_root::get_storyteller_rust_root;
use tokens::tokens::tts_models::TtsModelToken;
use tokens::tokens::users::UserToken;

pub async fn create_all_tts_documents(mysql: &Pool<MySql>, elasticsearch: &Elasticsearch) -> AnyhowResult<()> {
  info!("Create all TTS documents.");

  const SCOPE_USERNAME : Option<&str> = None;
  const REQUIRE_MOD_APPROVED : bool = false;

  // TODO(bt,2023-10-26): Paginate this query (!!)
  let all_tts_models = list_tts_models(
    mysql,
    SCOPE_USERNAME,
    REQUIRE_MOD_APPROVED
  ).await?;

  create_tts_model_index(&elasticsearch, true).await?;

  for record in all_tts_models {
    create_document_from_record(elasticsearch, &record).await?;
  }

  Ok(())
}

async fn create_document_from_record(elasticsearch: &Elasticsearch, record: &TtsModelRecordForList) -> AnyhowResult<()> {
  info!("Create record for {:?} - {:?}", record.model_token, record.title);

  let document = TtsModelDocument {
    token: TtsModelToken::new_from_str(&record.model_token),
    title: record.title.clone(),
    ietf_language_tag: record.ietf_language_tag.clone(),
    ietf_primary_language_subtag: record.ietf_primary_language_subtag.clone(),
    creator_username: record.creator_username.clone(),
    creator_display_name: record.creator_display_name.clone(),
    creator_user_token: UserToken::new_from_str(&record.creator_user_token),
    creator_set_visibility: record.creator_set_visibility,
    created_at: record.created_at,
    updated_at: record.updated_at,
  };

  let op : BulkOperation<_> = BulkOperation::index(&document)
      .id(document.get_document_id())
      .into();

  let response = elasticsearch
      .bulk(BulkParts::Index(TTS_MODEL_INDEX))
      .body(vec![op])
      .send()
      .await?;

  let json: Value = response.json().await?;

  let had_errors = json["errors"].as_bool().unwrap_or(false);
  if had_errors {
    let failed: Vec<&Value> = json["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| !v["error"].is_null())
        .collect();

    // TODO: retry failures
    error!("Errors during indexing. Failures: {}", failed.len());
  }

  Ok(())
}

// NB: Adapted from elasticsearch crate examples source
async fn create_tts_model_index(client: &Elasticsearch, _delete: bool) -> AnyhowResult<()> {

  info!("Creating TTS model index...");

  let index_path = get_storyteller_rust_root()
      .join("_database/elasticsearch/index_definitions/tts_models_v1.json");

  info!("Reading index file: {:?}", index_path);

  let index_definition = read_to_string(index_path).await?;

  create_index_if_not_exists(CreateIndexArgs {
    client,
    index_name: TTS_MODEL_INDEX,
    index_definition: &index_definition,
    delete_existing: true,
  }).await?;

  Ok(())
}
