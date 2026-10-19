use anyhow::anyhow;
use log::{error, info, warn};

use bucket_paths::legacy::typified_paths::public::media_files::bucket_file_path::MediaFileBucketPath;
use enums::by_table::generic_inference_jobs::inference_result_type::InferenceResultType;
use enums::by_table::media_files::media_file_class::MediaFileClass;
use enums::by_table::media_files::media_file_origin_category::MediaFileOriginCategory;
use enums::by_table::media_files::media_file_type::MediaFileType;
use enums::common::generation_provider::GenerationProvider;
use errors::AnyhowResult;
use hashing::sha256::sha256_hash_bytes::sha256_hash_bytes;
use mimetypes::mimetype_for_bytes::get_mimetype_for_bytes;
use mysql_queries::queries::generic_inference::api_providers::kinovi_web::list_pending_kinovi_web_video_jobs::PendingKinoviWebJob;
use mysql_queries::queries::generic_inference::job::select_inference_job_status_for_update::select_inference_job_status_for_update;
use mysql_queries::queries::generic_inference::web::mark_generic_inference_job_successfully_done_by_token_with_executor::{mark_generic_inference_job_successfully_done_by_token_with_executor, MarkGenericInferenceJobSuccessfullyDoneByTokenWithExecutorArgs};
use mysql_queries::queries::media_files::create::insert_builder::media_file_insert_builder::MediaFileInsertBuilder;
use kinovi_web_client::requests::poll_orders::poll_orders::{OrderStatus, MediaResult};
use tokens::tokens::batch_generations::BatchGenerationToken;

use crate::alert_on_error::alert_pager_and_return_err;
use crate::job_dependencies::JobDependencies;
use crate::jobs::order_processing_job::is_job_status_terminal::is_job_status_terminal;

const PREFIX: &str = "artcraft_";

/// Import a completed Suno audio order from Kinovi.
///
/// Preserve every returned clip in one batch and expose the first as the
/// primary result. Download and validate audio before marking the job complete.
pub async fn process_successful_audio_job(
  deps: &JobDependencies,
  job: &PendingKinoviWebJob,
  order: &OrderStatus,
) -> AnyhowResult<()> {
  let results = audio_results(order)?;

  info!(
    "Processing completed audio order {}: {} audio(s) to download.",
    order.order_id,
    results.len(),
  );

  // Complete all network IO before acquiring a database connection. Media
  // records and job completion are then committed together so a failed second
  // clip or a retry cannot expose partial/duplicate batches in the library.
  let maybe_batch_token = (results.len() > 1).then(BatchGenerationToken::generate);
  let mut media_files = Vec::with_capacity(results.len());
  for (idx, result) in results.iter().enumerate() {
    media_files.push(
      download_and_upload_audio(deps, job, order, result, idx, maybe_batch_token.as_ref()).await?,
    );
  }

  // Finalize inside a transaction: re-check the job is still pending under a row
  // lock, then mark it complete. See `process_successful_video_job` for the rationale.
  let mut transaction = deps.mysql_pool.begin().await.map_err(|err| {
    anyhow!(
      "error beginning finalize transaction for job {}: {:?}",
      job.job_token.as_str(),
      err
    )
  })?;

  let maybe_status = select_inference_job_status_for_update(&mut *transaction, &job.job_token)
    .await
    .map_err(|err| {
      anyhow!(
        "error locking job {} for finalize: {:?}",
        job.job_token.as_str(),
        err
      )
    })?;

  // A concurrent completion or cancellation must win over this import.
  let status = match maybe_status {
    Some(status) => status,
    None => {
      let _ = transaction.rollback().await;
      return Err(anyhow!(
        "Job {} vanished before finalize (order {})",
        job.job_token.as_str(),
        order.order_id,
      ));
    },
  };

  if is_job_status_terminal(status) {
    warn!(
      "Job {} is already terminal ({:?}); skipping mark-done (order {}). \
      {} bucket object(s) may be orphaned.",
      job.job_token.as_str(),
      status,
      order.order_id,
      media_files.len(),
    );
    let _ = transaction.rollback().await;
    return Ok(());
  }

  let mut primary_token = None;
  for media_file in &media_files {
    let token = media_file.insert_acquire(&mut *transaction).await?;
    if primary_token.is_none() {
      primary_token = Some(token);
    }
  }
  let primary_token = primary_token.ok_or_else(|| anyhow!("No audio results to finalize"))?;

  // Commit all clips and the job status within the same transaction.

  if let Err(err) = mark_generic_inference_job_successfully_done_by_token_with_executor(
    MarkGenericInferenceJobSuccessfullyDoneByTokenWithExecutorArgs {
      executor: &mut *transaction,
      token: &job.job_token,
      maybe_entity_type: Some(InferenceResultType::MediaFile),
      maybe_entity_token: Some(primary_token.as_str()),
      total_job_duration: None,
      inference_duration: None,
    },
  )
  .await
  {
    let _ = transaction.rollback().await;
    error!(
      "Error marking audio job {} done: {:?}",
      job.job_token.as_str(),
      err
    );
    return alert_pager_and_return_err(
      &deps.pager,
      "Kinovi audio job completion update failed",
      anyhow!("error marking job done: {:?}", err),
      Some(job),
    );
  }

  transaction.commit().await.map_err(|err| {
    anyhow!(
      "error committing finalize transaction for job {}: {:?}",
      job.job_token.as_str(),
      err
    )
  })?;

  info!(
    "Audio job {} completed successfully.",
    job.job_token.as_str()
  );

  Ok(())
}

/// Upload a clip and prepare its media record without holding a DB connection.
async fn download_and_upload_audio(
  deps: &JobDependencies,
  job: &PendingKinoviWebJob,
  order: &OrderStatus,
  result: &MediaResult,
  index: usize,
  maybe_batch_token: Option<&BatchGenerationToken>,
) -> AnyhowResult<MediaFileInsertBuilder> {
  let audio_url = result.url.as_str();

  info!(
    "Downloading audio {}/{} for order {} from: {}",
    index + 1,
    order.results.len().max(1),
    order.order_id,
    audio_url,
  );

  let audio_bytes: Vec<u8> = deps
    .download_client
    .get(audio_url)
    .send()
    .await
    .map_err(|err| anyhow!("reqwest error downloading audio {}: {:?}", index, err))?
    .error_for_status()
    .map_err(|err| anyhow!("HTTP error downloading audio {}: {:?}", index, err))?
    .bytes()
    .await
    .map_err(|err| anyhow!("error reading audio bytes for index {}: {:?}", index, err))?
    .to_vec();

  info!(
    "Downloaded {} bytes for audio {}/{} of order {}",
    audio_bytes.len(),
    index + 1,
    order.results.len().max(1),
    order.order_id,
  );

  let (suffix, mime_type, file_type) = audio_metadata(&audio_bytes)?;

  let checksum = sha256_hash_bytes(&audio_bytes)
    .map_err(|err| anyhow!("error hashing audio {}: {:?}", index, err))?;

  let bucket_path = MediaFileBucketPath::generate_new(Some(PREFIX), Some(suffix));
  let object_path = bucket_path.get_full_object_path_str();

  info!(
    "Uploading audio {}/{} to public bucket at path: {}",
    index + 1,
    order.results.len().max(1),
    object_path,
  );

  deps
    .public_bucket_client
    .upload_file_with_content_type_process(object_path, &audio_bytes, mime_type)
    .await
    .map_err(|err| anyhow!("bucket upload failed for audio {}: {:?}", index, err))?;

  let media_file = MediaFileInsertBuilder::new()
    .checksum_sha2(&checksum)
    .creator_ip_address(&job.creator_ip_address)
    .creator_set_visibility(job.creator_set_visibility)
    .file_size_bytes(audio_bytes.len() as u64)
    .maybe_batch_generation_token(maybe_batch_token)
    .maybe_creator_anonymous_visitor(job.maybe_creator_anonymous_visitor_token.as_ref())
    .maybe_creator_user(job.maybe_creator_user_token.as_ref())
    .maybe_generation_provider(Some(GenerationProvider::Artcraft))
    .maybe_prompt_token(job.maybe_prompt_token.as_ref())
    .maybe_source_job_token(Some(&job.job_token))
    .maybe_platform_type(job.maybe_platform_type)
    .media_file_class(MediaFileClass::Audio)
    .media_file_origin_category(MediaFileOriginCategory::Inference)
    .media_file_type(file_type)
    .mime_type(mime_type)
    .public_bucket_directory_hash(&bucket_path);

  Ok(media_file)
}

/// Some provider responses expose only a top-level URL. Do not lose those
/// single-clip orders or import a duplicate of the primary clip in a batch.
fn audio_results(order: &OrderStatus) -> AnyhowResult<Vec<MediaResult>> {
  if !order.results.is_empty() {
    return Ok(order.results.clone());
  }
  let url = order
    .result_url
    .as_ref()
    .filter(|url| !url.is_empty())
    .ok_or_else(|| anyhow!("Completed audio order {} has no results", order.order_id))?;
  Ok(vec![MediaResult {
    url: url.clone(),
    maybe_width: None,
    maybe_height: None,
  }])
}

fn audio_metadata(bytes: &[u8]) -> AnyhowResult<(&'static str, &'static str, MediaFileType)> {
  match get_mimetype_for_bytes(bytes) {
    Some("audio/mpeg") => Ok((".mp3", "audio/mpeg", MediaFileType::Mp3)),
    Some("audio/x-wav" | "audio/wav") => Ok((".wav", "audio/wav", MediaFileType::Wav)),
    Some("audio/flac" | "audio/x-flac") => Ok((".flac", "audio/flac", MediaFileType::Flac)),
    Some("audio/ogg") => Ok((".ogg", "audio/ogg", MediaFileType::Ogg)),
    Some("audio/x-m4a" | "audio/m4a") => Ok((".m4a", "audio/mp4", MediaFileType::M4a)),
    Some("audio/aac" | "audio/x-aac") => Ok((".aac", "audio/aac", MediaFileType::Aac)),
    other => Err(anyhow!(
      "Unsupported Kinovi audio result MIME type: {:?}",
      other
    )),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use kinovi_web_client::requests::poll_orders::poll_orders::{OrderMediaType, TaskStatus};

  #[test]
  fn accepts_mp3_and_wav_with_correct_file_metadata() {
    assert_eq!(
      audio_metadata(b"ID3\x04\x00\x00\x00\x00\x00\x00").unwrap(),
      (".mp3", "audio/mpeg", MediaFileType::Mp3)
    );
    assert_eq!(
      audio_metadata(b"RIFF\x24\x00\x00\x00WAVEfmt ").unwrap(),
      (".wav", "audio/wav", MediaFileType::Wav)
    );
  }

  #[test]
  fn rejects_video_and_error_pages_instead_of_importing_them_as_audio() {
    assert!(audio_metadata(b"\x00\x00\x00\x20ftypisom\x00\x00\x02\x00isomiso2avc1mp41").is_err());
    assert!(audio_metadata(b"<html>Access denied</html>").is_err());
  }

  #[test]
  fn preserves_both_clips_without_duplicating_the_primary_url() {
    let mut order = completed_order();
    order.result_url = Some("https://example.test/one.mp3".into());
    order.results = vec![
      clip("https://example.test/one.mp3"),
      clip("https://example.test/two.mp3"),
    ];
    let results = audio_results(&order).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].url, "https://example.test/one.mp3");
    assert_eq!(results[1].url, "https://example.test/two.mp3");
  }

  #[test]
  fn supports_top_level_only_results_and_rejects_missing_results() {
    let mut order = completed_order();
    assert!(audio_results(&order).is_err());
    order.result_url = Some("https://example.test/song.mp3".into());
    let results = audio_results(&order).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://example.test/song.mp3");
    order.result_url = Some(String::new());
    assert!(audio_results(&order).is_err());
  }

  fn clip(url: &str) -> MediaResult {
    MediaResult {
      url: url.into(),
      maybe_width: None,
      maybe_height: None,
    }
  }

  fn completed_order() -> OrderStatus {
    OrderStatus {
      order_id: "audio-order".into(),
      task_status: TaskStatus::Completed,
      result_url: None,
      results: vec![],
      fail_reason: None,
      created_at: "2026-10-18T00:00:00Z".into(),
      created_at_utc: None,
      media_type: Some(OrderMediaType::Audio),
      total_credits: None,
    }
  }
}
