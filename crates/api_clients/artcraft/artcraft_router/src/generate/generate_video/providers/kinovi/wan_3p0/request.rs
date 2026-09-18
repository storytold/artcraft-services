use kinovi_web_client::generate::video::generate_wan_3p0_text_to_video::{generate_wan_3p0_text_to_video, GenerateWan3p0TextToVideoArgs, GenerateWan3p0TextToVideoRequest};
use kinovi_web_client::generate::video::generate_wan_3p0_image_to_video::{generate_wan_3p0_image_to_video, GenerateWan3p0ImageToVideoArgs, GenerateWan3p0ImageToVideoRequest};
use kinovi_web_client::generate::video::generate_wan_3p0_ref_to_video::{generate_wan_3p0_ref_to_video, GenerateWan3p0RefToVideoArgs, GenerateWan3p0RefToVideoRequest};
use kinovi_web_client::generate::video::generate_wan_3p0_prime_text_to_video::{generate_wan_3p0_prime_text_to_video, GenerateWan3p0PrimeTextToVideoArgs, GenerateWan3p0PrimeTextToVideoRequest};
use kinovi_web_client::generate::video::generate_wan_3p0_prime_image_to_video::{generate_wan_3p0_prime_image_to_video, GenerateWan3p0PrimeImageToVideoArgs, GenerateWan3p0PrimeImageToVideoRequest};
use kinovi_web_client::generate::video::generate_wan_3p0_prime_ref_to_video::{generate_wan_3p0_prime_ref_to_video, GenerateWan3p0PrimeRefToVideoArgs, GenerateWan3p0PrimeRefToVideoRequest};
use kinovi_web_client::pricing::cost::kinovi_fractional_generation_cost::KinoviFractionalGenerationCost;
use kinovi_web_client::pricing::kinovi_cost_calculator_trait::KinoviCostCalculatorTrait;
use crate::client::router_kinovi_web_client::RouterKinoviWebClient;
use crate::errors::artcraft_router_error::ArtcraftRouterError;
use crate::errors::provider_error::ProviderError;
use crate::generate::generate_video::generate_video_response::{GenerateVideoResponse, KinoviWebVideoResponsePayload};

#[derive(Clone, Debug)]
pub struct KinoviWan3p0RequestState {
  pub request: KinoviWan3p0Request,
}

/// Provider bindings selected internally from the two public Wan models.
#[derive(Clone, Debug)]
pub enum KinoviWan3p0Request {
  Text(GenerateWan3p0TextToVideoRequest),
  Image(GenerateWan3p0ImageToVideoRequest),
  Reference(GenerateWan3p0RefToVideoRequest),
  PrimeText(GenerateWan3p0PrimeTextToVideoRequest),
  PrimeImage(GenerateWan3p0PrimeImageToVideoRequest),
  PrimeReference(GenerateWan3p0PrimeRefToVideoRequest),
}

impl KinoviWan3p0RequestState {
  pub async fn send(&self, client: &RouterKinoviWebClient) -> Result<GenerateVideoResponse, ArtcraftRouterError> {
    let session = &client.session;
    let response = match &self.request {
      KinoviWan3p0Request::Text(request) => generate_wan_3p0_text_to_video(GenerateWan3p0TextToVideoArgs { request: request.clone(), session, maybe_host_override: None }).await,
      KinoviWan3p0Request::Image(request) => generate_wan_3p0_image_to_video(GenerateWan3p0ImageToVideoArgs { request: request.clone(), session, maybe_host_override: None }).await,
      KinoviWan3p0Request::Reference(request) => generate_wan_3p0_ref_to_video(GenerateWan3p0RefToVideoArgs { request: request.clone(), session, maybe_host_override: None }).await,
      KinoviWan3p0Request::PrimeText(request) => generate_wan_3p0_prime_text_to_video(GenerateWan3p0PrimeTextToVideoArgs { request: request.clone(), session, maybe_host_override: None }).await,
      KinoviWan3p0Request::PrimeImage(request) => generate_wan_3p0_prime_image_to_video(GenerateWan3p0PrimeImageToVideoArgs { request: request.clone(), session, maybe_host_override: None }).await,
      KinoviWan3p0Request::PrimeReference(request) => generate_wan_3p0_prime_ref_to_video(GenerateWan3p0PrimeRefToVideoArgs { request: request.clone(), session, maybe_host_override: None }).await,
    }.map_err(|err| ArtcraftRouterError::Provider(ProviderError::KinoviWeb(err)))?;
    Ok(GenerateVideoResponse::KinoviWeb(KinoviWebVideoResponsePayload {
      order_id: response.order_id,
      task_id: response.task_id,
      maybe_order_ids: response.order_ids,
      maybe_task_ids: response.task_ids,
    }))
  }
}

impl KinoviWan3p0Request {
  pub fn calculate_enterprise_costs(&self) -> KinoviFractionalGenerationCost {
    match self {
      Self::Text(request) => request.calculate_enterprise_costs(),
      Self::Image(request) => request.calculate_enterprise_costs(),
      Self::Reference(request) => request.calculate_enterprise_costs(),
      Self::PrimeText(request) => request.calculate_enterprise_costs(),
      Self::PrimeImage(request) => request.calculate_enterprise_costs(),
      Self::PrimeReference(request) => request.calculate_enterprise_costs(),
    }
  }
}
