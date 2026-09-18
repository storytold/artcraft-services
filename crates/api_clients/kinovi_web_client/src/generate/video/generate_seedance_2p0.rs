use crate::creds::kinovi_web_session::KinoviWebSession;
use crate::error::kinovi_web_error::KinoviWebError;
use crate::pricing::cost::kinovi_seedance_fractional_generation_cost::KinoviSeedanceFractionalGenerationCost;
use crate::pricing::kinovi_cost_calculator_trait::KinoviCostCalculatorTrait;
use crate::pricing::kinovi_pricing_rate::KinoviPricingRate;
use crate::pricing::kinovi_pricing_tier::KinoviPricingTier;
use crate::requests::kinovi_host::KinoviHost;
use crate::requests::workflow_run_task::workflow_run_task::{
  workflow_run_task, KinoviAspectRatioRaw, KinoviBatchCountRaw, KinoviBitrateRaw,
  KinoviModelTypeRaw, KinoviOutputResolutionRaw, WorkflowRunTaskArgs,
  WorkflowRunTaskRequest,
};

// ── Args ──

pub struct GenerateSeedance2p0Args<'a> {
  pub request: GenerateSeedance2p0Request,
  pub session: &'a KinoviWebSession,
  pub host_override: Option<KinoviHost>,
}

// ── Request ──

#[derive(Clone, Debug)]
pub struct GenerateSeedance2p0Request {
  pub prompt: String,
  pub aspect_ratio: Option<KinoviSeedance2p0AspectRatio>,
  pub output_resolution: Option<KinoviSeedance2p0OutputResolution>,
  pub duration_seconds: u8,
  pub batch_count: Option<KinoviSeedance2p0BatchCount>,
  pub start_frame_url: Option<String>,
  pub end_frame_url: Option<String>,
  pub reference_image_urls: Option<Vec<String>>,
  pub reference_video_urls: Option<Vec<String>>,
  pub reference_audio_urls: Option<Vec<String>>,
  pub character_ids: Option<Vec<String>>,
  pub use_face_blur_hack: Option<bool>,
  /// Output video bitrate. None defaults to "standard"; `High` requests a
  /// higher bitrate. Does not affect cost.
  pub bitrate: Option<KinoviSeedance2p0Bitrate>,
  /// Whether to generate audio. None leaves the provider default unchanged.
  pub maybe_generate_audio: Option<bool>,
}

// ── Enums ──

#[derive(Debug, Clone, Copy)]
pub enum KinoviSeedance2p0AspectRatio {
  Landscape16x9,
  UltraWide21x9,
  Portrait9x16,
  Square1x1,
  Standard4x3,
  Portrait3x4,
}

#[derive(Debug, Clone, Copy)]
pub enum KinoviSeedance2p0OutputResolution {
  FourEightyP,
  SevenTwentyP,
  TenEightyP,
  FourK,
}

#[derive(Debug, Clone, Copy)]
pub enum KinoviSeedance2p0BatchCount {
  One,
  Two,
  Three,
  Four,
  Five,
  Six,
  Seven,
  Eight,
}

#[derive(Debug, Clone, Copy)]
pub enum KinoviSeedance2p0Bitrate {
  High,
}

// ── Pricing ──
//
// Seedance 2.0 Pro credit pricing (per second of output duration):
//
// | Resolution | Consumer credits/sec | Enterprise credits/sec |
// |------------|----------------------|------------------------|
// | 480p       |                   15 |                      — |
// | 720p       |                   40 |                   37.9 |
// | 1080p      |                   90 |                      — |
// | 4K         |                  200 |                      — |
//
// Attaching reference VIDEOS adds a per-output-second surcharge, billed per
// second of OUTPUT duration (not the reference video's duration). Reference
// images and audio are free.
//
// | Resolution | Consumer surcharge/sec | Enterprise surcharge/sec |
// |------------|------------------------|--------------------------|
// | 480p       |                      4 |                        — |
// | 720p       |                      8 |                     7.58 |
// | 1080p      |                     18 |                        — |
// | 4K         |                     40 |                        — |
//
// The enterprise 720p rates come from the negotiated combined prices:
// 40 → 37.9 credits/sec without a reference video, 48 → 45.48 with one; the
// surcharge is the difference (45.48 − 37.9 = 7.58). "—" = no negotiated
// discount; enterprise pricing falls back to the consumer credit rate.
//
// Default resolution (None) is 720p. Batch count multiplies the total cost.

/// Per-second base credit rates by resolution.
const SEEDANCE_2P0_480P: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 15.0,
  maybe_enterprise_credits: None,
};
const SEEDANCE_2P0_720P: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 40.0,
  maybe_enterprise_credits: Some(37.9),
};
const SEEDANCE_2P0_1080P: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 90.0,
  maybe_enterprise_credits: None,
};
const SEEDANCE_2P0_4K: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 200.0,
  maybe_enterprise_credits: None,
};

/// Per-second video-reference surcharge rates by resolution. Flat per
/// generation regardless of how many reference videos are attached
/// (assumption — Kinovi's pricing page only shows one).
const SEEDANCE_2P0_480P_VIDEO_REF_SURCHARGE: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 4.0,
  maybe_enterprise_credits: None,
};
const SEEDANCE_2P0_720P_VIDEO_REF_SURCHARGE: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 8.0,
  maybe_enterprise_credits: Some(7.58),
};
const SEEDANCE_2P0_1080P_VIDEO_REF_SURCHARGE: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 18.0,
  maybe_enterprise_credits: None,
};
const SEEDANCE_2P0_4K_VIDEO_REF_SURCHARGE: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 40.0,
  maybe_enterprise_credits: None,
};

impl KinoviCostCalculatorTrait for GenerateSeedance2p0Request {
  type Cost = KinoviSeedanceFractionalGenerationCost;

  /// Calculate the cost of this generation request, in Kinovi credits and
  /// USD cents, at the given pricing tier.
  fn calculate_costs(&self, tier: KinoviPricingTier) -> KinoviSeedanceFractionalGenerationCost {
    let base_rate = match self.output_resolution {
      Some(KinoviSeedance2p0OutputResolution::FourEightyP) => SEEDANCE_2P0_480P,
      Some(KinoviSeedance2p0OutputResolution::SevenTwentyP) | None => SEEDANCE_2P0_720P,
      Some(KinoviSeedance2p0OutputResolution::TenEightyP) => SEEDANCE_2P0_1080P,
      Some(KinoviSeedance2p0OutputResolution::FourK) => SEEDANCE_2P0_4K,
    };
    let surcharge_rate = match self.output_resolution {
      Some(KinoviSeedance2p0OutputResolution::FourEightyP) => SEEDANCE_2P0_480P_VIDEO_REF_SURCHARGE,
      Some(KinoviSeedance2p0OutputResolution::SevenTwentyP) | None => SEEDANCE_2P0_720P_VIDEO_REF_SURCHARGE,
      Some(KinoviSeedance2p0OutputResolution::TenEightyP) => SEEDANCE_2P0_1080P_VIDEO_REF_SURCHARGE,
      Some(KinoviSeedance2p0OutputResolution::FourK) => SEEDANCE_2P0_4K_VIDEO_REF_SURCHARGE,
    };

    let batch_multiplier: f64 = match self.batch_count {
      None | Some(KinoviSeedance2p0BatchCount::One) => 1.0,
      Some(KinoviSeedance2p0BatchCount::Two) => 2.0,
      Some(KinoviSeedance2p0BatchCount::Three) => 3.0,
      Some(KinoviSeedance2p0BatchCount::Four) => 4.0,
      Some(KinoviSeedance2p0BatchCount::Five) => 5.0,
      Some(KinoviSeedance2p0BatchCount::Six) => 6.0,
      Some(KinoviSeedance2p0BatchCount::Seven) => 7.0,
      Some(KinoviSeedance2p0BatchCount::Eight) => 8.0,
    };

    let output_seconds = f64::from(self.duration_seconds) * batch_multiplier;
    let base_credits = base_rate.credits(tier) * output_seconds;
    let maybe_video_reference_surcharge_credits = if self.has_video_reference() {
      Some(surcharge_rate.credits(tier) * output_seconds)
    } else {
      None
    };

    KinoviSeedanceFractionalGenerationCost::from_base_and_surcharge_at_tier(
      tier,
      base_credits,
      maybe_video_reference_surcharge_credits,
    )
  }
}

impl GenerateSeedance2p0Request {
  fn has_video_reference(&self) -> bool {
    self.reference_video_urls
      .as_ref()
      .is_some_and(|urls| !urls.is_empty())
  }
}

// ── Response ──

pub struct GenerateSeedance2p0Response {
  pub task_id: String,
  pub order_id: String,
  pub task_ids: Option<Vec<String>>,
  pub order_ids: Option<Vec<String>>,
}

// ── Entry point ──

pub async fn generate_seedance_2p0(
  args: GenerateSeedance2p0Args<'_>,
) -> Result<GenerateSeedance2p0Response, KinoviWebError> {
  let req = args.request;

  let raw_request = WorkflowRunTaskRequest {
    model_type: KinoviModelTypeRaw::Seedance2Pro,
    prompt: req.prompt,
    aspect_ratio: map_aspect_ratio(req.aspect_ratio),
    output_resolution: Some(map_output_resolution(req.output_resolution)),
    batch_count: map_batch_count(req.batch_count),
    duration_seconds: req.duration_seconds,
    start_frame_url: req.start_frame_url,
    end_frame_url: req.end_frame_url,
    reference_image_urls: req.reference_image_urls,
    reference_video_urls: req.reference_video_urls,
    reference_audio_urls: req.reference_audio_urls,
    character_ids: req.character_ids,
    use_face_blur_hack: req.use_face_blur_hack,
    bitrate: map_bitrate(req.bitrate),
    maybe_output_format: None,
    maybe_generate_audio: req.maybe_generate_audio,
  };

  let raw_response = workflow_run_task(WorkflowRunTaskArgs {
    request: raw_request,
    session: args.session,
    host_override: args.host_override,
  }).await?;

  Ok(GenerateSeedance2p0Response {
    task_id: raw_response.task_id,
    order_id: raw_response.order_id,
    task_ids: raw_response.task_ids,
    order_ids: raw_response.order_ids,
  })
}

// ── Mapping helpers ──

fn map_aspect_ratio(ar: Option<KinoviSeedance2p0AspectRatio>) -> KinoviAspectRatioRaw {
  match ar {
    Some(KinoviSeedance2p0AspectRatio::Landscape16x9) => KinoviAspectRatioRaw::Landscape16x9,
    Some(KinoviSeedance2p0AspectRatio::UltraWide21x9) => KinoviAspectRatioRaw::UltraWide21x9,
    Some(KinoviSeedance2p0AspectRatio::Portrait9x16) => KinoviAspectRatioRaw::Portrait9x16,
    Some(KinoviSeedance2p0AspectRatio::Square1x1) => KinoviAspectRatioRaw::Square1x1,
    Some(KinoviSeedance2p0AspectRatio::Standard4x3) => KinoviAspectRatioRaw::Landscape4x3,
    Some(KinoviSeedance2p0AspectRatio::Portrait3x4) => KinoviAspectRatioRaw::Portrait3x4,
    None => KinoviAspectRatioRaw::Landscape16x9,
  }
}

fn map_output_resolution(res: Option<KinoviSeedance2p0OutputResolution>) -> KinoviOutputResolutionRaw {
  match res {
    Some(KinoviSeedance2p0OutputResolution::FourEightyP) => KinoviOutputResolutionRaw::FourEightyP,
    // Unset resolves to 720p — MUST stay in lockstep with calculate_costs(),
    // which prices None as 720p. Never leave the resolution to the Kinovi
    // server-side default; a silent upstream change would make supplier cost
    // diverge from what we billed.
    Some(KinoviSeedance2p0OutputResolution::SevenTwentyP) | None => KinoviOutputResolutionRaw::SevenTwentyP,
    Some(KinoviSeedance2p0OutputResolution::TenEightyP) => KinoviOutputResolutionRaw::TenEightyP,
    Some(KinoviSeedance2p0OutputResolution::FourK) => KinoviOutputResolutionRaw::FourK,
  }
}

fn map_batch_count(bc: Option<KinoviSeedance2p0BatchCount>) -> KinoviBatchCountRaw {
  match bc {
    Some(KinoviSeedance2p0BatchCount::One) | None => KinoviBatchCountRaw::One,
    Some(KinoviSeedance2p0BatchCount::Two) => KinoviBatchCountRaw::Two,
    Some(KinoviSeedance2p0BatchCount::Three) => KinoviBatchCountRaw::Three,
    Some(KinoviSeedance2p0BatchCount::Four) => KinoviBatchCountRaw::Four,
    Some(KinoviSeedance2p0BatchCount::Five) => KinoviBatchCountRaw::Five,
    Some(KinoviSeedance2p0BatchCount::Six) => KinoviBatchCountRaw::Six,
    Some(KinoviSeedance2p0BatchCount::Seven) => KinoviBatchCountRaw::Seven,
    Some(KinoviSeedance2p0BatchCount::Eight) => KinoviBatchCountRaw::Eight,
  }
}

fn map_bitrate(bitrate: Option<KinoviSeedance2p0Bitrate>) -> Option<KinoviBitrateRaw> {
  match bitrate {
    Some(KinoviSeedance2p0Bitrate::High) => Some(KinoviBitrateRaw::High),
    None => None,
  }
}

// ── Tests ──

#[cfg(test)]
mod tests {
  use super::*;
  use crate::creds::kinovi_web_session::KinoviWebSession;
  use crate::test_utils::get_test_cookies::get_test_cookies;
  use crate::test_utils::setup_test_logging::setup_test_logging;
  use errors::AnyhowResult;
  use log::LevelFilter;

  mod pricing_tests {
    use super::*;

    /// Expected fractional cents are written to 4 decimal places.
    const FLOAT_TOLERANCE: f64 = 0.0001;

    // ── Consumer credit tables ──
    //
    // The standard published rates: 480p 15/s, 720p 40/s, 1080p 90/s,
    // 4K 200/s. Batch count multiplies the total.

    mod consumer_pricing_tables {
      use super::*;

      #[test]
      fn table_480p() {
        assert_eq!(consumer_credits(&r480(4)), 60.0);
        assert_eq!(consumer_credits(&r480(5)), 75.0);
        assert_eq!(consumer_credits(&r480(10)), 150.0);
        assert_eq!(consumer_credits(&r480(15)), 225.0);

        assert_eq!(consumer_cents(&r480(4)), 32);
        assert_eq!(consumer_cents(&r480(5)), 39);
        assert_eq!(consumer_cents(&r480(10)), 78);
        assert_eq!(consumer_cents(&r480(15)), 117);
      }

      #[test]
      fn table_720p() {
        assert_eq!(consumer_credits(&r720(4)), 160.0);
        assert_eq!(consumer_credits(&r720(5)), 200.0);
        assert_eq!(consumer_credits(&r720(10)), 400.0);
        assert_eq!(consumer_credits(&r720(15)), 600.0);

        assert_eq!(consumer_cents(&r720(4)), 83);
        assert_eq!(consumer_cents(&r720(5)), 104);
        assert_eq!(consumer_cents(&r720(10)), 208);
        assert_eq!(consumer_cents(&r720(15)), 311);
      }

      #[test]
      fn table_720p_with_video_reference() {
        // 48 credits/sec (40 base + 8 surcharge).
        assert_eq!(consumer_credits(&with_video_ref(r720(4))), 192.0);
        assert_eq!(consumer_credits(&with_video_ref(r720(5))), 240.0);
        assert_eq!(consumer_credits(&with_video_ref(r720(10))), 480.0);
        assert_eq!(consumer_credits(&with_video_ref(r720(15))), 720.0);

        assert_eq!(consumer_cents(&with_video_ref(r720(4))), 100);
        assert_eq!(consumer_cents(&with_video_ref(r720(5))), 125);
        assert_eq!(consumer_cents(&with_video_ref(r720(10))), 249);
        assert_eq!(consumer_cents(&with_video_ref(r720(15))), 374);
      }

      #[test]
      fn table_1080p() {
        assert_eq!(consumer_credits(&r1080(4)), 360.0);
        assert_eq!(consumer_credits(&r1080(5)), 450.0);
        assert_eq!(consumer_credits(&r1080(10)), 900.0);
        assert_eq!(consumer_credits(&r1080(15)), 1350.0);

        assert_eq!(consumer_cents(&r1080(4)), 187);
        assert_eq!(consumer_cents(&r1080(5)), 234);
        assert_eq!(consumer_cents(&r1080(10)), 467);
        assert_eq!(consumer_cents(&r1080(15)), 700);
      }

      #[test]
      fn table_4k() {
        assert_eq!(consumer_credits(&r4k(4)), 800.0);
        assert_eq!(consumer_credits(&r4k(5)), 1000.0);
        assert_eq!(consumer_credits(&r4k(10)), 2000.0);
        assert_eq!(consumer_credits(&r4k(15)), 3000.0);

        assert_eq!(consumer_cents(&r4k(4)), 415);
        assert_eq!(consumer_cents(&r4k(5)), 519);
        assert_eq!(consumer_cents(&r4k(10)), 1037);
        assert_eq!(consumer_cents(&r4k(15)), 1555);
      }

      #[test]
      fn table_4k_with_video_reference() {
        // 240 credits/sec (200 base + 40 surcharge).
        assert_eq!(consumer_credits(&with_video_ref(r4k(4))), 960.0);
        assert_eq!(consumer_credits(&with_video_ref(r4k(5))), 1200.0);
        assert_eq!(consumer_credits(&with_video_ref(r4k(10))), 2400.0);
        assert_eq!(consumer_credits(&with_video_ref(r4k(15))), 3600.0);

        assert_eq!(consumer_cents(&with_video_ref(r4k(4))), 498);
        assert_eq!(consumer_cents(&with_video_ref(r4k(5))), 622);
        assert_eq!(consumer_cents(&with_video_ref(r4k(10))), 1244);
        assert_eq!(consumer_cents(&with_video_ref(r4k(15))), 1866);
      }

      #[test]
      fn explicit_720p_same_as_default() {
        let default = r720(5).calculate_consumer_costs();
        let explicit = build_request(5, Some(KinoviSeedance2p0OutputResolution::SevenTwentyP), None).calculate_consumer_costs();
        assert_eq!(default, explicit);
      }
    }

    // ── Enterprise 720p (the only negotiated discount) ──
    //
    // Base 40 → 37.9 credits/sec; with a reference video 48 → 45.48
    // credits/sec (surcharge 8 → 7.58).

    mod enterprise_720p {
      use super::*;

      #[test]
      fn table_base() {
        assert_eq!(enterprise_credits(&r720(4)), 151.6);
        assert_eq!(enterprise_credits(&r720(5)), 189.5);
        assert_eq!(enterprise_credits(&r720(10)), 379.0);
        assert_eq!(enterprise_credits(&r720(15)), 568.5);

        assert_eq!(enterprise_cents(&r720(4)), 63);
        assert_eq!(enterprise_cents(&r720(5)), 78);
        assert_eq!(enterprise_cents(&r720(10)), 156);
        assert_eq!(enterprise_cents(&r720(15)), 234);
      }

      #[test]
      fn table_with_video_reference() {
        // 45.48 credits/sec combined.
        assert_eq!(enterprise_credits(&with_video_ref(r720(4))), 181.92);
        assert_eq!(enterprise_credits(&with_video_ref(r720(5))), 227.4);
        assert_eq!(enterprise_credits(&with_video_ref(r720(10))), 454.8);
        assert_eq!(enterprise_credits(&with_video_ref(r720(15))), 682.2);

        assert_eq!(enterprise_cents(&with_video_ref(r720(4))), 75);
        assert_eq!(enterprise_cents(&with_video_ref(r720(5))), 94);
        assert_eq!(enterprise_cents(&with_video_ref(r720(10))), 188);
        assert_eq!(enterprise_cents(&with_video_ref(r720(15))), 281);
      }

      #[test]
      fn base_and_surcharge_breakdown_5s() {
        let costs = with_video_ref(r720(5)).calculate_enterprise_costs();
        assert_eq!(costs.base_cost.kinovi_credits, 189.5); // 5s × 37.9
        assert_eq!(costs.video_reference_surcharge_cost.map(|c| c.kinovi_credits), Some(37.9)); // 5s × 7.58
        assert_eq!(costs.total_cost.kinovi_credits, 227.4); // 5s × 45.48

        assert_eq!(costs.total_cost.usd_cents_rounded_up, 94);
        assert_eq!(costs.total_cost.usd_cents_rounded_down, 93);
        assert!((costs.total_cost.usd_cents_fractional - 93.5187).abs() < FLOAT_TOLERANCE);
      }

      #[test]
      fn batch_multiplies_discounted_credits() {
        let batch4 = build_request(5, None, Some(KinoviSeedance2p0BatchCount::Four)).calculate_enterprise_costs();
        assert_eq!(batch4.total_cost.kinovi_credits, 758.0); // 189.5 × 4
      }
    }

    // ── Enterprise fallback (no negotiated discount at 480p/1080p/4K) ──
    //
    // Credits fall back to the consumer rate; only the credit purchase rate
    // (243.16 vs 192.98 credits/$1) makes enterprise cheaper in USD.

    mod enterprise_fallback {
      use super::*;

      #[test]
      fn credits_match_consumer_rates() {
        for request in [r480(5), r1080(5), r4k(5), with_video_ref(r4k(5))] {
          assert_eq!(enterprise_credits(&request), consumer_credits(&request));
        }
      }

      #[test]
      fn table_480p() {
        assert_eq!(enterprise_credits(&r480(4)), 60.0);
        assert_eq!(enterprise_credits(&r480(5)), 75.0);
        assert_eq!(enterprise_credits(&r480(10)), 150.0);
        assert_eq!(enterprise_credits(&r480(15)), 225.0);

        assert_eq!(enterprise_cents(&r480(4)), 25);
        assert_eq!(enterprise_cents(&r480(5)), 31);
        assert_eq!(enterprise_cents(&r480(10)), 62);
        assert_eq!(enterprise_cents(&r480(15)), 93);
      }

      #[test]
      fn table_1080p() {
        assert_eq!(enterprise_credits(&r1080(4)), 360.0);
        assert_eq!(enterprise_credits(&r1080(5)), 450.0);
        assert_eq!(enterprise_credits(&r1080(10)), 900.0);
        assert_eq!(enterprise_credits(&r1080(15)), 1350.0);

        assert_eq!(enterprise_cents(&r1080(4)), 149);
        assert_eq!(enterprise_cents(&r1080(5)), 186);
        assert_eq!(enterprise_cents(&r1080(10)), 371);
        assert_eq!(enterprise_cents(&r1080(15)), 556);
      }

      #[test]
      fn table_4k() {
        assert_eq!(enterprise_credits(&r4k(4)), 800.0);
        assert_eq!(enterprise_credits(&r4k(5)), 1000.0);
        assert_eq!(enterprise_credits(&r4k(10)), 2000.0);
        assert_eq!(enterprise_credits(&r4k(15)), 3000.0);

        assert_eq!(enterprise_cents(&r4k(4)), 330);
        assert_eq!(enterprise_cents(&r4k(5)), 412);
        assert_eq!(enterprise_cents(&r4k(10)), 823);
        assert_eq!(enterprise_cents(&r4k(15)), 1234);
      }

      #[test]
      fn usd_is_still_cheaper_at_the_enterprise_purchase_rate() {
        for request in [r480(5), r1080(5), r4k(5)] {
          let consumer = request.calculate_consumer_costs().total_cost.usd_cents_fractional;
          let enterprise = request.calculate_enterprise_costs().total_cost.usd_cents_fractional;
          assert!(enterprise < consumer);
        }
      }
    }

    // ── Video-reference surcharge ──
    //
    // Attaching a reference video adds a per-output-second surcharge
    // (consumer: 480p +4/s, 720p +8/s, 1080p +18/s, 4K +40/s). Flat per
    // generation regardless of how many reference videos are attached.

    mod video_reference_surcharge_tests {
      use super::*;

      /// The full consumer base + surcharge table. Asserts every part: base,
      /// surcharge, and the derived total.
      #[test]
      fn consumer_base_and_surcharge_table() {
        // (request, duration, base credits, surcharge credits)
        let cases: &[(fn(u8) -> GenerateSeedance2p0Request, u8, f64, f64)] = &[
          // 480p: base 15/s + 4/s
          (r480, 5, 75.0, 20.0),
          (r480, 10, 150.0, 40.0),
          // 720p: base 40/s + 8/s
          (r720, 5, 200.0, 40.0),
          (r720, 10, 400.0, 80.0),
          // 1080p: base 90/s + 18/s
          (r1080, 5, 450.0, 90.0),
          (r1080, 10, 900.0, 180.0),
          // 4K: base 200/s + 40/s
          (r4k, 5, 1000.0, 200.0),
          (r4k, 10, 2000.0, 400.0),
        ];

        for (make, duration, base, surcharge) in cases {
          let costs = with_video_ref(make(*duration)).calculate_consumer_costs();
          assert_eq!(costs.base_cost.kinovi_credits, *base, "base for {duration}s");
          assert_eq!(costs.video_reference_surcharge_cost.map(|c| c.kinovi_credits), Some(*surcharge), "surcharge for {duration}s");
          assert_eq!(costs.total_cost.kinovi_credits, base + surcharge, "total for {duration}s");
        }
      }

      /// The base and surcharge parts each carry their own USD conversions.
      #[test]
      fn parts_have_their_own_usd_conversions() {
        // Consumer 720p 5s + video ref: base 200, surcharge 40, total 240.
        let costs = with_video_ref(r720(5)).calculate_consumer_costs();

        assert_eq!(costs.base_cost.usd_cents_rounded_up, 104);
        assert_eq!(costs.base_cost.usd_cents_rounded_down, 103);
        assert!((costs.base_cost.usd_cents_fractional - 103.6377).abs() < FLOAT_TOLERANCE);

        let surcharge = costs.video_reference_surcharge_cost.expect("should have surcharge");
        assert_eq!(surcharge.kinovi_credits, 40.0);
        assert_eq!(surcharge.usd_cents_rounded_up, 21);
        assert_eq!(surcharge.usd_cents_rounded_down, 20);
        assert!((surcharge.usd_cents_fractional - 20.7275).abs() < FLOAT_TOLERANCE);

        assert_eq!(costs.total_cost.kinovi_credits, 240.0);
        assert_eq!(costs.total_cost.usd_cents_rounded_up, 125);
        assert_eq!(costs.total_cost.usd_cents_rounded_down, 124);
        assert!((costs.total_cost.usd_cents_fractional - 124.3652).abs() < FLOAT_TOLERANCE);
      }

      #[test]
      fn empty_video_reference_list_has_no_surcharge() {
        let mut request = r720(5);
        request.reference_video_urls = Some(vec![]);
        for costs in [request.calculate_consumer_costs(), request.calculate_enterprise_costs()] {
          assert!(costs.video_reference_surcharge_cost.is_none());
        }
      }

      /// Surcharge is flat per generation regardless of how many reference
      /// videos are attached (assumption — Kinovi's page only shows one).
      #[test]
      fn multiple_video_references_charge_once() {
        let mut request = r720(5);
        request.reference_video_urls = Some(vec![
          "https://example.com/a.mp4".to_string(),
          "https://example.com/b.mp4".to_string(),
        ]);
        assert_eq!(consumer_credits(&request), 240.0);
        assert_eq!(enterprise_credits(&request), 227.4);
      }

      /// The surcharge applies per generated video, so batches multiply it.
      #[test]
      fn batch_multiplies_surcharge() {
        let request = with_video_ref(build_request(5, None, Some(KinoviSeedance2p0BatchCount::Two)));
        // (200 base + 40 surcharge) × 2 = 480 credits.
        let costs = request.calculate_consumer_costs();
        assert_eq!(costs.base_cost.kinovi_credits, 400.0);
        assert_eq!(costs.video_reference_surcharge_cost.map(|c| c.kinovi_credits), Some(80.0));
        assert_eq!(costs.total_cost.kinovi_credits, 480.0);
      }
    }

    // ── Batch multiplier ──

    mod batch_tests {
      use super::*;
      use crate::test_utils::assert_batch_cost::assert_batch_cost;

      const BATCHES: [KinoviSeedance2p0BatchCount; 8] = [
        KinoviSeedance2p0BatchCount::One,
        KinoviSeedance2p0BatchCount::Two,
        KinoviSeedance2p0BatchCount::Three,
        KinoviSeedance2p0BatchCount::Four,
        KinoviSeedance2p0BatchCount::Five,
        KinoviSeedance2p0BatchCount::Six,
        KinoviSeedance2p0BatchCount::Seven,
        KinoviSeedance2p0BatchCount::Eight,
      ];

      // Each pair is (credits, USD cents rounded up), for batches 1 through 8.
      // Pin literal totals so rate changes fail independently of scaling checks.
      #[test]
      fn fixed_five_second_batch_prices_without_video_reference() {
        let cases = [
          (KinoviSeedance2p0OutputResolution::FourEightyP, KinoviPricingTier::Consumer, [
            (75.0, 39), (150.0, 78), (225.0, 117), (300.0, 156),
            (375.0, 195), (450.0, 234), (525.0, 273), (600.0, 311),
          ]),
          (KinoviSeedance2p0OutputResolution::SevenTwentyP, KinoviPricingTier::Consumer, [
            (200.0, 104), (400.0, 208), (600.0, 311), (800.0, 415),
            (1000.0, 519), (1200.0, 622), (1400.0, 726), (1600.0, 830),
          ]),
          (KinoviSeedance2p0OutputResolution::TenEightyP, KinoviPricingTier::Consumer, [
            (450.0, 234), (900.0, 467), (1350.0, 700), (1800.0, 933),
            (2250.0, 1166), (2700.0, 1400), (3150.0, 1633), (3600.0, 1866),
          ]),
          (KinoviSeedance2p0OutputResolution::FourK, KinoviPricingTier::Consumer, [
            (1000.0, 519), (2000.0, 1037), (3000.0, 1555), (4000.0, 2073),
            (5000.0, 2591), (6000.0, 3110), (7000.0, 3628), (8000.0, 4146),
          ]),
          (KinoviSeedance2p0OutputResolution::FourEightyP, KinoviPricingTier::Enterprise, [
            (75.0, 31), (150.0, 62), (225.0, 93), (300.0, 124),
            (375.0, 155), (450.0, 186), (525.0, 216), (600.0, 247),
          ]),
          (KinoviSeedance2p0OutputResolution::SevenTwentyP, KinoviPricingTier::Enterprise, [
            (189.5, 78), (379.0, 156), (568.5, 234), (758.0, 312),
            (947.5, 390), (1137.0, 468), (1326.5, 546), (1516.0, 624),
          ]),
          (KinoviSeedance2p0OutputResolution::TenEightyP, KinoviPricingTier::Enterprise, [
            (450.0, 186), (900.0, 371), (1350.0, 556), (1800.0, 741),
            (2250.0, 926), (2700.0, 1111), (3150.0, 1296), (3600.0, 1481),
          ]),
          (KinoviSeedance2p0OutputResolution::FourK, KinoviPricingTier::Enterprise, [
            (1000.0, 412), (2000.0, 823), (3000.0, 1234), (4000.0, 1646),
            (5000.0, 2057), (6000.0, 2468), (7000.0, 2879), (8000.0, 3291),
          ]),
        ];
        for (resolution, tier, prices) in cases {
          for (batch, (credits, usd_cents)) in BATCHES.into_iter().zip(prices) {
            let request = build_request(5, Some(resolution), Some(batch));
            let cost = request.calculate_costs(tier).total_cost;
            assert_eq!(
              (cost.kinovi_credits, cost.usd_cents_rounded_up),
              (credits, usd_cents),
              "{resolution:?} {tier:?} {batch:?}",
            );
          }
        }
      }

      #[test]
      fn fixed_five_second_batch_prices_with_video_reference() {
        let cases = [
          (KinoviSeedance2p0OutputResolution::FourEightyP, KinoviPricingTier::Consumer, [
            (95.0, 50), (190.0, 99), (285.0, 148), (380.0, 197),
            (475.0, 247), (570.0, 296), (665.0, 345), (760.0, 394),
          ]),
          (KinoviSeedance2p0OutputResolution::SevenTwentyP, KinoviPricingTier::Consumer, [
            (240.0, 125), (480.0, 249), (720.0, 374), (960.0, 498),
            (1200.0, 622), (1440.0, 747), (1680.0, 871), (1920.0, 995),
          ]),
          (KinoviSeedance2p0OutputResolution::TenEightyP, KinoviPricingTier::Consumer, [
            (540.0, 280), (1080.0, 560), (1620.0, 840), (2160.0, 1120),
            (2700.0, 1400), (3240.0, 1679), (3780.0, 1959), (4320.0, 2239),
          ]),
          (KinoviSeedance2p0OutputResolution::FourK, KinoviPricingTier::Consumer, [
            (1200.0, 622), (2400.0, 1244), (3600.0, 1866), (4800.0, 2488),
            (6000.0, 3110), (7200.0, 3731), (8400.0, 4353), (9600.0, 4975),
          ]),
          (KinoviSeedance2p0OutputResolution::FourEightyP, KinoviPricingTier::Enterprise, [
            (95.0, 40), (190.0, 79), (285.0, 118), (380.0, 157),
            (475.0, 196), (570.0, 235), (665.0, 274), (760.0, 313),
          ]),
          (KinoviSeedance2p0OutputResolution::SevenTwentyP, KinoviPricingTier::Enterprise, [
            (227.4, 94), (454.8, 188), (682.2, 281), (909.6, 375),
            (1137.0, 468), (1364.4, 562), (1591.8, 655), (1819.2, 749),
          ]),
          (KinoviSeedance2p0OutputResolution::TenEightyP, KinoviPricingTier::Enterprise, [
            (540.0, 223), (1080.0, 445), (1620.0, 667), (2160.0, 889),
            (2700.0, 1111), (3240.0, 1333), (3780.0, 1555), (4320.0, 1777),
          ]),
          (KinoviSeedance2p0OutputResolution::FourK, KinoviPricingTier::Enterprise, [
            (1200.0, 494), (2400.0, 988), (3600.0, 1481), (4800.0, 1975),
            (6000.0, 2468), (7200.0, 2962), (8400.0, 3455), (9600.0, 3949),
          ]),
        ];
        for (resolution, tier, prices) in cases {
          for (batch, (credits, usd_cents)) in BATCHES.into_iter().zip(prices) {
            let request = build_request(5, Some(resolution), Some(batch));
            let request = with_video_ref(request);
            let cost = request.calculate_costs(tier).total_cost;
            assert_eq!(
              (cost.kinovi_credits, cost.usd_cents_rounded_up),
              (credits, usd_cents),
              "{resolution:?} {tier:?} {batch:?}",
            );
          }
        }
      }

      #[test]
      fn all_batches_scale_every_resolution_and_surcharge_at_both_tiers() {
        let batches = [
          (None, 1),
          (Some(KinoviSeedance2p0BatchCount::One), 1),
          (Some(KinoviSeedance2p0BatchCount::Two), 2),
          (Some(KinoviSeedance2p0BatchCount::Three), 3),
          (Some(KinoviSeedance2p0BatchCount::Four), 4),
          (Some(KinoviSeedance2p0BatchCount::Five), 5),
          (Some(KinoviSeedance2p0BatchCount::Six), 6),
          (Some(KinoviSeedance2p0BatchCount::Seven), 7),
          (Some(KinoviSeedance2p0BatchCount::Eight), 8),
        ];
        for tier in [KinoviPricingTier::Enterprise, KinoviPricingTier::Consumer] {
          for resolution in [
            None,
            Some(KinoviSeedance2p0OutputResolution::FourEightyP),
            Some(KinoviSeedance2p0OutputResolution::SevenTwentyP),
            Some(KinoviSeedance2p0OutputResolution::TenEightyP),
            Some(KinoviSeedance2p0OutputResolution::FourK),
          ] {
            for duration in [4, 5, 15] {
              for references in [
                None,
                Some(vec![]),
                Some(vec!["https://example.com/ref.mp4".to_string()]),
                Some(vec!["https://example.com/ref1.mp4".to_string(), "https://example.com/ref2.mp4".to_string()]),
              ] {
                let mut request = build_request(duration, resolution, None);
                request.reference_video_urls = references;
                let single = request.calculate_costs(tier);
                for (batch, count) in batches {
                  request.batch_count = batch;
                  let batched = request.calculate_costs(tier);
                  let context = format!("{tier:?} {resolution:?} {duration}s {batch:?} references={:?}", request.reference_video_urls);
                  assert_batch_cost(single.base_cost, batched.base_cost, count, tier, &context);
                  assert_batch_cost(single.total_cost, batched.total_cost, count, tier, &context);
                  match (single.video_reference_surcharge_cost, batched.video_reference_surcharge_cost) {
                    (None, None) => {},
                    (Some(single), Some(batched)) => assert_batch_cost(single, batched, count, tier, &context),
                    _ => panic!("{context}: batch count changed surcharge presence"),
                  }
                }
              }
            }
          }
        }
      }

      #[test]
      fn batch_1_is_base() {
        let base = consumer_credits(&r720(5));
        let explicit = consumer_credits(&build_request(5, None, Some(KinoviSeedance2p0BatchCount::One)));
        assert_eq!(base, explicit);
      }
    }

    // ── Relative pricing ──

    mod relative_tests {
      use super::*;

      #[test]
      fn resolution_ordering_at_both_tiers() {
        for tier in [KinoviPricingTier::Enterprise, KinoviPricingTier::Consumer] {
          let c480 = r480(5).calculate_costs(tier).total_cost.kinovi_credits;
          let c720 = r720(5).calculate_costs(tier).total_cost.kinovi_credits;
          let c1080 = r1080(5).calculate_costs(tier).total_cost.kinovi_credits;
          let c4k = r4k(5).calculate_costs(tier).total_cost.kinovi_credits;
          assert!(c480 < c720 && c720 < c1080 && c1080 < c4k, "{tier:?}");
        }
      }

      #[test]
      fn cost_scales_with_duration() {
        let c3 = enterprise_credits(&r720(3));
        let c10 = enterprise_credits(&r720(10));
        let c15 = enterprise_credits(&r720(15));
        assert!(c3 < c10);
        assert!(c10 < c15);
      }
    }

    // ── Tier dispatch ──

    #[test]
    fn convenience_methods_match_explicit_tier() {
      let request = with_video_ref(r720(5));
      assert_eq!(request.calculate_consumer_costs(), request.calculate_costs(KinoviPricingTier::Consumer));
      assert_eq!(request.calculate_enterprise_costs(), request.calculate_costs(KinoviPricingTier::Enterprise));
    }

    // ── Aspect ratio doesn't affect cost ──

    #[test]
    fn aspect_ratio_does_not_affect_credits() {
      let baseline = consumer_credits(&r720(5));

      let ratios = [
        KinoviSeedance2p0AspectRatio::Landscape16x9,
        KinoviSeedance2p0AspectRatio::UltraWide21x9,
        KinoviSeedance2p0AspectRatio::Portrait9x16,
        KinoviSeedance2p0AspectRatio::Square1x1,
        KinoviSeedance2p0AspectRatio::Standard4x3,
        KinoviSeedance2p0AspectRatio::Portrait3x4,
      ];

      for ar in &ratios {
        let mut request = r720(5);
        request.aspect_ratio = Some(*ar);
        assert_eq!(
          consumer_credits(&request), baseline,
          "Aspect ratio {:?} should not change credits from baseline {}", ar, baseline,
        );
      }
    }

    // ── Bitrate doesn't affect cost ──

    #[test]
    fn high_bitrate_does_not_affect_credits() {
      let baseline = consumer_credits(&r720(5));

      let mut high = r720(5);
      high.bitrate = Some(KinoviSeedance2p0Bitrate::High);

      assert_eq!(
        consumer_credits(&high), baseline,
        "High bitrate should not change credits from baseline {}", baseline,
      );
    }

    // ── Helpers ──

    fn build_request(
      duration_seconds: u8,
      output_resolution: Option<KinoviSeedance2p0OutputResolution>,
      batch_count: Option<KinoviSeedance2p0BatchCount>,
    ) -> GenerateSeedance2p0Request {
      GenerateSeedance2p0Request {
        prompt: String::new(),
        aspect_ratio: None,
        output_resolution,
        batch_count,
        duration_seconds,
        start_frame_url: None,
        end_frame_url: None,
        reference_image_urls: None,
        reference_video_urls: None,
        reference_audio_urls: None,
        character_ids: None,
        use_face_blur_hack: None,
        bitrate: None,
        maybe_generate_audio: None,
      }
    }

    fn r480(dur: u8) -> GenerateSeedance2p0Request {
      build_request(dur, Some(KinoviSeedance2p0OutputResolution::FourEightyP), None)
    }

    fn r720(dur: u8) -> GenerateSeedance2p0Request {
      build_request(dur, None, None)
    }

    fn r1080(dur: u8) -> GenerateSeedance2p0Request {
      build_request(dur, Some(KinoviSeedance2p0OutputResolution::TenEightyP), None)
    }

    fn r4k(dur: u8) -> GenerateSeedance2p0Request {
      build_request(dur, Some(KinoviSeedance2p0OutputResolution::FourK), None)
    }

    fn with_video_ref(mut request: GenerateSeedance2p0Request) -> GenerateSeedance2p0Request {
      request.reference_video_urls = Some(vec!["https://example.com/ref.mp4".to_string()]);
      request
    }

    fn consumer_credits(request: &GenerateSeedance2p0Request) -> f64 {
      request.calculate_consumer_costs().total_cost.kinovi_credits
    }

    fn consumer_cents(request: &GenerateSeedance2p0Request) -> u64 {
      request.calculate_consumer_costs().total_cost.usd_cents_rounded_up
    }

    fn enterprise_credits(request: &GenerateSeedance2p0Request) -> f64 {
      request.calculate_enterprise_costs().total_cost.kinovi_credits
    }

    fn enterprise_cents(request: &GenerateSeedance2p0Request) -> u64 {
      request.calculate_enterprise_costs().total_cost.usd_cents_rounded_up
    }
  }

  use crate::requests::prepare_file_upload::prepare_file_upload::{prepare_file_upload, PrepareFileUploadArgs};
  use crate::requests::upload_file::upload_file::{upload_file, UploadFileArgs};

  const STEAMPUNK_CLOWN_ID: &str = "char_1775176566518_sik0te";
  const MOCHI_ID: &str = "char_1775177718294_g2pitx";

  mod text_to_video {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_text_to_video_default() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p0Request {
          prompt: "A corgi and a shiba are playing chess against one another".to_string(),
          aspect_ratio: None,
          output_resolution: None,
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: None,
          end_frame_url: None,
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
          character_ids: None,
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("t2v default — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert!(!result.order_id.is_empty());
      assert_eq!(1, 2);
      Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_text_to_video_1080p() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p0Request {
          prompt: "A dragon soaring over a medieval castle at sunset".to_string(),
          aspect_ratio: Some(KinoviSeedance2p0AspectRatio::Landscape16x9),
          output_resolution: Some(KinoviSeedance2p0OutputResolution::TenEightyP),
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: None,
          end_frame_url: None,
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
          character_ids: None,
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("t2v 1080p — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert_eq!(1, 2);
      Ok(())
    }
  }

  mod ultra_wide {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_text_to_video_21x9() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p0Request {
          prompt: "A corgi is riding on the back of a sauropod dinosaur".to_string(),
          aspect_ratio: Some(KinoviSeedance2p0AspectRatio::UltraWide21x9),
          output_resolution: None,
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: None,
          end_frame_url: None,
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
          character_ids: None,
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("t2v 21:9 — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert_eq!(1, 2);
      Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_keyframe_21x9() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let start_frame_url = upload_test_image(&session, test_data::web::image_urls::WIDE_CORGI_SHIBA_TREASURE_OCEAN_URL).await?;

      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p0Request {
          prompt: "The dogs sail across the ocean on a treasure ship.".to_string(),
          aspect_ratio: Some(KinoviSeedance2p0AspectRatio::UltraWide21x9),
          output_resolution: None,
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: Some(start_frame_url),
          end_frame_url: None,
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
          character_ids: None,
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("keyframe 21:9 — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert_eq!(1, 2);
      Ok(())
    }
  }

  mod keyframe {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_keyframe_start_frame() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let start_frame_url = upload_test_image(&session, test_data::web::image_urls::JUNO_AT_LAKE_IMAGE_URL).await?;

      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p0Request {
          prompt: "The corgi dog watches the lake as the sun sets.".to_string(),
          aspect_ratio: Some(KinoviSeedance2p0AspectRatio::Landscape16x9),
          output_resolution: None,
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: Some(start_frame_url),
          end_frame_url: None,
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
          character_ids: None,
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("keyframe — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert_eq!(1, 2);
      Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_keyframe_start_and_end_frame() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let start_frame_url = upload_test_image(&session, test_data::web::image_urls::JUNO_AT_LAKE_IMAGE_URL).await?;
      let end_frame_url = upload_test_image(&session, test_data::web::image_urls::FOREST_BACKDROP_IMAGE_URL).await?;

      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p0Request {
          prompt: "The dog walks from the lake toward the camera.".to_string(),
          aspect_ratio: Some(KinoviSeedance2p0AspectRatio::Landscape16x9),
          output_resolution: None,
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: Some(start_frame_url),
          end_frame_url: Some(end_frame_url),
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
          character_ids: None,
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("keyframe start+end — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert_eq!(1, 2);
      Ok(())
    }
  }

  mod image_reference {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_image_references() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let img1 = upload_test_image(&session, test_data::web::image_urls::JUNO_AT_LAKE_IMAGE_URL).await?;
      let img2 = upload_test_image(&session, test_data::web::image_urls::WHITE_HOUSE_SUNSET_IMAGE_URL).await?;

      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p0Request {
          prompt: "The dog in @1 runs through the scenery in @2.".to_string(),
          aspect_ratio: Some(KinoviSeedance2p0AspectRatio::Landscape16x9),
          output_resolution: None,
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: None,
          end_frame_url: None,
          reference_image_urls: Some(vec![img1, img2]),
          reference_video_urls: None,
          reference_audio_urls: None,
          character_ids: None,
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("image ref — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert_eq!(1, 2);
      Ok(())
    }

    /// 4K with image references (Seedance 2.0 only), 5-second clip.
    #[tokio::test]
    #[ignore]
    async fn test_4k_image_references() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let img1 = upload_test_image(&session, test_data::web::image_urls::JUNO_AT_LAKE_IMAGE_URL).await?;
      let img2 = upload_test_image(&session, test_data::web::image_urls::ERNEST_SCARED_STUPID_IMAGE_URL).await?;
      let img3 = upload_test_image(&session, test_data::web::image_urls::FOREST_BACKDROP_IMAGE_URL).await?;

      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p0Request {
          prompt: "The dog in @1 explores the scenery in @3 and meets the friendly man in @2. Cinematic 4K detail.".to_string(),
          aspect_ratio: Some(KinoviSeedance2p0AspectRatio::Landscape16x9),
          output_resolution: Some(KinoviSeedance2p0OutputResolution::FourK),
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: None,
          end_frame_url: None,
          reference_image_urls: Some(vec![img1, img2, img3]),
          reference_video_urls: None,
          reference_audio_urls: None,
          character_ids: None,
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("4K image ref — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert!(!result.order_id.is_empty());
      assert_eq!(1, 2, "Inspect output above");
      Ok(())
    }
  }

  mod video_reference {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_video_reference() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;

      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p0Request {
          prompt: "Change @video1 to a nighttime scene with moonlight.".to_string(),
          aspect_ratio: Some(KinoviSeedance2p0AspectRatio::Landscape16x9),
          output_resolution: None,
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: None,
          end_frame_url: None,
          reference_image_urls: None,
          reference_video_urls: Some(vec![
            "https://static.seedance2-pro.com/materials/20260315/1773594284659-3a46d231.mp4".to_string(),
          ]),
          reference_audio_urls: None,
          character_ids: None,
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("video ref — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert_eq!(1, 2);
      Ok(())
    }
  }

  mod character_reference {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_single_character() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;

      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p0Request {
          prompt: "@Steampunk Clown is juggling flaming torches in a circus tent.".to_string(),
          aspect_ratio: Some(KinoviSeedance2p0AspectRatio::Landscape16x9),
          output_resolution: None,
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: None,
          end_frame_url: None,
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
          character_ids: Some(vec![STEAMPUNK_CLOWN_ID.to_string()]),
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("character ref — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert_eq!(1, 2);
      Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_two_characters() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;

      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p0Request {
          prompt: "@Steampunk Clown and @Mochi are playing fetch in a sunny park.".to_string(),
          aspect_ratio: Some(KinoviSeedance2p0AspectRatio::Landscape16x9),
          output_resolution: None,
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: None,
          end_frame_url: None,
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
          character_ids: Some(vec![STEAMPUNK_CLOWN_ID.to_string(), MOCHI_ID.to_string()]),
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("two characters — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert_eq!(1, 2);
      Ok(())
    }
  }

  fn test_session() -> AnyhowResult<KinoviWebSession> {
    let cookies = get_test_cookies()?;
    Ok(KinoviWebSession::from_cookies_string(cookies))
  }

  async fn upload_test_image(session: &KinoviWebSession, image_url: &str) -> AnyhowResult<String> {
    let image_bytes = crate::test_utils::http_download::http_download_to_bytes(
      image_url,
    ).await?;

    let prepare_result = prepare_file_upload(PrepareFileUploadArgs {
      session,
      extension: "jpg".to_string(),
      host_override: None,
    }).await?;

    let upload_result = upload_file(UploadFileArgs {
      upload_url: prepare_result.upload_url,
      file_bytes: image_bytes,
      host_override: None,
    }).await?;

    Ok(upload_result.public_url)
  }
}
