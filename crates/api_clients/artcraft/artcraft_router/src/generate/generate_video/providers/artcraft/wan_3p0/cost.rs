use enums::common::generation::common_resolution::CommonResolution;
use enums::common::generation::common_video_model::CommonVideoModel;
use crate::generate::generate_video::providers::artcraft::wan_3p0::request::ArtcraftWan3p0RequestState;
use crate::generate::generate_video::video_generation_cost_estimate::VideoGenerationCostEstimate;

// ArtCraft USD cents per output second, in 480p/720p/1080p order.
// Pricing snapshot: 2026-09-18. Review alongside the Wan provider pricing
// after September 23; see kinovi_web_client/costs_wan_3p0.md.
const WAN_CENTS_PER_SECOND: [f64; 3] = [4.82858056, 9.65716111, 19.30948396];
const PRIME_CENTS_PER_SECOND: [f64; 3] = [9.12495283, 18.78695219, 37.57390438];

pub struct ArtcraftWan3p0CostState {
  pub is_prime: bool,
  pub resolution: CommonResolution,
  pub duration_seconds: u16,
}

impl ArtcraftWan3p0CostState {
  pub fn from_request(state: &ArtcraftWan3p0RequestState) -> Self {
    Self {
      is_prime: matches!(state.request.model, Some(CommonVideoModel::Wan3p0Prime)),
      resolution: state.request.resolution.unwrap_or(CommonResolution::SevenTwentyP),
      duration_seconds: state.request.duration_seconds.unwrap_or(5),
    }
  }

  pub fn estimate_cost(&self) -> VideoGenerationCostEstimate {
    let rates = if self.is_prime { PRIME_CENTS_PER_SECOND } else { WAN_CENTS_PER_SECOND };
    let index = match self.resolution {
      CommonResolution::FourEightyP => 0,
      CommonResolution::TenEightyP => 2,
      _ => 1,
    };
    let cents = (rates[index] * f64::from(self.duration_seconds)).ceil() as u64;
    VideoGenerationCostEstimate {
      cost_in_credits: Some(cents), cost_in_usd_cents: Some(cents),
      is_free: false, is_unlimited: false, is_rate_limited: false,
      has_watermark: false, failures_are_refunded: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn regular_wan_prices_are_fixed_per_resolution_and_duration() {
    let expected = [
      (CommonResolution::FourEightyP, [10, 25, 49]),
      (CommonResolution::SevenTwentyP, [20, 49, 97]),
      (CommonResolution::TenEightyP, [39, 97, 194]),
    ];
    for (resolution, prices) in expected {
      for (duration, price) in [(2, prices[0]), (5, prices[1]), (10, prices[2])] {
        let cost = ArtcraftWan3p0CostState {
          is_prime: false,
          resolution,
          duration_seconds: duration,
        }
        .estimate_cost();
        assert_eq!(cost.cost_in_credits, Some(price));
        assert_eq!(cost.cost_in_usd_cents, Some(price));
      }
    }
  }

  #[test]
  fn prime_wan_prices_are_fixed_per_resolution_and_duration() {
    let expected = [
      (CommonResolution::FourEightyP, [19, 46, 92]),
      (CommonResolution::SevenTwentyP, [38, 94, 188]),
      (CommonResolution::TenEightyP, [76, 188, 376]),
    ];
    for (resolution, prices) in expected {
      for (duration, price) in [(2, prices[0]), (5, prices[1]), (10, prices[2])] {
        let cost = ArtcraftWan3p0CostState {
          is_prime: true,
          resolution,
          duration_seconds: duration,
        }
        .estimate_cost();
        assert_eq!(cost.cost_in_credits, Some(price));
        assert_eq!(cost.cost_in_usd_cents, Some(price));
      }
    }
  }
}
