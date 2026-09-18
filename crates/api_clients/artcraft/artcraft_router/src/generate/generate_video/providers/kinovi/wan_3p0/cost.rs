use kinovi_web_client::pricing::cost::kinovi_fractional_generation_cost::KinoviFractionalGenerationCost;
use crate::generate::generate_video::providers::kinovi::wan_3p0::draft::KinoviWan3p0DraftState;
use crate::generate::generate_video::providers::kinovi::wan_3p0::request::KinoviWan3p0RequestState;
use crate::generate::generate_video::video_generation_cost_estimate::VideoGenerationCostEstimate;

pub struct KinoviWan3p0CostState {
  costs: KinoviFractionalGenerationCost,
}

impl KinoviWan3p0CostState {
  pub fn from_draft(draft: &KinoviWan3p0DraftState) -> Self {
    // Media does not affect Wan pricing. Build a pricing-only request without
    // uploading anything, keeping the planned model, duration and resolution.
    Self::from_request(&draft.with_resolved_media(None, None, None, None, None))
  }

  pub fn from_request(request: &KinoviWan3p0RequestState) -> Self {
    Self { costs: request.request.calculate_enterprise_costs() }
  }

  pub fn estimate_cost(&self) -> VideoGenerationCostEstimate {
    VideoGenerationCostEstimate {
      cost_in_credits: Some(self.costs.kinovi_credits.round() as u64),
      cost_in_usd_cents: Some(self.costs.usd_cents_rounded_up),
      is_free: false, is_unlimited: false, is_rate_limited: false,
      has_watermark: false, failures_are_refunded: None,
    }
  }
}
