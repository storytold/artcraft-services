use artcraft_api_defs::omni_gen::cost_and_generate_requests::omni_gen_video_cost_and_generate_request::OmniGenVideoCostAndGenerateRequest;
use enums::common::generation::common_aspect_ratio::CommonAspectRatio;
use enums::common::generation::common_resolution::CommonResolution;
use enums::common::generation::common_video_model::CommonVideoModel;
use crate::api::router_aspect_ratio::RouterAspectRatio;
use crate::api::router_resolution::RouterResolution;
use crate::api::router_video_model::RouterVideoModel;
use crate::errors::artcraft_router_error::ArtcraftRouterError;
use crate::generate::generate_video::generate_video_request_builder::GenerateVideoRequestBuilder;
use crate::generate::generate_video::providers::artcraft::resolve::{resolve_image_ref, resolve_image_list_ref, resolve_video_list_ref, resolve_audio_list_ref};
use crate::generate::generate_video::providers::artcraft::wan_3p0::request::ArtcraftWan3p0RequestState;
use crate::generate::generate_video::providers::wan_3p0_common::normalize_wan_3p0_builder;
use crate::generate::generate_video::video_generation_draft_or_request::VideoGenerationDraftOrRequest;
use crate::generate::generate_video::video_generation_request::VideoGenerationRequest;

pub fn build_artcraft_wan_3p0(builder: GenerateVideoRequestBuilder) -> Result<VideoGenerationDraftOrRequest, ArtcraftRouterError> {
  let builder = normalize_wan_3p0_builder(builder)?;
  let prime = matches!(builder.model, RouterVideoModel::Wan3p0Prime);
  let aspect = match builder.aspect_ratio.unwrap() {
    RouterAspectRatio::WideSixteenByNine => CommonAspectRatio::WideSixteenByNine,
    RouterAspectRatio::TallNineBySixteen => CommonAspectRatio::TallNineBySixteen,
    RouterAspectRatio::WideFourByThree => CommonAspectRatio::WideFourByThree,
    RouterAspectRatio::TallThreeByFour => CommonAspectRatio::TallThreeByFour,
    RouterAspectRatio::Square => CommonAspectRatio::Square,
    _ => unreachable!("normalized Wan aspect ratio"),
  };
  let resolution = match builder.resolution.unwrap() {
    RouterResolution::FourEightyP => CommonResolution::FourEightyP,
    RouterResolution::SevenTwentyP => CommonResolution::SevenTwentyP,
    RouterResolution::TenEightyP => CommonResolution::TenEightyP,
    _ => unreachable!("normalized Wan resolution"),
  };
  let idempotency_token = builder.get_or_generate_idempotency_token();
  let request = OmniGenVideoCostAndGenerateRequest {
    model: Some(if prime { CommonVideoModel::Wan3p0Prime } else { CommonVideoModel::Wan3p0 }),
    idempotency_token: Some(idempotency_token),
    prompt: builder.prompt,
    start_frame_image_media_token: resolve_image_ref(builder.start_frame)?,
    end_frame_image_media_token: resolve_image_ref(builder.end_frame)?,
    reference_image_media_tokens: resolve_image_list_ref(builder.reference_images)?,
    reference_video_media_tokens: resolve_video_list_ref(builder.reference_videos)?,
    reference_audio_media_tokens: resolve_audio_list_ref(builder.reference_audio)?,
    resolution: Some(resolution),
    aspect_ratio: Some(aspect),
    duration_seconds: builder.duration_seconds,
    video_batch_count: Some(1),
    generate_audio: builder.generate_audio,
    reference_character_tokens: None,
    negative_prompt: None,
    bitrate: None,
    maybe_output_format: None,
    quality: None,
    estimate_only: None,
  };
  let state = ArtcraftWan3p0RequestState { request };
  Ok(VideoGenerationDraftOrRequest::Request(if prime {
    VideoGenerationRequest::ArtcraftWan3p0Prime(state)
  } else {
    VideoGenerationRequest::ArtcraftWan3p0(state)
  }))
}
