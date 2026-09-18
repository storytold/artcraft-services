use crate::api::router_video_model::RouterVideoModel;
use crate::errors::artcraft_router_error::ArtcraftRouterError;
use crate::generate::generate_video::generate_video_request_builder::GenerateVideoRequestBuilder;
use crate::generate::generate_video::providers::kinovi::wan_3p0::draft::KinoviWan3p0DraftState;
use crate::generate::generate_video::providers::wan_3p0_common::normalize_wan_3p0_builder;
use crate::generate::generate_video::video_generation_draft::VideoGenerationDraftRequest;
use crate::generate::generate_video::video_generation_draft_or_request::VideoGenerationDraftOrRequest;

pub fn build_kinovi_wan_3p0(builder: GenerateVideoRequestBuilder) -> Result<VideoGenerationDraftOrRequest, ArtcraftRouterError> {
  let builder = normalize_wan_3p0_builder(builder)?;
  let prime = matches!(builder.model, RouterVideoModel::Wan3p0Prime);
  let draft = KinoviWan3p0DraftState { builder };
  Ok(VideoGenerationDraftOrRequest::Draft(if prime {
    VideoGenerationDraftRequest::KinoviWan3p0Prime(draft)
  } else {
    VideoGenerationDraftRequest::KinoviWan3p0(draft)
  }))
}
