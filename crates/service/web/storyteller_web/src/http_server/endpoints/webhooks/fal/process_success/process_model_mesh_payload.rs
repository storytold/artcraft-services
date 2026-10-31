use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::webhooks::fal::process_success::attach_cover_image::{attach_cover_image, AttachCoverImageArgs};
use crate::http_server::endpoints::webhooks::fal::process_success::attach_prompt_imageref_cover::try_to_attach_prompt_imageref_cover;
use crate::http_server::endpoints::webhooks::fal::process_success::resolve_mesh_file_metadata::resolve_mesh_file_metadata;
use crate::state::server_state::ServerState;
use crate::util::http_download_url_to_bytes::http_download_url_to_bytes;
use bytes::Bytes;
use bucket_paths::legacy::typified_paths::public::media_files::bucket_file_path::MediaFileBucketPath;
use enums::by_table::media_files::media_file_class::MediaFileClass;
use enums::by_table::media_files::media_file_engine_category::MediaFileEngineCategory;
use enums::by_table::media_files::media_file_origin_category::MediaFileOriginCategory;
use enums::by_table::media_files::media_file_origin_product_category::MediaFileOriginProductCategory;
use enums::by_table::media_files::media_file_type::MediaFileType;
use fal_client::webhook_payload::hydrated::hydrated_webhook_contents::{ModelMeshData, ModelObjData, PreprocessedImageData};
use hashing::sha256::sha256_hash_bytes::sha256_hash_bytes;
use log::{info, warn};
use enums::common::generation_provider::GenerationProvider;
use mysql_queries::queries::generic_inference::api_providers::fal::get_inference_job_by_fal_id::FalJobDetails;
use mysql_queries::queries::media_files::create::insert_builder::media_file_insert_builder::MediaFileInsertBuilder;
use tokens::tokens::batch_generations::BatchGenerationToken;
use tokens::tokens::media_files::MediaFileToken;

const PREFIX : Option<&str> = Some("artcraft_");

// NB: triposplat ply gaussian splat files also arrive via this payload handler.
//  These are decidedly *not* "mesh" files!
pub async fn process_model_mesh_payload(
  model_mesh_data: &ModelMeshData,
  maybe_preprocessed_image: Option<&PreprocessedImageData>,
  job: &FalJobDetails,
  server_state: &ServerState,
) -> Result<MediaFileToken, CommonWebError> {
  let mesh_url = model_mesh_data.url
      .as_deref()
      .ok_or_else(|| {
        warn!("No `url` in model mesh payload");
        CommonWebError::server_error_with_message("no `url` in model mesh payload")
      })?;

  let media_token = upload_mesh_file(UploadMeshFileArgs {
    mesh_url,
    maybe_content_type: model_mesh_data.content_type.as_deref(),
    maybe_file_name: model_mesh_data.file_name.as_deref(),
    maybe_batch_token: None,
    job,
    server_state,
  }).await?;

  // TripoSplat sends the segmented input image alongside the splat; attach it
  // as the cover image. NB: Fail open — the result is usable without a cover.
  if let Some(preprocessed_image) = maybe_preprocessed_image {
    let result = try_to_attach_preprocessed_image(
      preprocessed_image,
      job,
      server_state,
      &media_token,
    ).await;

    if let Err(err) = result {
      warn!("Could not attach preprocessed image as cover image to media file: {:?}", err);
    }
  } else {
    // No image in the payload: fall back to the prompt's imageref as the
    // cover image.
    try_to_attach_prompt_imageref_cover(job, server_state, &media_token).await;
  }

  Ok(media_token)
}

/// `model_obj` payloads (e.g. Hunyuan 3D v3.1 Rapid's OBJ output) have the
/// same file shape as `model_mesh` and get the same handling.
pub async fn process_model_obj_payload(
  model_obj_data: &ModelObjData,
  job: &FalJobDetails,
  server_state: &ServerState,
) -> Result<MediaFileToken, CommonWebError> {
  let mesh_url = model_obj_data.url
      .as_deref()
      .ok_or_else(|| {
        warn!("No `url` in model obj payload");
        CommonWebError::server_error_with_message("no `url` in model obj payload")
      })?;

  let media_token = upload_mesh_file(UploadMeshFileArgs {
    mesh_url,
    maybe_content_type: model_obj_data.content_type.as_deref(),
    maybe_file_name: model_obj_data.file_name.as_deref(),
    maybe_batch_token: None,
    job,
    server_state,
  }).await?;

  // `model_obj` payloads never carry a thumbnail: fall back to the prompt's
  // imageref as the cover image.
  try_to_attach_prompt_imageref_cover(job, server_state, &media_token).await;

  Ok(media_token)
}

pub(crate) struct UploadMeshFileArgs<'a> {
  pub mesh_url: &'a str,
  pub maybe_content_type: Option<&'a str>,
  /// Used as a last-resort extension hint when neither magic bytes nor the
  /// fal content_type identify the file (common for FBX/OBJ meshes).
  pub maybe_file_name: Option<&'a str>,
  /// Set when the mesh is one of several files from a single generation
  /// (e.g. part splitting).
  pub maybe_batch_token: Option<&'a BatchGenerationToken>,
  pub job: &'a FalJobDetails,
  pub server_state: &'a ServerState,
}

/// Download a mesh file from fal, upload it to our public bucket, and insert
/// a media file record. Shared by the single-mesh payload handlers
/// (`model_mesh`, `model_obj`) and the multi-file `result_files` handler.
pub(crate) async fn upload_mesh_file(
  args: UploadMeshFileArgs<'_>,
) -> Result<MediaFileToken, CommonWebError> {
  let UploadMeshFileArgs {
    mesh_url,
    maybe_content_type,
    maybe_file_name,
    maybe_batch_token,
    job,
    server_state,
  } = args;

  // Download with a retry if the first attempt returns suspiciously few bytes.
  let mut file_bytes = http_download_url_to_bytes(mesh_url)
      .await
      .map_err(|err| {
        warn!("Failed to download mesh from {}: {:?}", mesh_url, err);
        CommonWebError::from_error(err)
      })?;

  if file_bytes.len() <= 10 {
    warn!(
      "Downloaded only {} bytes from {} — retrying once",
      file_bytes.len(),
      mesh_url,
    );
    file_bytes = http_download_url_to_bytes(mesh_url)
        .await
        .map_err(|err| {
          warn!("Failed to download mesh on retry from {}: {:?}", mesh_url, err);
          CommonWebError::from_error(err)
        })?;
  }

  // Resolve mesh metadata. A validated OBJ ZIP package is unpacked to its OBJ.
  // Other files retain the magic bytes / fal content_type / file name order.
  let mut metadata = resolve_mesh_file_metadata(&file_bytes, maybe_content_type, maybe_file_name)
      .ok_or_else(|| {
        warn!(
          "Could not determine file type for mesh (bytes: {}, fal content_type: {:?}, file_name: {:?})",
          file_bytes.len(),
          maybe_content_type,
          maybe_file_name,
        );
        CommonWebError::server_error_with_message(
          &format!("Could not determine file type for mesh (bytes: {}, fal content_type: {:?})",
            file_bytes.len(), maybe_content_type))
      })?;

  // For a ZIP package, store the extracted raw OBJ (size, hash, and upload
  // below all use these bytes) rather than the downloaded archive.
  let file_bytes = match metadata.replacement_file_bytes.take() {
    Some(extracted) => Bytes::from(extracted),
    None => file_bytes,
  };

  let mime_type = metadata.mime_type.as_str();

  info!("Mime type of mesh: {}, source: {:?}", mime_type, metadata.source);

  let media_file_type = metadata.media_file_type;
  let extension_with_period = metadata.extension_with_period;

  let file_size_bytes = file_bytes.len();
  let file_hash = sha256_hash_bytes(&file_bytes)
      .map_err(|err| {
        warn!("Failed to hash mesh bytes: {:?}", err);
        CommonWebError::from_anyhow_error(err)
      })?;

  // PLY results are Gaussian splats (e.g. TripoSplat), not meshes: store
  // them the way the World Labs splat pipeline does — `Splat` media class,
  // tagged as world generation, with no `Object` engine category. No fal
  // mesh endpoint returns PLY, so the file type is a reliable discriminator.
  let is_gaussian_splat = matches!(media_file_type, MediaFileType::Ply);

  let public_upload_path = MediaFileBucketPath::generate_new(PREFIX, Some(&extension_with_period));

  info!("Uploading media to bucket path: {}", public_upload_path.get_full_object_path_str());

  server_state.public_bucket_client.upload_file_with_content_type_process(
    public_upload_path.get_full_object_path_str(),
    file_bytes.as_ref(),
    &mime_type)
      .await
      .map_err(|err| {
        warn!("Failed to upload mesh to bucket: {:?}", err);
        CommonWebError::from_anyhow_error(err)
      })?;

  let mut insert_builder = MediaFileInsertBuilder::new()
      .checksum_sha2(&file_hash)
      .creator_ip_address(&job.creator_ip_address)
      .file_size_bytes(file_size_bytes as u64)
      .maybe_batch_generation_token(maybe_batch_token)
      .maybe_creator_anonymous_visitor(job.maybe_creator_anonymous_visitor_token.as_ref())
      .maybe_creator_user(job.maybe_creator_user_token.as_ref())
      .maybe_generation_provider(Some(GenerationProvider::Artcraft))
      .maybe_prompt_token(job.maybe_prompt_token.as_ref())
      .maybe_source_job_token(Some(&job.job_token))
      .maybe_platform_type(job.maybe_platform_type)
      .media_file_origin_category(MediaFileOriginCategory::Inference)
      .media_file_type(media_file_type)
      .mime_type(mime_type)
      .public_bucket_directory_hash(&public_upload_path);

  if is_gaussian_splat {
    insert_builder = insert_builder
        .media_file_class(MediaFileClass::Splat)
        .media_file_origin_product_category(MediaFileOriginProductCategory::WorldGeneration);
  } else {
    insert_builder = insert_builder
        .media_file_class(MediaFileClass::Mesh)
        .maybe_engine_category(Some(MediaFileEngineCategory::Object));
  }

  let media_token = insert_builder
      .insert_pool(&server_state.mysql_pool)
      .await
      .map_err(|err| {
        warn!("Failed to insert mesh media file record: {:?}", err);
        CommonWebError::from_error(err)
      })?;

  info!("Mesh media file uploaded with token: {}", media_token);

  Ok(media_token)
}

/// Attach a TripoSplat `preprocessed_image` (the segmented input image) as
/// the cover image of the uploaded splat, mirroring how `model_glb` results
/// attach their `thumbnail`.
async fn try_to_attach_preprocessed_image(
  preprocessed_image: &PreprocessedImageData,
  job: &FalJobDetails,
  server_state: &ServerState,
  splat_media_token: &MediaFileToken,
) -> Result<(), CommonWebError> {
  info!("Fal Preprocessed Image Data: {:?}", preprocessed_image);

  let image_url = preprocessed_image.url
      .as_deref()
      .ok_or_else(|| {
        warn!("No `url` in preprocessed image payload");
        CommonWebError::server_error_with_message("no `url` in preprocessed image payload")
      })?;

  attach_cover_image(AttachCoverImageArgs {
    image_url,
    maybe_content_type: preprocessed_image.content_type.as_deref(),
    maybe_origin_product_category: Some(MediaFileOriginProductCategory::WorldGeneration),
    target_media_token: splat_media_token,
    job,
    server_state,
  }).await
}


// This test exercises the complete download -> ZIP extraction -> bucket write ->
// media_files row path. It uses the guarded test database and the existing
// in-process stub bucket from TestHarness; no real provider or storage is called.
#[cfg(all(test, not(feature = "skip_database_tests")))]
mod uploader_integration_tests {
  use std::io::{Cursor, Read, Write};
  use std::marker::PhantomData;
  use std::net::TcpListener;
  use std::sync::atomic::{AtomicU64, Ordering};
  use std::thread;
  use std::time::{SystemTime, UNIX_EPOCH};

  use bucket_paths::legacy::typified_paths::public::media_files::bucket_file_path::MediaFileBucketPath;
  use enums::by_table::media_files::media_file_class::MediaFileClass;
  use enums::by_table::media_files::media_file_type::MediaFileType;
  use enums::common::visibility::Visibility;
  use hashing::sha256::sha256_hash_bytes::sha256_hash_bytes;
  use mysql_queries::queries::generic_inference::api_providers::fal::get_inference_job_by_fal_id::get_inference_job_by_fal_id;
  use mysql_queries::queries::generic_inference::api_providers::fal::insert_generic_inference_job_for_fal_queue::{
    insert_generic_inference_job_for_fal_queue, FalCategory, InsertGenericInferenceForFalArgs,
  };
  use mysql_queries::queries::media_files::get::get_media_file::get_media_file;
  use tokens::tokens::generic_inference_jobs::InferenceJobToken;
  use zip::write::FileOptions;
  use zip::ZipWriter;

  use super::{upload_mesh_file, UploadMeshFileArgs};
  use crate::http_server::endpoints::omni_gen::generate::video::tests::support::TestHarness;

  const OBJ_BYTES: &[u8] = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

  #[tokio::test]
  async fn zip_obj_upload_stores_raw_obj_and_consistent_metadata() {
    let harness = TestHarness::create().await;

    // Insert a real Fal job so the uploaded media row uses a valid source job.
    let generated_job_token = InferenceJobToken::generate();
    let external_job_id = format!("mesh-upload-e2e-{}", generated_job_token.as_str());
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("system clock")
      .as_nanos() as u64;
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let idempotency_token = format!("{:016x}{:016x}", now, NEXT_ID.fetch_add(1, Ordering::Relaxed));

    let mut connection = harness.pool.acquire().await.expect("acquire test database connection");
    insert_generic_inference_job_for_fal_queue(InsertGenericInferenceForFalArgs {
      uuid_idempotency_token: &idempotency_token,
      maybe_external_third_party_id: &external_job_id,
      fal_category: FalCategory::ObjectGeneration,
      maybe_model_type: None,
      maybe_inference_args: None,
      maybe_prompt_token: None,
      maybe_creator_user_token: None,
      maybe_avt_token: None,
      creator_ip_address: "127.0.0.1",
      creator_set_visibility: Visibility::Public,
      maybe_platform_type: None,
      mysql_executor: &mut *connection,
      phantom: PhantomData,
    })
    .await
    .expect("insert test Fal job");
    drop(connection);

    let job = get_inference_job_by_fal_id(&external_job_id, &harness.pool)
      .await
      .expect("read test Fal job")
      .expect("test Fal job exists");

    let zip_bytes = make_obj_package();
    let source_url = serve_bytes_once(zip_bytes.clone());

    // Deliberately supply misleading non-ZIP hints: ZIP magic must win.
    let media_token = upload_mesh_file(UploadMeshFileArgs {
      mesh_url: &source_url,
      maybe_content_type: Some("model/obj"),
      maybe_file_name: Some("model.obj"),
      maybe_batch_token: None,
      job: &job,
      server_state: &harness.server_state,
    })
    .await
    .unwrap_or_else(|err| panic!("mesh upload failed: {:?}", err));

    let media = get_media_file(&media_token, true, &harness.pool)
      .await
      .expect("query uploaded media row")
      .expect("uploaded media row exists");

    assert_eq!(media.media_type, MediaFileType::Obj);
    assert_eq!(media.media_class, MediaFileClass::Mesh);
    assert_eq!(media.maybe_mime_type.as_deref(), Some("model/obj"));
    assert_eq!(media.maybe_public_bucket_extension.as_deref(), Some(".obj"));

    let (file_size_bytes, checksum_sha2): (i32, String) =
      sqlx::query_as("SELECT file_size_bytes, checksum_sha2 FROM media_files WHERE token = ?")
        .bind(media_token.as_str())
        .fetch_one(&harness.pool)
        .await
        .expect("read stored size and checksum");

    assert_eq!(file_size_bytes, OBJ_BYTES.len() as i32);
    assert_eq!(checksum_sha2, sha256_hash_bytes(OBJ_BYTES).expect("hash OBJ bytes"));

    // Read the uploaded object back through the actual LegacyBucketClient.
    let object_path = MediaFileBucketPath::from_object_hash(
      &media.public_bucket_directory_hash,
      media.maybe_public_bucket_prefix.as_deref(),
      media.maybe_public_bucket_extension.as_deref(),
    );
    let stored_bytes = harness
      .server_state
      .public_bucket_client
      .download_file(object_path.get_full_object_path_str())
      .await
      .expect("download uploaded mesh from the test bucket");

    assert_eq!(stored_bytes, OBJ_BYTES, "bucket must contain raw OBJ, not ZIP bytes");
    assert_ne!(stored_bytes, zip_bytes);
  }

  fn make_obj_package() -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
      .start_file("model.obj", FileOptions::default())
      .expect("start OBJ entry");
    writer.write_all(OBJ_BYTES).expect("write OBJ entry");
    writer.finish().expect("finish ZIP package").into_inner()
  }

  fn serve_bytes_once(body: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test HTTP server");
    let address = listener.local_addr().expect("local test HTTP server address");

    thread::spawn(move || {
      let (mut stream, _) = listener.accept().expect("accept mesh download");
      let mut request = [0_u8; 2048];
      let _ = stream.read(&mut request);
      let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len(),
      );
      stream.write_all(headers.as_bytes()).expect("write HTTP headers");
      stream.write_all(&body).expect("write ZIP response body");
    });

    format!("http://{address}/model.zip")
  }
}
