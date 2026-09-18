//! Wan 3.0 Prime text-to-video through the Kinovi consumer workflow endpoint.

use crate::creds::kinovi_web_session::KinoviWebSession;
use crate::error::kinovi_web_error::KinoviWebError;
use crate::generate::video::wan_3p0::{BUSINESS_TYPE, Wan3p0ApiParams, Wan3p0Model};
use crate::pricing::cost::kinovi_fractional_generation_cost::KinoviFractionalGenerationCost;
use crate::pricing::kinovi_cost_calculator_trait::KinoviCostCalculatorTrait;
use crate::pricing::kinovi_pricing_tier::KinoviPricingTier;
use crate::requests::kinovi_host::KinoviHost;
use crate::requests::workflow_run_task::workflow_run_task::{workflow_run_task_custom, WorkflowRunTaskCustomArgs};

pub use crate::generate::video::wan_3p0::{KinoviWan3p0AspectRatio, KinoviWan3p0OutputResolution};
pub use crate::requests::workflow_run_task::workflow_run_task::WorkflowRunTaskResponse as GenerateWan3p0PrimeTextToVideoResponse;

pub struct GenerateWan3p0PrimeTextToVideoArgs<'a> {
  pub request: GenerateWan3p0PrimeTextToVideoRequest,
  pub session: &'a KinoviWebSession,
  pub maybe_host_override: Option<KinoviHost>,
}

#[derive(Clone, Debug)]
pub struct GenerateWan3p0PrimeTextToVideoRequest {
  /// Required prompt, up to 20,000 characters.
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
}

impl Default for GenerateWan3p0PrimeTextToVideoRequest {
  fn default() -> Self {
    Self {
      prompt: String::new(),
      maybe_aspect_ratio: None,
      maybe_output_resolution: None,
      duration_seconds: 5,
      maybe_seed: None,
      maybe_generate_audio: None,
    }
  }
}

impl KinoviCostCalculatorTrait for GenerateWan3p0PrimeTextToVideoRequest {
  type Cost = KinoviFractionalGenerationCost;

  /// Prices output seconds only. Images, video/audio references, seed, and
  /// aspect ratio do not change the credit charge. Both tiers use the same
  /// credit rate and their respective credit-to-USD conversion.
  fn calculate_costs(&self, tier: KinoviPricingTier) -> Self::Cost {
    Wan3p0Model::PrimeTextToVideo.calculate_costs(self.maybe_output_resolution, self.duration_seconds, tier)
  }
}

impl GenerateWan3p0PrimeTextToVideoRequest {
  pub(super) fn build_api_params(self) -> Result<Wan3p0ApiParams, KinoviWebError> {
    let mut params = Wan3p0ApiParams::new(
      Wan3p0Model::PrimeTextToVideo,
      self.prompt,
      self.maybe_aspect_ratio,
      self.maybe_output_resolution,
      self.duration_seconds,
      self.maybe_seed,
      self.maybe_generate_audio,
    )?;
    params.validate_media(self.duration_seconds, None)?;
    Ok(params)
  }
}

pub async fn generate_wan_3p0_prime_text_to_video(
  args: GenerateWan3p0PrimeTextToVideoArgs<'_>,
) -> Result<GenerateWan3p0PrimeTextToVideoResponse, KinoviWebError> {
  workflow_run_task_custom(WorkflowRunTaskCustomArgs {
    business_type: BUSINESS_TYPE,
    api_params: args.request.build_api_params()?,
    session: args.session,
    host_override: args.maybe_host_override,
  })
  .await
}
