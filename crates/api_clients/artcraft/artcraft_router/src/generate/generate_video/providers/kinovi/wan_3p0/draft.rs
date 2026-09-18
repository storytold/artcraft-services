use kinovi_web_client::generate::video::generate_wan_3p0_text_to_video::GenerateWan3p0TextToVideoRequest;
use kinovi_web_client::generate::video::generate_wan_3p0_image_to_video::GenerateWan3p0ImageToVideoRequest;
use kinovi_web_client::generate::video::generate_wan_3p0_ref_to_video::GenerateWan3p0RefToVideoRequest;
use kinovi_web_client::generate::video::generate_wan_3p0_prime_text_to_video::GenerateWan3p0PrimeTextToVideoRequest;
use kinovi_web_client::generate::video::generate_wan_3p0_prime_image_to_video::GenerateWan3p0PrimeImageToVideoRequest;
use kinovi_web_client::generate::video::generate_wan_3p0_prime_ref_to_video::GenerateWan3p0PrimeRefToVideoRequest;
use kinovi_web_client::generate::video::wan_3p0::{KinoviWan3p0AspectRatio, KinoviWan3p0OutputResolution};
use crate::api::router_aspect_ratio::RouterAspectRatio;
use crate::api::router_resolution::RouterResolution;
use crate::api::router_video_model::RouterVideoModel;
use crate::errors::artcraft_router_error::ArtcraftRouterError;
use crate::generate::generate_video::generate_video_request_builder::GenerateVideoRequestBuilder;
use crate::generate::generate_video::providers::kinovi::resolve::{audio_list_ref_into_urls_or_tokens, image_list_ref_into_urls_or_tokens, video_list_ref_into_urls_or_tokens, resolve_and_upload_list, resolve_and_upload_single};
use crate::generate::generate_video::providers::kinovi::wan_3p0::request::{KinoviWan3p0Request, KinoviWan3p0RequestState};
use crate::generate::generate_video::providers::wan_3p0_common::{modality, Wan3p0Modality};
use crate::generate::generate_video::video_generation_draft_context::VideoGenerationDraftContext;

#[derive(Clone, Debug)]
pub struct KinoviWan3p0DraftState {
  /// Settings are normalized before this draft is constructed.
  pub(crate) builder: GenerateVideoRequestBuilder,
}

impl KinoviWan3p0DraftState {
  pub async fn to_request(&self, context: &VideoGenerationDraftContext<'_>) -> Result<KinoviWan3p0RequestState, ArtcraftRouterError> {
    let client = context.get_kinovi_web_client_ref()?;
    let session = &client.session;
    let map = context.media_file_to_artcraft_url_map;
    let paths = context.predownloaded_media_paths;
    let builder = &self.builder;
    let start = resolve_and_upload_single(session, builder.start_frame.clone(), map, paths).await?;
    let end = resolve_and_upload_single(session, builder.end_frame.clone(), map, paths).await?;
    let images = resolve_and_upload_list(session, builder.reference_images.clone().map(image_list_ref_into_urls_or_tokens), map, paths).await?;
    let videos = resolve_and_upload_list(session, builder.reference_videos.clone().map(video_list_ref_into_urls_or_tokens), map, paths).await?;
    let audio = resolve_and_upload_list(session, builder.reference_audio.clone().map(audio_list_ref_into_urls_or_tokens), map, paths).await?;
    Ok(self.with_resolved_media(start, end, images, videos, audio))
  }

  pub(crate) fn with_resolved_media(
    &self,
    maybe_start: Option<String>,
    maybe_end: Option<String>,
    maybe_images: Option<Vec<String>>,
    maybe_videos: Option<Vec<String>>,
    maybe_audio: Option<Vec<String>>,
  ) -> KinoviWan3p0RequestState {
    let builder = &self.builder;
    let aspect = match builder.aspect_ratio.unwrap() {
      RouterAspectRatio::WideSixteenByNine => KinoviWan3p0AspectRatio::Landscape16x9,
      RouterAspectRatio::TallNineBySixteen => KinoviWan3p0AspectRatio::Portrait9x16,
      RouterAspectRatio::WideFourByThree => KinoviWan3p0AspectRatio::Landscape4x3,
      RouterAspectRatio::TallThreeByFour => KinoviWan3p0AspectRatio::Portrait3x4,
      RouterAspectRatio::Square => KinoviWan3p0AspectRatio::Square1x1,
      _ => unreachable!("normalized Wan aspect ratio"),
    };
    let resolution = match builder.resolution.unwrap() {
      RouterResolution::FourEightyP => KinoviWan3p0OutputResolution::FourEightyP,
      RouterResolution::SevenTwentyP => KinoviWan3p0OutputResolution::SevenTwentyP,
      RouterResolution::TenEightyP => KinoviWan3p0OutputResolution::TenEightyP,
      _ => unreachable!("normalized Wan resolution"),
    };
    let request = match (matches!(builder.model, RouterVideoModel::Wan3p0Prime), modality(builder)) {
      (false, Wan3p0Modality::Text) => KinoviWan3p0Request::Text(GenerateWan3p0TextToVideoRequest {
        prompt: builder.prompt.clone().unwrap_or_default(),
        maybe_aspect_ratio: Some(aspect),
        maybe_output_resolution: Some(resolution),
        duration_seconds: builder.duration_seconds.unwrap() as u8,
        maybe_generate_audio: builder.generate_audio,
        ..Default::default()
      }),
      (false, Wan3p0Modality::Image) => KinoviWan3p0Request::Image(GenerateWan3p0ImageToVideoRequest {
        prompt: builder.prompt.clone().unwrap_or_default(),
        maybe_aspect_ratio: Some(aspect),
        maybe_output_resolution: Some(resolution),
        duration_seconds: builder.duration_seconds.unwrap() as u8,
        maybe_generate_audio: builder.generate_audio,
        maybe_start_frame_url: maybe_start,
        maybe_end_frame_url: maybe_end,
        ..Default::default()
      }),
      (false, Wan3p0Modality::Reference) => KinoviWan3p0Request::Reference(GenerateWan3p0RefToVideoRequest {
        prompt: builder.prompt.clone().unwrap_or_default(),
        maybe_aspect_ratio: Some(aspect),
        maybe_output_resolution: Some(resolution),
        duration_seconds: builder.duration_seconds.unwrap() as u8,
        maybe_generate_audio: builder.generate_audio,
        maybe_reference_image_urls: maybe_images,
        maybe_reference_video_urls: maybe_videos,
        maybe_reference_audio_urls: maybe_audio,
        maybe_total_reference_video_duration_millis: builder.total_reference_video_input_seconds.map(|seconds| u32::from(seconds) * 1000),
        ..Default::default()
      }),
      (true, Wan3p0Modality::Text) => KinoviWan3p0Request::PrimeText(GenerateWan3p0PrimeTextToVideoRequest {
        prompt: builder.prompt.clone().unwrap_or_default(),
        maybe_aspect_ratio: Some(aspect),
        maybe_output_resolution: Some(resolution),
        duration_seconds: builder.duration_seconds.unwrap() as u8,
        maybe_generate_audio: builder.generate_audio,
        ..Default::default()
      }),
      (true, Wan3p0Modality::Image) => KinoviWan3p0Request::PrimeImage(GenerateWan3p0PrimeImageToVideoRequest {
        prompt: builder.prompt.clone().unwrap_or_default(),
        maybe_aspect_ratio: Some(aspect),
        maybe_output_resolution: Some(resolution),
        duration_seconds: builder.duration_seconds.unwrap() as u8,
        maybe_generate_audio: builder.generate_audio,
        maybe_start_frame_url: maybe_start,
        maybe_end_frame_url: maybe_end,
        ..Default::default()
      }),
      (true, Wan3p0Modality::Reference) => KinoviWan3p0Request::PrimeReference(GenerateWan3p0PrimeRefToVideoRequest {
        prompt: builder.prompt.clone().unwrap_or_default(),
        maybe_aspect_ratio: Some(aspect),
        maybe_output_resolution: Some(resolution),
        duration_seconds: builder.duration_seconds.unwrap() as u8,
        maybe_generate_audio: builder.generate_audio,
        maybe_reference_image_urls: maybe_images,
        maybe_reference_video_urls: maybe_videos,
        maybe_reference_audio_urls: maybe_audio,
        maybe_total_reference_video_duration_millis: builder.total_reference_video_input_seconds.map(|seconds| u32::from(seconds) * 1000),
        ..Default::default()
      }),
    };
    KinoviWan3p0RequestState { request }
  }
}
