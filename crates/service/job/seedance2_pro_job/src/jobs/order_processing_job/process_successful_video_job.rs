use anyhow::anyhow;
use log::{error, info, warn};

use bucket_paths::legacy::typified_paths::public::media_files::bucket_file_path::MediaFileBucketPath;
use enums::by_table::generic_inference_jobs::inference_result_type::InferenceResultType;
use enums::by_table::media_files::media_file_class::MediaFileClass;
use enums::by_table::media_files::media_file_origin_category::MediaFileOriginCategory;
use enums::by_table::media_files::media_file_origin_product_category::MediaFileOriginProductCategory;
use enums::by_table::media_files::media_file_type::MediaFileType;
use enums::common::generation_provider::GenerationProvider;
use errors::AnyhowResult;
use ffmpeg_utils::ffprobe::ffprobe_get_info::{ffprobe_get_info_from_bytes, VideoInfo};
use mimetypes::mimetype_for_bytes::get_mimetype_for_bytes;
use hashing::sha256::sha256_hash_bytes::sha256_hash_bytes;
use mysql_queries::queries::generic_inference::api_providers::kinovi_web::list_pending_kinovi_web_video_jobs::PendingKinoviWebJob;
use mysql_queries::queries::generic_inference::job::select_inference_job_status_for_update::select_inference_job_status_for_update;
use mysql_queries::queries::generic_inference::web::mark_generic_inference_job_successfully_done_by_token_with_executor::{mark_generic_inference_job_successfully_done_by_token_with_executor, MarkGenericInferenceJobSuccessfullyDoneByTokenWithExecutorArgs};
use mysql_queries::queries::media_files::create::insert_builder::media_file_insert_builder::MediaFileInsertBuilder;
use kinovi_web_client::requests::poll_orders::poll_orders::OrderStatus;

use crate::alert_on_error::alert_pager_and_return_err;
use crate::job_dependencies::JobDependencies;
use crate::jobs::order_processing_job::is_job_status_terminal::is_job_status_terminal;

const PREFIX: &str = "artcraft_";

/// Download the completed video, upload to bucket, create media file record, and mark job done.
pub async fn process_successful_video_job(
  deps: &JobDependencies,
  job: &PendingKinoviWebJob,
  order: &OrderStatus,
) -> AnyhowResult<()> {
  // Get the video URL.
  let video_url = match &order.result_url {
    Some(url) => url.as_str(),
    None => {
      // Fall back to the first result entry if the top-level result_url is missing.
      match order.results.first() {
        Some(result) => result.url.as_str(),
        None => {
          return Err(anyhow!(
            "Completed order {} has no result_url and no results",
            order.order_id
          ));
        }
      }
    }
  };

  info!(
    "Downloading video for order {} from: {}",
    order.order_id, video_url
  );

  // Download the video bytes.
  let video_bytes: Vec<u8> = match deps.download_client.get(video_url).send().await.and_then(|resp| resp.error_for_status()) {
    Ok(resp) => match resp.bytes().await {
      Ok(bytes) => bytes.to_vec(),
      Err(err) => {
        error!("Error reading video bytes for order {}: {:?}", order.order_id, err);
        return alert_pager_and_return_err(
          &deps.pager,
          "Kinovi video download failed",
          anyhow!("error reading video bytes: {:?}", err),
          Some(job),
        );
      }
    },
    Err(err) => {
      error!("Error downloading video for order {}: {:?}", order.order_id, err);
      return alert_pager_and_return_err(
        &deps.pager,
        "Kinovi video download failed",
        anyhow!("reqwest error downloading video: {:?}", err),
        Some(job),
      );
    }
  };

  info!(
    "Downloaded {} bytes for order {}",
    video_bytes.len(),
    order.order_id
  );

  let (suffix, mimetype, media_file_type) = video_metadata(&video_bytes)?;
  let (video_bytes, probe_result) = tokio::task::spawn_blocking(move || {
    let info = ffprobe_get_info_from_bytes(&video_bytes);
    (video_bytes, info)
  }).await?;
  let video_info = probe_result.unwrap_or_else(|err| {
    warn!("Could not probe video for order {}: {:?}", order.order_id, err);
    VideoInfo::default()
  });

  // Hash the video.
  let checksum = sha256_hash_bytes(&video_bytes)
    .map_err(|err| anyhow!("error hashing video: {:?}", err))?;

  // Build the bucket path.
  let bucket_path = MediaFileBucketPath::generate_new(Some(PREFIX), Some(suffix));

  let object_path = bucket_path.get_full_object_path_str();

  info!(
    "Uploading video to public bucket at path: {}",
    object_path
  );

  // Upload to public bucket.
  let upload_result = deps
    .public_bucket_client
    .upload_file_with_content_type_process(object_path, &video_bytes, mimetype)
    .await;

  if let Err(err) = upload_result {
    error!("Error uploading video for order {}: {:?}", order.order_id, err);
    return alert_pager_and_return_err(
      &deps.pager,
      "Kinovi bucket upload failed",
      anyhow!("error uploading video to bucket: {:?}", err),
      Some(job),
    );
  }

  info!(
    "Uploaded video for order {}. Creating media file record.",
    order.order_id
  );

  // Prefer the downloaded file's dimensions; provider fields are a fallback.
  let maybe_result = order.results.iter().find(|result| result.url == video_url);
  let maybe_frame_width = video_info.dimensions.as_ref()
    .and_then(|d| u32::try_from(d.width).ok())
    .or_else(|| maybe_result.and_then(|result| result.maybe_width));
  let maybe_frame_height = video_info.dimensions.as_ref()
    .and_then(|d| u32::try_from(d.height).ok())
    .or_else(|| maybe_result.and_then(|result| result.maybe_height));
  let maybe_duration_millis = video_info.duration.map(|duration| u64::from(duration.millis));

  // Insert media file record.
  let media_file_result = MediaFileInsertBuilder::new()
    .checksum_sha2(&checksum)
    .creator_ip_address(&job.creator_ip_address)
    .creator_set_visibility(job.creator_set_visibility)
    .file_size_bytes(video_bytes.len() as u64)
    .maybe_creator_anonymous_visitor(job.maybe_creator_anonymous_visitor_token.as_ref())
    .maybe_creator_user(job.maybe_creator_user_token.as_ref())
    .maybe_duration_millis(maybe_duration_millis)
    .maybe_frame_height(maybe_frame_height)
    .maybe_frame_width(maybe_frame_width)
    .maybe_generation_provider(Some(GenerationProvider::Artcraft))
    .maybe_prompt_token(job.maybe_prompt_token.as_ref())
    .maybe_source_job_token(Some(&job.job_token))
    .maybe_platform_type(job.maybe_platform_type)
    .media_file_class(MediaFileClass::Video)
    .media_file_origin_category(MediaFileOriginCategory::Inference)
    .media_file_origin_product_category(MediaFileOriginProductCategory::VideoGeneration)
    .media_file_type(media_file_type)
    .mime_type(mimetype)
    .public_bucket_directory_hash(&bucket_path)
    .insert_pool(&deps.mysql_pool)
    .await;

  let media_file_token = match media_file_result {
    Ok(token) => token,
    Err(err) => {
      error!("Error inserting media file record for order {}: {:?}", order.order_id, err);
      return alert_pager_and_return_err(
        &deps.pager,
        "Kinovi media file insert failed",
        anyhow!("error inserting media file record: {:?}", err),
        Some(job),
      );
    }
  };

  info!(
    "Created media file {} for order {}. Finalizing job {}.",
    media_file_token.as_str(),
    order.order_id,
    job.job_token.as_str()
  );

  // Finalize inside a transaction: re-check the job is still pending under a row
  // lock, then mark it complete. This guards against a concurrent finalizer
  // (another poll, a web cancel) having already settled the job between our
  // pre-check and now. If it has, the freshly-uploaded media file is left
  // orphaned (a rare, harmless cost).
  finalize_success(deps, job, order, media_file_token.as_str()).await
}

async fn finalize_success(
  deps: &JobDependencies,
  job: &PendingKinoviWebJob,
  order: &OrderStatus,
  media_file_token: &str,
) -> AnyhowResult<()> {
  let mut transaction = deps.mysql_pool.begin().await.map_err(|err| {
    anyhow!("error beginning finalize transaction for job {}: {:?}", job.job_token.as_str(), err)
  })?;

  let maybe_status = select_inference_job_status_for_update(&mut *transaction, &job.job_token)
    .await
    .map_err(|err| anyhow!("error locking job {} for finalize: {:?}", job.job_token.as_str(), err))?;

  // ── Terminal-state guard (do NOT remove) ──
  //
  // Bail unless the job is still pending. A concurrent finalizer (another poll,
  // a web cancel) may have settled it between the processing loop's pre-check
  // and this locked re-read. If we don't stop here we'd re-mark a finished job
  // and leak the just-uploaded media file. This is the single most important
  // check in this function, so it's a discrete, early-returning step.

  let status = match maybe_status {
    Some(status) => status,
    None => {
      let _ = transaction.rollback().await;
      return Err(anyhow!(
        "Job {} vanished before finalize (order {})",
        job.job_token.as_str(), order.order_id,
      ));
    }
  };

  if is_job_status_terminal(status) {
    warn!(
      "Job {} is already terminal ({:?}); skipping mark-done (order {}). \
      Media file {} may be orphaned.",
      job.job_token.as_str(), status, order.order_id, media_file_token,
    );
    let _ = transaction.rollback().await;
    return Ok(());
  }

  // Still pending — mark it done within the locked transaction.

  if let Err(err) = mark_generic_inference_job_successfully_done_by_token_with_executor(
    MarkGenericInferenceJobSuccessfullyDoneByTokenWithExecutorArgs {
      executor: &mut *transaction,
      token: &job.job_token,
      maybe_entity_type: Some(InferenceResultType::MediaFile),
      maybe_entity_token: Some(media_file_token),
      total_job_duration: None,
      inference_duration: None,
    },
  ).await {
    let _ = transaction.rollback().await;
    error!("Error marking job {} done: {:?}", job.job_token.as_str(), err);
    return alert_pager_and_return_err(
      &deps.pager,
      "Kinovi job completion update failed",
      anyhow!("error marking job done: {:?}", err),
      Some(job),
    );
  }

  transaction.commit().await.map_err(|err| {
    anyhow!("error committing finalize transaction for job {}: {:?}", job.job_token.as_str(), err)
  })?;

  info!("Job {} completed successfully.", job.job_token.as_str());

  Ok(())
}

/// Detect the actual container instead of labeling every provider result as MP4.
fn video_metadata(bytes: &[u8]) -> AnyhowResult<(&'static str, &'static str, MediaFileType)> {
  match get_mimetype_for_bytes(bytes) {
    Some("video/quicktime") => Ok((".mov", "video/quicktime", MediaFileType::Mov)),
    Some("video/mp4") => Ok((".mp4", "video/mp4", MediaFileType::Mp4)),
    other => Err(anyhow!("Unsupported Kinovi video result MIME type: {:?}", other)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn detects_quicktime_result() {
    let bytes = b"\x00\x00\x00\x14ftypqt  \x00\x00\x02\x00qt  ";
    assert_eq!(video_metadata(bytes).unwrap(), (".mov", "video/quicktime", MediaFileType::Mov));
  }

  #[test]
  fn detects_mp4_result() {
    let bytes = b"\x00\x00\x00\x20ftypisom\x00\x00\x02\x00isomiso2avc1mp41";
    assert_eq!(video_metadata(bytes).unwrap(), (".mp4", "video/mp4", MediaFileType::Mp4));
  }

  #[test]
  fn rejects_error_page_as_video() {
    assert!(video_metadata(b"<html>Download failed</html>").is_err());
  }
}
