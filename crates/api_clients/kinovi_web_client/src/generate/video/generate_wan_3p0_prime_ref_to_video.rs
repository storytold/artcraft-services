//! Wan 3.0 Prime ref-to-video through the Kinovi consumer workflow endpoint.

use crate::creds::kinovi_web_session::KinoviWebSession;
use crate::error::kinovi_web_error::KinoviWebError;
use crate::generate::video::wan_3p0::{BUSINESS_TYPE, Wan3p0ApiParams, Wan3p0Model};
use crate::pricing::cost::kinovi_fractional_generation_cost::KinoviFractionalGenerationCost;
use crate::pricing::kinovi_cost_calculator_trait::KinoviCostCalculatorTrait;
use crate::pricing::kinovi_pricing_tier::KinoviPricingTier;
use crate::requests::kinovi_host::KinoviHost;
use crate::requests::workflow_run_task::workflow_run_task::{workflow_run_task_custom, WorkflowRunTaskCustomArgs};

pub use crate::generate::video::wan_3p0::{KinoviWan3p0AspectRatio, KinoviWan3p0OutputResolution};
pub use crate::requests::workflow_run_task::workflow_run_task::WorkflowRunTaskResponse as GenerateWan3p0PrimeRefToVideoResponse;

pub struct GenerateWan3p0PrimeRefToVideoArgs<'a> {
  pub request: GenerateWan3p0PrimeRefToVideoRequest,
  pub session: &'a KinoviWebSession,
  pub maybe_host_override: Option<KinoviHost>,
}

#[derive(Clone, Debug)]
pub struct GenerateWan3p0PrimeRefToVideoRequest {
  /// Up to 20,000 characters; may be empty when media is supplied.
  pub prompt: String,
  /// None resolves to 16:9.
  pub maybe_aspect_ratio: Option<KinoviWan3p0AspectRatio>,
  /// None resolves to 720p for both pricing and generation.
  pub maybe_output_resolution: Option<KinoviWan3p0OutputResolution>,
  /// Any whole-second duration from 2 through 30 (not just the UI presets).
  pub duration_seconds: u8,
  /// Optional noise seed, inclusive range 0..=2147483647.
  pub maybe_seed: Option<u32>,
  /// None keeps Kinovi's default generated soundtrack; false disables it.
  pub maybe_generate_audio: Option<bool>,
  /// Up to 10 reference images, addressed as Image 1, Image 2, etc.
  pub maybe_reference_image_urls: Option<Vec<String>>,
  /// Up to 5 videos, each 1-15s, totaling at most 15s.
  pub maybe_reference_video_urls: Option<Vec<String>>,
  /// Up to 5 audio files, each 1-15s, totaling at most 15s.
  /// Actual file durations are validated by Kinovi.
  pub maybe_reference_audio_urls: Option<Vec<String>>,
  /// Optional measured TOTAL reference-video duration, used only for validation.
  /// Reference duration + output duration must not exceed 30s. This metadata
  /// is not sent or billed. Without it, Kinovi validates the actual durations.
  pub maybe_total_reference_video_duration_millis: Option<u32>,
}

impl Default for GenerateWan3p0PrimeRefToVideoRequest {
  fn default() -> Self {
    Self {
      prompt: String::new(),
      maybe_aspect_ratio: None,
      maybe_output_resolution: None,
      duration_seconds: 5,
      maybe_seed: None,
      maybe_generate_audio: None,
      maybe_reference_image_urls: None,
      maybe_reference_video_urls: None,
      maybe_reference_audio_urls: None,
      maybe_total_reference_video_duration_millis: None,
    }
  }
}

impl KinoviCostCalculatorTrait for GenerateWan3p0PrimeRefToVideoRequest {
  type Cost = KinoviFractionalGenerationCost;

  /// Prices output seconds only. Images, video/audio references, seed, and
  /// aspect ratio do not change the credit charge. Both tiers use the same
  /// credit rate and their respective credit-to-USD conversion.
  fn calculate_costs(&self, tier: KinoviPricingTier) -> Self::Cost {
    Wan3p0Model::PrimeRefToVideo.calculate_costs(self.maybe_output_resolution, self.duration_seconds, tier)
  }
}

impl GenerateWan3p0PrimeRefToVideoRequest {
  pub(super) fn build_api_params(self) -> Result<Wan3p0ApiParams, KinoviWebError> {
    let mut params = Wan3p0ApiParams::new(
      Wan3p0Model::PrimeRefToVideo,
      self.prompt,
      self.maybe_aspect_ratio,
      self.maybe_output_resolution,
      self.duration_seconds,
      self.maybe_seed,
      self.maybe_generate_audio,
    )?;
    params.uploaded_urls = self.maybe_reference_image_urls;
    params.video_urls = self.maybe_reference_video_urls;
    params.audio_urls = self.maybe_reference_audio_urls;
    params.validate_media(self.duration_seconds, self.maybe_total_reference_video_duration_millis)?;
    Ok(params)
  }
}

pub async fn generate_wan_3p0_prime_ref_to_video(
  args: GenerateWan3p0PrimeRefToVideoArgs<'_>,
) -> Result<GenerateWan3p0PrimeRefToVideoResponse, KinoviWebError> {
  workflow_run_task_custom(WorkflowRunTaskCustomArgs {
    business_type: BUSINESS_TYPE,
    api_params: args.request.build_api_params()?,
    session: args.session,
    host_override: args.maybe_host_override,
  })
  .await
}
