use crate::creds::kinovi_web_session::KinoviWebSession;
use crate::error::kinovi_web_error::KinoviWebError;
use crate::pricing::cost::kinovi_fractional_generation_cost::KinoviFractionalGenerationCost;
use crate::pricing::kinovi_cost_calculator_trait::KinoviCostCalculatorTrait;
use crate::pricing::kinovi_pricing_rate::KinoviPricingRate;
use crate::pricing::kinovi_pricing_tier::KinoviPricingTier;
use crate::requests::kinovi_host::KinoviHost;
use crate::requests::workflow_run_task::workflow_run_task::{
  workflow_run_task, KinoviAspectRatioRaw, KinoviBatchCountRaw, KinoviBitrateRaw, KinoviModelTypeRaw,
  KinoviOutputFormatRaw, KinoviOutputResolutionRaw, WorkflowRunTaskArgs, WorkflowRunTaskRequest,
};

// ── Constants ──

/// Seedance 2.5 supports at most 30 seconds of video: reference-video input
/// seconds beyond this are clamped for billing.
pub const MAX_BILLED_INPUT_SECONDS: u8 = 30;

/// The minimum TOTAL billed input seconds when video references are
/// attached: however short the input videos are, the total bills at least 4
/// seconds. (Three 1-second videos sum to 3 and clamp to 4; three 3-second
/// videos sum to 9 and bill 9.)
pub const MIN_BILLED_INPUT_SECONDS: u8 = 4;

// ── Args ──

pub struct GenerateSeedance2p5Args<'a> {
  pub request: GenerateSeedance2p5Request,
  pub session: &'a KinoviWebSession,
  pub host_override: Option<KinoviHost>,
}

// ── Request ──

/// Seedance 2.5 supports keyframe (image-to-video) and reference
/// (text-to-video / reference-to-video) modalities, and 480p/720p/1080p
/// output.
///
/// On the wire it reuses the 2.5 *Preview* business type
/// (`seedance25-preview-video-generation`) with model `seedance2-5`;
/// text-to-video is sent as mode "reference".
#[derive(Clone, Debug)]
pub struct GenerateSeedance2p5Request {
  pub prompt: String,

  pub modality: KinoviSeedance2p5Modality,

  pub output_resolution: Option<KinoviSeedance2p5OutputResolution>,

  pub duration_seconds: u8,
  pub batch_count: Option<KinoviSeedance2p5BatchCount>,

  /// CALCULATION-ONLY (never sent on the wire): the total seconds of
  /// reference video input, summed across all reference videos. When video
  /// references are attached, Kinovi bills the input seconds on top of the
  /// output duration — see the pricing notes on [`calculate_costs`].
  ///
  /// [`calculate_costs`]: GenerateSeedance2p5Request::calculate_costs
  pub total_input_seconds: Option<u8>,

  /// Controls `faceBlurMode`: true sends "on"; false or None sends "off"
  /// (the model always sends the field, like 2.5 Preview).
  pub use_face_blur_hack: Option<bool>,

  /// Output video bitrate. None keeps the standard bitrate; `High` requests
  /// a higher bitrate. This does not select a container or lossless codec.
  pub maybe_bitrate: Option<KinoviSeedance2p5Bitrate>,

  /// Output container. None leaves the provider default unchanged.
  pub maybe_output_format: Option<KinoviSeedance2p5OutputFormat>,

  /// Whether to generate audio. None leaves the provider default unchanged.
  pub maybe_generate_audio: Option<bool>,
}

// ── Modality ──

/// The generation modality. The aspect ratio choice lives here because the
/// two modalities support disjoint aspect ratio sets.
#[derive(Clone, Debug)]
pub enum KinoviSeedance2p5Modality {
  /// Image-to-video (start frame + optional end frame).
  ///
  /// The aspect ratio is always "Adaptive" — the only choice the API allows
  /// in this modality, expressed on the wire by omitting the `aspectRatio`
  /// field entirely — so there is nothing to configure.
  Keyframe {
    start_frame_url: String,
    end_frame_url: Option<String>,
  },

  /// Text-to-video / reference-to-video. "Adaptive" is NOT supported here;
  /// `None` defaults to 16:9.
  Reference {
    aspect_ratio: Option<KinoviSeedance2p5AspectRatio>,
    /// Reference images, referenced in prompts as @image1, @image2, etc.
    reference_image_urls: Option<Vec<String>>,
    /// Reference videos, referenced in prompts as @video1, @video2, etc.
    reference_video_urls: Option<Vec<String>>,
    /// Reference audio, referenced in prompts as @audio1, @audio2, etc.
    reference_audio_urls: Option<Vec<String>>,
  },
}

// ── Enums ──

/// Aspect ratios for the reference (text/reference-to-video) modality.
/// There is deliberately no "Adaptive" variant: Adaptive is keyframe-only
/// (and there it is the only choice).
#[derive(Debug, Clone, Copy)]
pub enum KinoviSeedance2p5AspectRatio {
  Landscape16x9,
  UltraWide21x9,
  Portrait9x16,
  Square1x1,
  Standard4x3,
  Portrait3x4,
}

/// Output resolution. Seedance 2.5 supports 480p, 720p, and (since
/// 2026-08-17) 1080p.
#[derive(Debug, Clone, Copy)]
pub enum KinoviSeedance2p5OutputResolution {
  FourEightyP,
  SevenTwentyP,
  TenEightyP,
}

#[derive(Debug, Clone, Copy)]
pub enum KinoviSeedance2p5Bitrate {
  High,
}

/// Output container for Seedance 2.5.
#[derive(Debug, Clone, Copy)]
pub enum KinoviSeedance2p5OutputFormat {
  Mp4,
  Mov,
}

/// Number of videos to generate in one request (1–8).
#[derive(Debug, Clone, Copy)]
pub enum KinoviSeedance2p5BatchCount {
  One,
  Two,
  Three,
  Four,
  Five,
  Six,
  Seven,
  Eight,
}

// ── Pricing ──
//
// Seedance 2.5 credit pricing (credits per billed second). Without video
// references (text-to-video, keyframe, and reference mode with only
// image/audio references), billed seconds = output duration:
//
// | Resolution | Consumer credits/sec | Enterprise credits/sec |
// |------------|----------------------|------------------------|
// | 480p       |                   26 |          (no discount) |
// | 720p       |                   59 |          (no discount) |
// | 1080p      |                143.4 |                 136.23 |
//
// With video references, the rate drops but the billed seconds are the
// output duration PLUS the total seconds of reference video input
// (`total_input_seconds`, summed across all reference videos and clamped to
// the [`MIN_BILLED_INPUT_SECONDS`]..=[`MAX_BILLED_INPUT_SECONDS`] range).
// E.g. a 30s output with a 10s video reference bills as 40 seconds; a 30s
// output with two 7s references (14s total input) bills as 44 seconds.
//
// | Resolution | Consumer credits/sec | Enterprise credits/sec |
// |------------|----------------------|------------------------|
// | 480p       |                   16 |          (no discount) |
// | 720p       |                   35 |          (no discount) |
// | 1080p      |                85.68 |                   81.4 |
//
// The 1080p rates were last updated 2026-09-17 (previously 107.55 / 103.25
// and 64.26 / 61.69 credits/sec). 480p and 720p have no separately negotiated
// enterprise credit rate, so enterprise bills the same credits as consumer
// (the tiers still convert to USD at their own purchase rates). Default
// resolution (None) is 720p.

/// Per-second credit rates without video references (billed over the output
/// duration only).
const SEEDANCE_2P5_480P: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 26.0,
  maybe_enterprise_credits: None,
};
const SEEDANCE_2P5_720P: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 59.0,
  maybe_enterprise_credits: None,
};
const SEEDANCE_2P5_1080P: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 143.4,
  maybe_enterprise_credits: Some(136.23),
};

/// Per-second credit rates when video references are attached. These REPLACE
/// the base rates (they are not surcharges) and bill over output + input
/// seconds.
const SEEDANCE_2P5_480P_VIDEO_REF: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 16.0,
  maybe_enterprise_credits: None,
};
const SEEDANCE_2P5_720P_VIDEO_REF: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 35.0,
  maybe_enterprise_credits: None,
};
const SEEDANCE_2P5_1080P_VIDEO_REF: KinoviPricingRate = KinoviPricingRate {
  consumer_credits: 85.68,
  maybe_enterprise_credits: Some(81.4),
};

impl KinoviCostCalculatorTrait for GenerateSeedance2p5Request {
  type Cost = KinoviFractionalGenerationCost;

  /// Calculate the cost of this generation request, in Kinovi credits and
  /// USD cents, at the given pricing tier.
  fn calculate_costs(&self, tier: KinoviPricingTier) -> KinoviFractionalGenerationCost {
    let has_video_references = matches!(
      &self.modality,
      KinoviSeedance2p5Modality::Reference { reference_video_urls: Some(urls), .. } if !urls.is_empty()
    );

    let (rate, billed_seconds) = if has_video_references {
      let rate = match self.output_resolution {
        Some(KinoviSeedance2p5OutputResolution::FourEightyP) => SEEDANCE_2P5_480P_VIDEO_REF,
        Some(KinoviSeedance2p5OutputResolution::SevenTwentyP) | None => SEEDANCE_2P5_720P_VIDEO_REF,
        Some(KinoviSeedance2p5OutputResolution::TenEightyP) => SEEDANCE_2P5_1080P_VIDEO_REF,
      };
      // An unknown (None) or zero input duration bills the worst-case
      // maximum — never default toward the minimum.
      let input_seconds = match self.total_input_seconds {
        None | Some(0) => MAX_BILLED_INPUT_SECONDS,
        Some(seconds) => seconds.clamp(MIN_BILLED_INPUT_SECONDS, MAX_BILLED_INPUT_SECONDS),
      };
      let seconds = u64::from(self.duration_seconds) + u64::from(input_seconds);
      (rate, seconds)
    } else {
      let rate = match self.output_resolution {
        Some(KinoviSeedance2p5OutputResolution::FourEightyP) => SEEDANCE_2P5_480P,
        Some(KinoviSeedance2p5OutputResolution::SevenTwentyP) | None => SEEDANCE_2P5_720P,
        Some(KinoviSeedance2p5OutputResolution::TenEightyP) => SEEDANCE_2P5_1080P,
      };
      (rate, u64::from(self.duration_seconds))
    };

    let batch_multiplier: f64 = match self.batch_count {
      None | Some(KinoviSeedance2p5BatchCount::One) => 1.0,
      Some(KinoviSeedance2p5BatchCount::Two) => 2.0,
      Some(KinoviSeedance2p5BatchCount::Three) => 3.0,
      Some(KinoviSeedance2p5BatchCount::Four) => 4.0,
      Some(KinoviSeedance2p5BatchCount::Five) => 5.0,
      Some(KinoviSeedance2p5BatchCount::Six) => 6.0,
      Some(KinoviSeedance2p5BatchCount::Seven) => 7.0,
      Some(KinoviSeedance2p5BatchCount::Eight) => 8.0,
    };

    let total_credits = rate.credits(tier) * billed_seconds as f64 * batch_multiplier;
    tier.cost_from_credits(total_credits)
  }
}

// ── Response ──

pub struct GenerateSeedance2p5Response {
  pub task_id: String,
  pub order_id: String,
  pub task_ids: Option<Vec<String>>,
  pub order_ids: Option<Vec<String>>,
}

// ── Entry point ──

pub async fn generate_seedance_2p5(
  args: GenerateSeedance2p5Args<'_>,
) -> Result<GenerateSeedance2p5Response, KinoviWebError> {
  let raw_response = workflow_run_task(WorkflowRunTaskArgs {
    request: to_raw_request(args.request),
    session: args.session,
    host_override: args.host_override,
  }).await?;

  Ok(GenerateSeedance2p5Response {
    task_id: raw_response.task_id,
    order_id: raw_response.order_id,
    task_ids: raw_response.task_ids,
    order_ids: raw_response.order_ids,
  })
}

// ── Mapping helpers ──

fn to_raw_request(req: GenerateSeedance2p5Request) -> WorkflowRunTaskRequest {
  let (
    aspect_ratio,
    start_frame_url,
    end_frame_url,
    reference_image_urls,
    reference_video_urls,
    reference_audio_urls,
  ) = match req.modality {
    KinoviSeedance2p5Modality::Keyframe { start_frame_url, end_frame_url } => (
      // Keyframe is always "Adaptive"; the raw layer omits the aspect ratio
      // for 2.5 keyframe requests, so this placeholder never hits the wire.
      KinoviAspectRatioRaw::Landscape16x9,
      Some(start_frame_url),
      end_frame_url,
      None,
      None,
      None,
    ),
    KinoviSeedance2p5Modality::Reference {
      aspect_ratio,
      reference_image_urls,
      reference_video_urls,
      reference_audio_urls,
    } => (
      map_aspect_ratio(aspect_ratio),
      None,
      None,
      reference_image_urls,
      reference_video_urls,
      reference_audio_urls,
    ),
  };

  WorkflowRunTaskRequest {
    model_type: KinoviModelTypeRaw::Seedance2p5,
    prompt: req.prompt,
    aspect_ratio,
    output_resolution: Some(map_output_resolution(req.output_resolution)),
    duration_seconds: req.duration_seconds,
    batch_count: map_batch_count(req.batch_count),
    start_frame_url,
    end_frame_url,
    reference_image_urls,
    reference_video_urls,
    reference_audio_urls,
    character_ids: None,
    use_face_blur_hack: req.use_face_blur_hack,
    bitrate: map_bitrate(req.maybe_bitrate),
    maybe_output_format: req.maybe_output_format.map(|format| match format {
      KinoviSeedance2p5OutputFormat::Mp4 => KinoviOutputFormatRaw::Mp4,
      KinoviSeedance2p5OutputFormat::Mov => KinoviOutputFormatRaw::Mov,
    }),
    maybe_generate_audio: req.maybe_generate_audio,
  }
}

fn map_bitrate(maybe_bitrate: Option<KinoviSeedance2p5Bitrate>) -> Option<KinoviBitrateRaw> {
  match maybe_bitrate {
    Some(KinoviSeedance2p5Bitrate::High) => Some(KinoviBitrateRaw::High),
    None => None,
  }
}

fn map_aspect_ratio(ar: Option<KinoviSeedance2p5AspectRatio>) -> KinoviAspectRatioRaw {
  match ar {
    Some(KinoviSeedance2p5AspectRatio::Landscape16x9) => KinoviAspectRatioRaw::Landscape16x9,
    Some(KinoviSeedance2p5AspectRatio::UltraWide21x9) => KinoviAspectRatioRaw::UltraWide21x9,
    Some(KinoviSeedance2p5AspectRatio::Portrait9x16) => KinoviAspectRatioRaw::Portrait9x16,
    Some(KinoviSeedance2p5AspectRatio::Square1x1) => KinoviAspectRatioRaw::Square1x1,
    Some(KinoviSeedance2p5AspectRatio::Standard4x3) => KinoviAspectRatioRaw::Landscape4x3,
    Some(KinoviSeedance2p5AspectRatio::Portrait3x4) => KinoviAspectRatioRaw::Portrait3x4,
    None => KinoviAspectRatioRaw::Landscape16x9,
  }
}

fn map_output_resolution(res: Option<KinoviSeedance2p5OutputResolution>) -> KinoviOutputResolutionRaw {
  match res {
    Some(KinoviSeedance2p5OutputResolution::FourEightyP) => KinoviOutputResolutionRaw::FourEightyP,
    // Unset resolves to 720p — MUST stay in lockstep with calculate_costs(),
    // which prices None as 720p.
    Some(KinoviSeedance2p5OutputResolution::SevenTwentyP) | None => KinoviOutputResolutionRaw::SevenTwentyP,
    Some(KinoviSeedance2p5OutputResolution::TenEightyP) => KinoviOutputResolutionRaw::TenEightyP,
  }
}

fn map_batch_count(bc: Option<KinoviSeedance2p5BatchCount>) -> KinoviBatchCountRaw {
  match bc {
    Some(KinoviSeedance2p5BatchCount::One) | None => KinoviBatchCountRaw::One,
    Some(KinoviSeedance2p5BatchCount::Two) => KinoviBatchCountRaw::Two,
    Some(KinoviSeedance2p5BatchCount::Three) => KinoviBatchCountRaw::Three,
    Some(KinoviSeedance2p5BatchCount::Four) => KinoviBatchCountRaw::Four,
    Some(KinoviSeedance2p5BatchCount::Five) => KinoviBatchCountRaw::Five,
    Some(KinoviSeedance2p5BatchCount::Six) => KinoviBatchCountRaw::Six,
    Some(KinoviSeedance2p5BatchCount::Seven) => KinoviBatchCountRaw::Seven,
    Some(KinoviSeedance2p5BatchCount::Eight) => KinoviBatchCountRaw::Eight,
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

    // ── No video references: output-duration billing ──
    //
    // 480p = 26 credits/sec and 720p = 59 credits/sec at both tiers (no
    // negotiated enterprise discount); 1080p = 143.4 consumer / 136.23
    // enterprise credits/sec.

    mod without_video_references {
      use super::*;

      #[test]
      fn table_480p() {
        assert_eq!(t2v_480(1).calculate_consumer_costs().kinovi_credits, 26.0);
        assert_eq!(t2v_480(5).calculate_consumer_costs().kinovi_credits, 130.0);
        assert_eq!(t2v_480(10).calculate_consumer_costs().kinovi_credits, 260.0);
        assert_eq!(t2v_480(15).calculate_consumer_costs().kinovi_credits, 390.0);
        assert_eq!(t2v_480(30).calculate_consumer_costs().kinovi_credits, 780.0);
      }

      #[test]
      fn table_720p() {
        assert_eq!(t2v_720(1).calculate_consumer_costs().kinovi_credits, 59.0);
        assert_eq!(t2v_720(5).calculate_consumer_costs().kinovi_credits, 295.0);
        assert_eq!(t2v_720(10).calculate_consumer_costs().kinovi_credits, 590.0);
        assert_eq!(t2v_720(15).calculate_consumer_costs().kinovi_credits, 885.0);
        assert_eq!(t2v_720(30).calculate_consumer_costs().kinovi_credits, 1770.0);
      }

      #[test]
      fn table_1080p() {
        // Enterprise 136.23 credits/sec (2026-09-17).
        assert_eq!(t2v_1080(4).calculate_enterprise_costs().kinovi_credits, 544.92);
        assert_eq!(t2v_1080(5).calculate_enterprise_costs().kinovi_credits, 681.15);
        assert_eq!(t2v_1080(10).calculate_enterprise_costs().kinovi_credits, 1362.3);
        assert_eq!(t2v_1080(15).calculate_enterprise_costs().kinovi_credits, 2043.45);
        assert_eq!(t2v_1080(20).calculate_enterprise_costs().kinovi_credits, 2724.6);
        assert_eq!(t2v_1080(25).calculate_enterprise_costs().kinovi_credits, 3405.75);
        assert_eq!(t2v_1080(30).calculate_enterprise_costs().kinovi_credits, 4086.9);

        // Consumer 143.4 credits/sec.
        assert_eq!(t2v_1080(4).calculate_consumer_costs().kinovi_credits, 573.6);
        assert_eq!(t2v_1080(5).calculate_consumer_costs().kinovi_credits, 717.0);
        assert_eq!(t2v_1080(10).calculate_consumer_costs().kinovi_credits, 1434.0);
        assert_eq!(t2v_1080(15).calculate_consumer_costs().kinovi_credits, 2151.0);
        assert_eq!(t2v_1080(20).calculate_consumer_costs().kinovi_credits, 2868.0);
        assert_eq!(t2v_1080(25).calculate_consumer_costs().kinovi_credits, 3585.0);
        assert_eq!(t2v_1080(30).calculate_consumer_costs().kinovi_credits, 4302.0);
      }

      #[test]
      fn no_enterprise_credit_discount_at_480p_and_720p() {
        // 480p and 720p have no negotiated enterprise credit rate: both
        // tiers bill the same credits (USD conversion still differs).
        for duration in [4u8, 10, 30] {
          assert_eq!(
            t2v_480(duration).calculate_consumer_costs().kinovi_credits,
            t2v_480(duration).calculate_enterprise_costs().kinovi_credits,
          );
          assert_eq!(
            t2v_720(duration).calculate_consumer_costs().kinovi_credits,
            t2v_720(duration).calculate_enterprise_costs().kinovi_credits,
          );
        }
      }

      #[test]
      fn default_resolution_is_720p() {
        let default = t2v_720(10).calculate_enterprise_costs();
        let mut explicit = t2v_720(10);
        explicit.output_resolution = Some(KinoviSeedance2p5OutputResolution::SevenTwentyP);
        assert_eq!(default, explicit.calculate_enterprise_costs());
      }

      #[test]
      fn keyframe_modality_uses_no_reference_rates() {
        let mut request = keyframe_request(10);
        request.output_resolution = Some(KinoviSeedance2p5OutputResolution::FourEightyP);
        assert_eq!(request.calculate_consumer_costs().kinovi_credits, 260.0);

        let request_720 = keyframe_request(10);
        assert_eq!(request_720.calculate_consumer_costs().kinovi_credits, 590.0);

        let mut request_1080 = keyframe_request(10);
        request_1080.output_resolution = Some(KinoviSeedance2p5OutputResolution::TenEightyP);
        assert_eq!(request_1080.calculate_enterprise_costs().kinovi_credits, 1362.3);
      }

      #[test]
      fn image_and_audio_references_do_not_change_the_rate() {
        let mut request = t2v_480(10);
        set_reference_urls(
          &mut request,
          Some(vec!["https://example.com/a.png".to_string(), "https://example.com/b.jpg".to_string()]),
          None,
          Some(vec!["https://example.com/ref.wav".to_string()]),
        );
        assert_eq!(request.calculate_consumer_costs().kinovi_credits, 260.0);
      }

      #[test]
      fn empty_video_reference_list_counts_as_no_references() {
        let mut request = t2v_480(10);
        set_reference_urls(&mut request, None, Some(Vec::new()), None);
        assert_eq!(request.calculate_consumer_costs().kinovi_credits, 260.0);
      }

      #[test]
      fn total_input_seconds_is_ignored_without_video_references() {
        let mut request = t2v_480(10);
        request.total_input_seconds = Some(60);
        assert_eq!(request.calculate_consumer_costs().kinovi_credits, 260.0);
      }
    }

    // ── With video references: input seconds are billed too ──
    //
    // 480p = 16 credits/sec and 720p = 35 credits/sec at both tiers; 1080p
    // = 85.68 consumer / 81.4 enterprise credits/sec. All over
    // (output duration + total_input_seconds) billed seconds.

    mod with_video_references {
      use super::*;

      #[test]
      fn thirty_second_output_with_ten_second_reference_bills_forty_seconds() {
        // The canonical example: 30s output + 10s of input video = 40 billed seconds.
        let request = video_ref_480(30, Some(10));
        assert_eq!(request.calculate_consumer_costs().kinovi_credits, 640.0);

        let request = video_ref_720(30, Some(10));
        assert_eq!(request.calculate_consumer_costs().kinovi_credits, 1400.0);
      }

      #[test]
      fn two_seven_second_references_bill_forty_four_seconds() {
        // Two 7s references (14s total input) + 30s output = 44 billed seconds.
        let mut request = video_ref_480(30, Some(14));
        set_reference_urls(
          &mut request,
          None,
          Some(vec![
            "https://example.com/ref1.mp4".to_string(),
            "https://example.com/ref2.mp4".to_string(),
          ]),
          None,
        );
        assert_eq!(request.calculate_consumer_costs().kinovi_credits, 704.0);
      }

      #[test]
      fn table_480p_with_ten_input_seconds() {
        assert_eq!(video_ref_480(5, Some(10)).calculate_consumer_costs().kinovi_credits, 240.0);
        assert_eq!(video_ref_480(10, Some(10)).calculate_consumer_costs().kinovi_credits, 320.0);
        assert_eq!(video_ref_480(30, Some(10)).calculate_consumer_costs().kinovi_credits, 640.0);
      }

      #[test]
      fn table_720p_with_ten_input_seconds() {
        assert_eq!(video_ref_720(5, Some(10)).calculate_consumer_costs().kinovi_credits, 525.0);
        assert_eq!(video_ref_720(10, Some(10)).calculate_consumer_costs().kinovi_credits, 700.0);
        assert_eq!(video_ref_720(30, Some(10)).calculate_consumer_costs().kinovi_credits, 1400.0);
      }

      #[test]
      fn table_1080p() {
        // A 20-second output with reference videos totaling 1–30 input
        // seconds: billed seconds = 20 + clamp(input, 4, 30). E.g. a 10s
        // input video + 20s generation = 30 billed seconds.
        //
        // Enterprise 81.4 credits/sec.
        assert_eq!(video_ref_1080(20, Some(1)).calculate_enterprise_costs().kinovi_credits, 1953.6);
        assert_eq!(video_ref_1080(20, Some(5)).calculate_enterprise_costs().kinovi_credits, 2035.0);
        assert_eq!(video_ref_1080(20, Some(10)).calculate_enterprise_costs().kinovi_credits, 2442.0);
        assert_eq!(video_ref_1080(20, Some(15)).calculate_enterprise_costs().kinovi_credits, 2849.0);
        assert_eq!(video_ref_1080(20, Some(20)).calculate_enterprise_costs().kinovi_credits, 3256.0);
        assert_eq!(video_ref_1080(20, Some(30)).calculate_enterprise_costs().kinovi_credits, 4070.0);

        // Consumer 85.68 credits/sec.
        assert_eq!(video_ref_1080(20, Some(1)).calculate_consumer_costs().kinovi_credits, 2056.32);
        assert_eq!(video_ref_1080(20, Some(5)).calculate_consumer_costs().kinovi_credits, 2142.0);
        assert_eq!(video_ref_1080(20, Some(10)).calculate_consumer_costs().kinovi_credits, 2570.4);
        assert_eq!(video_ref_1080(20, Some(15)).calculate_consumer_costs().kinovi_credits, 2998.8);
        assert_eq!(video_ref_1080(20, Some(20)).calculate_consumer_costs().kinovi_credits, 3427.2);
        assert_eq!(video_ref_1080(20, Some(30)).calculate_consumer_costs().kinovi_credits, 4284.0);
      }

      #[test]
      fn input_totals_under_four_seconds_clamp_to_four() {
        // Total input clamps to a 4-second minimum: 1s, 3s, and 4s totals
        // all bill 4; 5s bills 5.
        assert_eq!(video_ref_480(30, Some(1)).calculate_consumer_costs().kinovi_credits, 544.0);
        assert_eq!(video_ref_480(30, Some(3)).calculate_consumer_costs().kinovi_credits, 544.0);
        assert_eq!(video_ref_480(30, Some(4)).calculate_consumer_costs().kinovi_credits, 544.0);
        assert_eq!(video_ref_480(30, Some(5)).calculate_consumer_costs().kinovi_credits, 560.0);
      }

      #[test]
      fn missing_total_input_seconds_bills_the_worst_case_maximum() {
        // Unknown input duration with video references attached: assume the
        // 30-second maximum so the estimate never undershoots the charge.
        assert_eq!(video_ref_480(10, None).calculate_consumer_costs().kinovi_credits, 640.0);
        assert_eq!(video_ref_720(10, None).calculate_consumer_costs().kinovi_credits, 1400.0);
        // 1080p: 40 billed seconds at 81.4 enterprise / 85.68 consumer.
        assert_eq!(video_ref_1080(10, None).calculate_enterprise_costs().kinovi_credits, 3256.0);
        assert_eq!(video_ref_1080(10, None).calculate_consumer_costs().kinovi_credits, 3427.2);
      }

      #[test]
      fn input_seconds_clamp_to_max_billed_input_seconds() {
        // 200 input seconds clamp to 30: 30s output + 30 = 60 billed seconds.
        assert_eq!(video_ref_480(30, Some(200)).calculate_consumer_costs().kinovi_credits, 960.0);
        assert_eq!(
          video_ref_480(30, Some(200)).calculate_consumer_costs(),
          video_ref_480(30, Some(MAX_BILLED_INPUT_SECONDS)).calculate_consumer_costs(),
        );
      }

      #[test]
      fn mixed_references_still_use_video_reference_rates() {
        let mut request = video_ref_480(10, Some(5));
        set_reference_urls(
          &mut request,
          Some(vec!["https://example.com/a.png".to_string()]),
          Some(vec!["https://example.com/ref.mp4".to_string()]),
          Some(vec!["https://example.com/ref.wav".to_string()]),
        );
        assert_eq!(request.calculate_consumer_costs().kinovi_credits, 240.0);
      }
    }

    // ── USD cents conversion ──
    //
    // usd_cents = credits × 100 / credits_per_dollar, computed on integer
    // hundredths. Consumer converts at 192.98 credits/$1; enterprise
    // converts at the bulk rate of 243.16 credits/$1.

    mod usd_cents {
      use super::*;

      #[test]
      fn enterprise_cents_480p_5s() {
        // 130 credits; 13000/243.16 = 53.4627 cents.
        let cost = t2v_480(5).calculate_enterprise_costs();
        assert_eq!(cost.usd_cents_rounded_up, 54);
        assert_eq!(cost.usd_cents_rounded_down, 53);
        assert!((cost.usd_cents_fractional - 53.4627).abs() < FLOAT_TOLERANCE);
      }

      #[test]
      fn consumer_cents_480p_5s() {
        // 130 credits; 13000/192.98 = 67.3645 cents.
        let cost = t2v_480(5).calculate_consumer_costs();
        assert_eq!(cost.usd_cents_rounded_up, 68);
        assert_eq!(cost.usd_cents_rounded_down, 67);
        assert!((cost.usd_cents_fractional - 67.3645).abs() < FLOAT_TOLERANCE);
      }

      #[test]
      fn enterprise_cents_720p_30s() {
        // 1770 credits; 177000/243.16 = 727.9158 cents.
        let cost = t2v_720(30).calculate_enterprise_costs();
        assert_eq!(cost.usd_cents_rounded_up, 728);
        assert_eq!(cost.usd_cents_rounded_down, 727);
        assert!((cost.usd_cents_fractional - 727.9158).abs() < FLOAT_TOLERANCE);
      }

      #[test]
      fn enterprise_cents_720p_video_ref_40_billed_seconds() {
        // 35 × 40 = 1400 credits; 140000/243.16 = 575.7526 cents.
        let cost = video_ref_720(30, Some(10)).calculate_enterprise_costs();
        assert_eq!(cost.usd_cents_rounded_up, 576);
        assert_eq!(cost.usd_cents_rounded_down, 575);
        assert!((cost.usd_cents_fractional - 575.7526).abs() < FLOAT_TOLERANCE);
      }

      #[test]
      fn enterprise_cents_1080p_4s() {
        // 544.92 credits; 54492/243.16 = 224.0994 cents.
        let cost = t2v_1080(4).calculate_enterprise_costs();
        assert_eq!(cost.usd_cents_rounded_up, 225);
        assert_eq!(cost.usd_cents_rounded_down, 224);
        assert!((cost.usd_cents_fractional - 224.0994).abs() < FLOAT_TOLERANCE);
      }

      #[test]
      fn consumer_cents_1080p_4s() {
        // 573.6 credits; 57360/192.98 = 297.2329 cents.
        let cost = t2v_1080(4).calculate_consumer_costs();
        assert_eq!(cost.usd_cents_rounded_up, 298);
        assert_eq!(cost.usd_cents_rounded_down, 297);
        assert!((cost.usd_cents_fractional - 297.2329).abs() < FLOAT_TOLERANCE);
      }

      #[test]
      fn enterprise_cents_1080p_30s() {
        // 4086.9 credits; 408690/243.16 = 1680.7452 cents.
        let cost = t2v_1080(30).calculate_enterprise_costs();
        assert_eq!(cost.usd_cents_rounded_up, 1681);
        assert_eq!(cost.usd_cents_rounded_down, 1680);
        assert!((cost.usd_cents_fractional - 1680.7452).abs() < FLOAT_TOLERANCE);
      }

      #[test]
      fn enterprise_cents_1080p_video_ref_30_billed_seconds() {
        // 81.4 × 30 = 2442 credits; 244200/243.16 = 1004.2770 cents.
        let cost = video_ref_1080(20, Some(10)).calculate_enterprise_costs();
        assert_eq!(cost.usd_cents_rounded_up, 1005);
        assert_eq!(cost.usd_cents_rounded_down, 1004);
        assert!((cost.usd_cents_fractional - 1004.2770).abs() < FLOAT_TOLERANCE);
      }

      #[test]
      fn consumer_cents_1080p_video_ref_30_billed_seconds() {
        // 85.68 × 30 = 2570.4 credits; 257040/192.98 = 1331.9515 cents.
        let cost = video_ref_1080(20, Some(10)).calculate_consumer_costs();
        assert_eq!(cost.usd_cents_rounded_up, 1332);
        assert_eq!(cost.usd_cents_rounded_down, 1331);
        assert!((cost.usd_cents_fractional - 1331.9515).abs() < FLOAT_TOLERANCE);
      }
    }

    // ── Relative pricing ──

    mod relative_tests {
      use super::*;
      use crate::test_utils::assert_batch_cost::assert_batch_cost;

      #[test]
      fn all_batches_scale_every_resolution_modality_and_input_duration() {
        let batches = [
          (None, 1),
          (Some(KinoviSeedance2p5BatchCount::One), 1),
          (Some(KinoviSeedance2p5BatchCount::Two), 2),
          (Some(KinoviSeedance2p5BatchCount::Three), 3),
          (Some(KinoviSeedance2p5BatchCount::Four), 4),
          (Some(KinoviSeedance2p5BatchCount::Five), 5),
          (Some(KinoviSeedance2p5BatchCount::Six), 6),
          (Some(KinoviSeedance2p5BatchCount::Seven), 7),
          (Some(KinoviSeedance2p5BatchCount::Eight), 8),
        ];
        for tier in [KinoviPricingTier::Enterprise, KinoviPricingTier::Consumer] {
          for resolution in [
            None,
            Some(KinoviSeedance2p5OutputResolution::FourEightyP),
            Some(KinoviSeedance2p5OutputResolution::SevenTwentyP),
            Some(KinoviSeedance2p5OutputResolution::TenEightyP),
          ] {
            for duration in [4, 5, 30] {
              let text_request = text_to_video_request(duration, resolution);
              let mut keyframe = keyframe_request(duration);
              keyframe.output_resolution = resolution;
              let mut image_audio = text_request.clone();
              set_reference_urls(
                &mut image_audio,
                Some(vec!["https://example.com/image.png".to_string()]),
                None,
                Some(vec!["https://example.com/audio.mp3".to_string()]),
              );
              let mut empty_videos = text_request.clone();
              set_reference_urls(&mut empty_videos, None, Some(vec![]), None);
              let mut requests = vec![
                ("text".to_string(), text_request.clone()),
                ("keyframe".to_string(), keyframe),
                ("image/audio".to_string(), image_audio),
                ("empty videos".to_string(), empty_videos),
              ];
              // Include unknown/zero inputs, the minimum, ordinary input, the
              // maximum, and values clamped at either end of the billed range.
              for input_seconds in [None, Some(0), Some(1), Some(4), Some(10), Some(30), Some(31), Some(u8::MAX)] {
                let mut request = text_request.clone();
                set_reference_urls(&mut request, None, Some(vec!["https://example.com/ref.mp4".to_string()]), None);
                request.total_input_seconds = input_seconds;
                requests.push((format!("video input={input_seconds:?}"), request));
              }
              for (modality, mut request) in requests {
                let single = request.calculate_costs(tier);
                for (batch, count) in batches {
                  request.batch_count = batch;
                  let batched = request.calculate_costs(tier);
                  let context = format!("{tier:?} {resolution:?} {duration}s {batch:?} {modality}");
                  assert_batch_cost(single, batched, count, tier, &context);
                }
              }
            }
          }
        }
      }

      #[test]
      fn batch_multiplies_output_and_reference_input_costs() {
        let mut request = video_ref_480(5, Some(10));
        request.batch_count = Some(KinoviSeedance2p5BatchCount::Eight);
        assert_eq!(request.calculate_consumer_costs().kinovi_credits, 1920.0);
        assert_eq!(request.calculate_enterprise_costs().kinovi_credits, 1920.0);
        assert!(matches!(to_raw_request(request).batch_count, KinoviBatchCountRaw::Eight));
      }

      #[test]
      fn video_reference_rate_is_cheaper_per_second() {
        // 16 < 26, 35 < 59, and 81.4 < 136.23: the with-references rate is
        // lower per billed second (the input seconds are where the money
        // goes). Compare at the minimum billed input (1s clamps to 4) —
        // unknown or zero input assumes the 30-second maximum, which would
        // swamp the rate comparison.
        assert!(video_ref_480(10, Some(1)).calculate_consumer_costs().kinovi_credits
          < t2v_480(10).calculate_consumer_costs().kinovi_credits);
        assert!(video_ref_720(10, Some(1)).calculate_consumer_costs().kinovi_credits
          < t2v_720(10).calculate_consumer_costs().kinovi_credits);
        assert!(video_ref_1080(10, Some(1)).calculate_enterprise_costs().kinovi_credits
          < t2v_1080(10).calculate_enterprise_costs().kinovi_credits);
      }

      #[test]
      fn enterprise_1080p_is_cheaper_than_consumer() {
        for request in [t2v_1080(10), video_ref_1080(10, Some(10))] {
          let consumer = request.calculate_consumer_costs();
          let enterprise = request.calculate_enterprise_costs();
          assert!(enterprise.kinovi_credits < consumer.kinovi_credits);
          assert!(enterprise.usd_cents_fractional < consumer.usd_cents_fractional);
        }
      }

      #[test]
      fn cost_scales_linearly_with_duration() {
        for duration in [1u8, 5, 10, 30] {
          assert_eq!(
            t2v_480(duration).calculate_consumer_costs().kinovi_credits,
            (26 * u64::from(duration)) as f64,
          );
        }
      }
    }

    // ── Aspect ratio doesn't affect cost ──

    #[test]
    fn aspect_ratio_does_not_affect_credits() {
      let baseline = t2v_720(10).calculate_consumer_costs().kinovi_credits;

      let ratios = [
        KinoviSeedance2p5AspectRatio::Landscape16x9,
        KinoviSeedance2p5AspectRatio::UltraWide21x9,
        KinoviSeedance2p5AspectRatio::Portrait9x16,
        KinoviSeedance2p5AspectRatio::Square1x1,
        KinoviSeedance2p5AspectRatio::Standard4x3,
        KinoviSeedance2p5AspectRatio::Portrait3x4,
      ];

      for ar in &ratios {
        let mut request = t2v_720(10);
        request.modality = KinoviSeedance2p5Modality::Reference {
          aspect_ratio: Some(*ar),
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
        };
        assert_eq!(
          request.calculate_consumer_costs().kinovi_credits, baseline,
          "Aspect ratio {:?} should not change credits from baseline {}", ar, baseline,
        );
      }
    }

    // ── Helpers ──

    fn text_to_video_request(
      duration_seconds: u8,
      output_resolution: Option<KinoviSeedance2p5OutputResolution>,
    ) -> GenerateSeedance2p5Request {
      GenerateSeedance2p5Request {
        batch_count: None,
        prompt: String::new(),
        modality: KinoviSeedance2p5Modality::Reference {
          aspect_ratio: None,
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
        },
        output_resolution,
        duration_seconds,
        total_input_seconds: None,
        use_face_blur_hack: None,
        maybe_bitrate: None,
        maybe_output_format: None,
        maybe_generate_audio: None,
      }
    }

    fn t2v_480(duration: u8) -> GenerateSeedance2p5Request {
      text_to_video_request(duration, Some(KinoviSeedance2p5OutputResolution::FourEightyP))
    }

    fn t2v_720(duration: u8) -> GenerateSeedance2p5Request {
      text_to_video_request(duration, None)
    }

    fn t2v_1080(duration: u8) -> GenerateSeedance2p5Request {
      text_to_video_request(duration, Some(KinoviSeedance2p5OutputResolution::TenEightyP))
    }

    fn keyframe_request(duration: u8) -> GenerateSeedance2p5Request {
      GenerateSeedance2p5Request {
        batch_count: None,
        prompt: String::new(),
        modality: KinoviSeedance2p5Modality::Keyframe {
          start_frame_url: "https://example.com/start.png".to_string(),
          end_frame_url: Some("https://example.com/end.png".to_string()),
        },
        output_resolution: None,
        duration_seconds: duration,
        total_input_seconds: None,
        use_face_blur_hack: None,
        maybe_bitrate: None,
        maybe_output_format: None,
        maybe_generate_audio: None,
      }
    }

    fn video_ref_480(duration: u8, total_input_seconds: Option<u8>) -> GenerateSeedance2p5Request {
      let mut request = t2v_480(duration);
      set_reference_urls(&mut request, None, Some(vec!["https://example.com/ref.mp4".to_string()]), None);
      request.total_input_seconds = total_input_seconds;
      request
    }

    fn video_ref_720(duration: u8, total_input_seconds: Option<u8>) -> GenerateSeedance2p5Request {
      let mut request = t2v_720(duration);
      set_reference_urls(&mut request, None, Some(vec!["https://example.com/ref.mp4".to_string()]), None);
      request.total_input_seconds = total_input_seconds;
      request
    }

    fn video_ref_1080(duration: u8, total_input_seconds: Option<u8>) -> GenerateSeedance2p5Request {
      let mut request = t2v_1080(duration);
      set_reference_urls(&mut request, None, Some(vec!["https://example.com/ref.mp4".to_string()]), None);
      request.total_input_seconds = total_input_seconds;
      request
    }

    fn set_reference_urls(
      request: &mut GenerateSeedance2p5Request,
      images: Option<Vec<String>>,
      videos: Option<Vec<String>>,
      audio: Option<Vec<String>>,
    ) {
      let KinoviSeedance2p5Modality::Reference {
        reference_image_urls,
        reference_video_urls,
        reference_audio_urls,
        ..
      } = &mut request.modality else {
        panic!("set_reference_urls requires the Reference modality");
      };
      *reference_image_urls = images;
      *reference_video_urls = videos;
      *reference_audio_urls = audio;
    }
  }

  mod raw_mapping_tests {
    use super::*;

    #[test]
    fn keyframe_maps_frames_and_no_references() {
      let raw = to_raw_request(GenerateSeedance2p5Request {
        batch_count: None,
        prompt: "Car drives into the sunset".to_string(),
        modality: KinoviSeedance2p5Modality::Keyframe {
          start_frame_url: "https://example.com/start.png".to_string(),
          end_frame_url: Some("https://example.com/end.png".to_string()),
        },
        output_resolution: Some(KinoviSeedance2p5OutputResolution::FourEightyP),
        duration_seconds: 8,
        total_input_seconds: None,
        use_face_blur_hack: None,
        maybe_bitrate: None,
        maybe_output_format: None,
        maybe_generate_audio: None,
      });

      assert!(matches!(raw.model_type, KinoviModelTypeRaw::Seedance2p5));
      assert!(raw.bitrate.is_none());
      assert_eq!(raw.start_frame_url.as_deref(), Some("https://example.com/start.png"));
      assert_eq!(raw.end_frame_url.as_deref(), Some("https://example.com/end.png"));
      assert!(raw.reference_image_urls.is_none());
      assert!(raw.reference_video_urls.is_none());
      assert!(raw.reference_audio_urls.is_none());
    }

    #[test]
    fn reference_maps_references_and_no_frames() {
      let raw = to_raw_request(GenerateSeedance2p5Request {
        batch_count: None,
        prompt: "The t-rex @image1 eats the banana".to_string(),
        modality: KinoviSeedance2p5Modality::Reference {
          aspect_ratio: Some(KinoviSeedance2p5AspectRatio::UltraWide21x9),
          reference_image_urls: Some(vec!["https://example.com/a.png".to_string()]),
          reference_video_urls: Some(vec!["https://example.com/ref.mp4".to_string()]),
          reference_audio_urls: Some(vec!["https://example.com/ref.wav".to_string()]),
        },
        output_resolution: None,
        duration_seconds: 8,
        total_input_seconds: Some(7),
        use_face_blur_hack: None,
        maybe_bitrate: Some(KinoviSeedance2p5Bitrate::High),
        maybe_output_format: Some(KinoviSeedance2p5OutputFormat::Mov),
        maybe_generate_audio: Some(false),
      });

      assert!(matches!(raw.aspect_ratio, KinoviAspectRatioRaw::UltraWide21x9));
      assert!(matches!(raw.bitrate, Some(KinoviBitrateRaw::High)));
      assert!(matches!(raw.maybe_output_format, Some(KinoviOutputFormatRaw::Mov)));
      assert_eq!(raw.maybe_generate_audio, Some(false));
      assert!(raw.start_frame_url.is_none());
      assert!(raw.end_frame_url.is_none());
      assert_eq!(raw.reference_image_urls.as_deref().map(|urls| urls.len()), Some(1));
      assert_eq!(raw.reference_video_urls.as_deref().map(|urls| urls.len()), Some(1));
      assert_eq!(raw.reference_audio_urls.as_deref().map(|urls| urls.len()), Some(1));
    }

    #[test]
    fn resolution_1080p_maps_to_raw_1080p() {
      let raw = to_raw_request(GenerateSeedance2p5Request {
        batch_count: None,
        prompt: "A car driving on the beach".to_string(),
        modality: KinoviSeedance2p5Modality::Reference {
          aspect_ratio: None,
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
        },
        output_resolution: Some(KinoviSeedance2p5OutputResolution::TenEightyP),
        duration_seconds: 4,
        total_input_seconds: None,
        use_face_blur_hack: None,
        maybe_bitrate: None,
        maybe_output_format: None,
        maybe_generate_audio: None,
      });

      assert!(matches!(raw.output_resolution, Some(KinoviOutputResolutionRaw::TenEightyP)));
    }

    #[test]
    fn reference_aspect_ratio_defaults_to_16x9() {
      let raw = to_raw_request(GenerateSeedance2p5Request {
        batch_count: None,
        prompt: "Lightning hits a building".to_string(),
        modality: KinoviSeedance2p5Modality::Reference {
          aspect_ratio: None,
          reference_image_urls: None,
          reference_video_urls: None,
          reference_audio_urls: None,
        },
        output_resolution: None,
        duration_seconds: 5,
        total_input_seconds: None,
        use_face_blur_hack: None,
        maybe_bitrate: None,
        maybe_output_format: None,
        maybe_generate_audio: None,
      });

      assert!(matches!(raw.aspect_ratio, KinoviAspectRatioRaw::Landscape16x9));
    }
  }

  mod real_requests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_text_to_video_480p() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let result = generate_seedance_2p5(GenerateSeedance2p5Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p5Request {
          batch_count: None,
          prompt: "Lightning hits a building".to_string(),
          modality: KinoviSeedance2p5Modality::Reference {
            aspect_ratio: Some(KinoviSeedance2p5AspectRatio::Landscape16x9),
            reference_image_urls: None,
            reference_video_urls: None,
            reference_audio_urls: None,
          },
          output_resolution: Some(KinoviSeedance2p5OutputResolution::FourEightyP),
          duration_seconds: 5,
          total_input_seconds: None,
          use_face_blur_hack: None,
          maybe_bitrate: None,
          maybe_output_format: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("t2v 480p — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert!(!result.order_id.is_empty());
      assert_eq!(1, 2, "Inspect output above");
      Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_text_to_video_1080p() -> AnyhowResult<()> {
      // Mirrors external/requests/sites/kinovi.ai/2026-08-17-seedance2p5-1080p/
      // 1_seedance2p5_1080p.txt.
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let result = generate_seedance_2p5(GenerateSeedance2p5Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p5Request {
          batch_count: None,
          prompt: "A car driving on the beach".to_string(),
          modality: KinoviSeedance2p5Modality::Reference {
            aspect_ratio: Some(KinoviSeedance2p5AspectRatio::Landscape16x9),
            reference_image_urls: None,
            reference_video_urls: None,
            reference_audio_urls: None,
          },
          output_resolution: Some(KinoviSeedance2p5OutputResolution::TenEightyP),
          duration_seconds: 4,
          total_input_seconds: None,
          use_face_blur_hack: None,
          maybe_bitrate: None,
          maybe_output_format: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("t2v 1080p — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert!(!result.order_id.is_empty());
      assert_eq!(1, 2, "Inspect output above");
      Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_keyframe_to_video_adaptive_1080p() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let result = generate_seedance_2p5(GenerateSeedance2p5Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p5Request {
          batch_count: None,
          prompt: "Car drives into the sunset".to_string(),
          modality: KinoviSeedance2p5Modality::Keyframe {
            start_frame_url: "https://static.seedance2-pro.com/materials/20260807/1786128486168-a2bf6132.png".to_string(),
            end_frame_url: Some("https://static.seedance2-pro.com/materials/20260807/1786128493608-d708413a.png".to_string()),
          },
          output_resolution: Some(KinoviSeedance2p5OutputResolution::TenEightyP),
          duration_seconds: 4,
          total_input_seconds: None,
          use_face_blur_hack: None,
          maybe_bitrate: None,
          maybe_output_format: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("keyframe 1080p — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert!(!result.order_id.is_empty());
      assert_eq!(1, 2, "Inspect output above");
      Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_keyframe_to_video_adaptive_480p() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let result = generate_seedance_2p5(GenerateSeedance2p5Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p5Request {
          batch_count: None,
          prompt: "Car drives into the sunset".to_string(),
          modality: KinoviSeedance2p5Modality::Keyframe {
            start_frame_url: "https://static.seedance2-pro.com/materials/20260807/1786128486168-a2bf6132.png".to_string(),
            end_frame_url: Some("https://static.seedance2-pro.com/materials/20260807/1786128493608-d708413a.png".to_string()),
          },
          output_resolution: Some(KinoviSeedance2p5OutputResolution::FourEightyP),
          duration_seconds: 8,
          total_input_seconds: None,
          use_face_blur_hack: None,
          maybe_bitrate: None,
          maybe_output_format: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("keyframe 480p — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert!(!result.order_id.is_empty());
      assert_eq!(1, 2, "Inspect output above");
      Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_reference_to_video_480p() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Trace);
      let session = test_session()?;
      let result = generate_seedance_2p5(GenerateSeedance2p5Args {
        session: &session,
        host_override: None,
        request: GenerateSeedance2p5Request {
          batch_count: None,
          prompt: "The t-rex @image1 eats the banana @image2".to_string(),
          modality: KinoviSeedance2p5Modality::Reference {
            aspect_ratio: Some(KinoviSeedance2p5AspectRatio::UltraWide21x9),
            reference_image_urls: Some(vec![
              "https://static.seedance2-pro.com/materials/20260807/1786128628611-d6ca7afd.png".to_string(),
              "https://static.seedance2-pro.com/materials/20260807/1786128638331-5395ff1c.jpg".to_string(),
            ]),
            reference_video_urls: None,
            reference_audio_urls: None,
          },
          output_resolution: Some(KinoviSeedance2p5OutputResolution::FourEightyP),
          duration_seconds: 8,
          total_input_seconds: None,
          use_face_blur_hack: None,
          maybe_bitrate: None,
          maybe_output_format: None,
          maybe_generate_audio: None,
        },
      }).await?;
      println!("reference 480p — task_id={}, order_id={}", result.task_id, result.order_id);
      assert!(!result.task_id.is_empty());
      assert!(!result.order_id.is_empty());
      assert_eq!(1, 2, "Inspect output above");
      Ok(())
    }

    fn test_session() -> AnyhowResult<KinoviWebSession> {
      let cookies = get_test_cookies()?;
      Ok(KinoviWebSession::from_cookies_string(cookies))
    }
  }
}
