use std::process::Command;

use actix_web::{test, web, App};
use enums::by_table::media_files::media_file_class::MediaFileClass;
use enums::by_table::media_files::media_file_type::MediaFileType;
use enums::common::generation::common_resolution::CommonResolution;
use enums::common::generation::common_video_model::CommonVideoModel;
use enums::common::generation::common_video_output_format::CommonVideoOutputFormat;
use mysql_queries::queries::media_files::get::get_media_file::get_media_file;
use tokens::tokens::media_files::MediaFileToken;

use crate::http_server::endpoints::media_files::upload::upload_generic::upload_media_file_handler::upload_media_file_handler;
use crate::http_server::endpoints::media_files::upload::upload_video_new::upload_new_video_media_file_handler::upload_new_video_media_file_handler;
use crate::http_server::endpoints::omni_api::upload::omni_upload_video_media_file_handler::omni_upload_video_media_file_handler;
use crate::http_server::endpoints::omni_gen::generate::video::tests::support::{base_generate_request, stub_traffic, TestHarness};

const BOUNDARY: &str = "artcraft-quicktime-test-boundary";

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn quicktime_uploads_are_video_references_with_mp4_or_mov_output() {
  let harness = TestHarness::create().await;
  let user = harness.create_funded_user(10_000).await;
  let api_key = harness.create_api_key(&user).await;
  let bytes = quicktime_fixture();
  let app = test::init_service(App::new()
    .app_data(web::Data::new(harness.server_state.clone()))
    .route("/new-video", web::post().to(upload_new_video_media_file_handler))
    .route("/omni-upload", web::post().to(omni_upload_video_media_file_handler))
    .route("/generic-upload", web::post().to(upload_media_file_handler))
  ).await;

  for route in ["/new-video", "/omni-upload", "/generic-upload"] {
    let idempotency = base_generate_request(CommonVideoModel::Seedance2p5).idempotency_token.unwrap();
    let body = multipart_body(&idempotency, &bytes);
    let request = test::TestRequest::post().uri(route)
      .insert_header(("Authorization", format!("Bearer {}", api_key.as_str_be_careful())))
      .insert_header(("Content-Type", format!("multipart/form-data; boundary={BOUNDARY}")))
      .peer_addr("127.0.0.1:9999".parse().unwrap())
      .set_payload(body).to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = test::read_body(response).await;
    assert!(status.is_success(), "{}: {}", route, String::from_utf8_lossy(&body));
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token = MediaFileToken::new_from_str(json["media_file_token"].as_str().unwrap());
    let media = get_media_file(&token, false, &harness.pool).await.unwrap().unwrap();
    assert_eq!(media.media_class, MediaFileClass::Video);
    assert_eq!(media.media_type, MediaFileType::Mov);
    assert_eq!(media.maybe_mime_type.as_deref(), Some("video/quicktime"));
    assert_eq!(media.maybe_public_bucket_extension.as_deref(), Some(".mov"));
    assert_eq!(media.maybe_duration_millis, Some(1000));

    for (format, expected) in [(None, "mp4"), (Some(CommonVideoOutputFormat::Mp4), "mp4"), (Some(CommonVideoOutputFormat::Mov), "mov")] {
      let mut request = base_generate_request(CommonVideoModel::Seedance2p5);
      let prompt = format!("QuickTime reference test {}", request.idempotency_token.as_deref().unwrap());
      request.prompt = Some(prompt.clone());
      request.reference_video_media_tokens = Some(vec![token.clone()]);
      request.maybe_output_format = format;
      request.generate_audio = Some(true);
      request.duration_seconds = Some(4);
      request.resolution = Some(CommonResolution::FourEightyP);
      let response = harness.post_generate(&user, request).await.expect("submit MOV video reference");
      assert!(response.success);

      let traffic = stub_traffic().lock().unwrap();
      let params = traffic.workflows.iter()
        .map(|request| &request["0"]["json"]["apiParams"])
        .find(|params| params["prompt"].as_str() == Some(&prompt))
        .expect("captured Kinovi request");
      assert_eq!(params["output_format"], expected);
      assert_eq!(params["generate_audio"], true);
      assert_eq!(params["mode"], "reference");
      assert!(params["videoUrls"][0].as_str().is_some_and(|url| url.ends_with(".mov")), "{}", params);
      assert_eq!(traffic.uploads.get("fixture.mov").unwrap(), &bytes);
    }
  }
}

fn quicktime_fixture() -> Vec<u8> {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("reference.mov");
  let output = Command::new("ffmpeg").args([
    "-hide_banner", "-loglevel", "error", "-nostdin",
    "-f", "lavfi", "-i", "color=c=blue:s=160x90:r=24",
    "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000",
    "-t", "1", "-c:v", "libx264", "-pix_fmt", "yuv420p",
    "-c:a", "pcm_s16le", "-ac", "2",
  ]).arg(&path).output().unwrap();
  assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
  std::fs::read(path).unwrap()
}

fn multipart_body(idempotency_token: &str, bytes: &[u8]) -> Vec<u8> {
  let mut body = format!(
    "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"uuid_idempotency_token\"\r\n\r\n{idempotency_token}\r\n\
     --{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"reference.mov\"\r\nContent-Type: video/quicktime\r\n\r\n"
  ).into_bytes();
  body.extend_from_slice(bytes);
  body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
  body
}
