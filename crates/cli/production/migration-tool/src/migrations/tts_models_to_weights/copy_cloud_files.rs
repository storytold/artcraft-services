use bucket_paths::legacy::old_bespoke_paths::bucket_path_unifier::BucketPathUnifier;
use bucket_paths::legacy::typified_paths::public::weight_files::bucket_file_path::WeightFileBucketPath;
use errors::AnyhowResult;
use filesys::file_deletion::safe_delete_directory::safe_delete_directory;
use filesys::file_deletion::safe_delete_file::safe_delete_file;
use hashing::sha256::sha256_hash_file::sha256_hash_file;
use mysql_queries::queries::model_weights::migration::upsert_model_weight_from_tts_model::CopiedTtsFileData;
use mysql_queries::queries::tts::tts_models::migration::list_whole_tts_models_using_cursor::WholeTtsModelRecord;

use crate::deps::Deps;

pub async fn copy_cloud_files(model: &WholeTtsModelRecord, deps: &Deps) -> AnyhowResult<CopiedTtsFileData> {
  let copied_file_data = copy_model(model, deps).await?;
  Ok(copied_file_data)
}

async fn copy_model(model: &WholeTtsModelRecord, deps: &Deps) -> AnyhowResult<CopiedTtsFileData> {
  let bucket_path_unifier = BucketPathUnifier::default_paths();

  let old_model_bucket_path = bucket_path_unifier.tts_synthesizer_path(&model.private_bucket_hash);

  // TODO(bt,2023-12-19): Probably faster to stream between buckets, but whatever.
  let temp_dir = tempfile::Builder::new().prefix("model_transfer").tempdir()?;
  let model_temp_fs_path = temp_dir.path().join("model.bin");

  deps.bucket_production_private.download_file_to_disk(&old_model_bucket_path, &model_temp_fs_path).await?;

  let file_checksum = sha256_hash_file(&model_temp_fs_path)?;

  let new_model_bucket_path = WeightFileBucketPath::generate_for_tt2_model();

  deps.bucket_production_public.upload_filename_with_content_type(
    &new_model_bucket_path.get_full_object_path_str(),
    &model_temp_fs_path,
    "application/octet-stream").await?;

  safe_delete_file(&model_temp_fs_path);
  safe_delete_directory(&temp_dir);

  Ok(CopiedTtsFileData {
    bucket_path: new_model_bucket_path,
    file_sha_hash: file_checksum,
  })
}

