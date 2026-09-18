//! Shared options and wire format for Wan 3.0 and Wan 3.0 Prime.
//!
//! Schema: https://kinovi.ai/models/wan3-ref-to-video (checked 2026-09-18).
//! All modes bill only OUTPUT seconds; reference media does not add a surcharge.
//!
//! Output accepts any whole-second duration from 2 through 30. Reference
//! videos must each be 1-15s and total at most 15s; their total plus output
//! duration must not exceed 30s. Thus 10s of video references allows at most
//! 20s of output. Audio and image references do not consume this video budget.
//! The optional measured reference duration enables local validation; otherwise
//! Kinovi checks actual file durations. This client does not probe media URLs.
//!
//! Pricing sources (all three modalities share each family's rates):
//! - https://kinovi.ai/models/wan3-text-to-video
//! - https://kinovi.ai/models/wan3-prime-ref-to-video
//! Pricing maintenance and the September 23 promotion: `costs_wan_3p0.md`
//! at the crate root.

use serde_derive::Serialize;

use crate::error::kinovi_web_client_error::KinoviWebClientError;
use crate::error::kinovi_web_error::KinoviWebError;
use crate::pricing::cost::kinovi_fractional_generation_cost::KinoviFractionalGenerationCost;
use crate::pricing::kinovi_pricing_tier::KinoviPricingTier;

pub(super) const BUSINESS_TYPE: &str = "wan3-video-generation";
pub const MAX_SEED: u32 = 2_147_483_647;
pub const MIN_DURATION_SECONDS: u8 = 2;
pub const MAX_DURATION_SECONDS: u8 = 30;
pub const MAX_REFERENCE_VIDEO_DURATION_MILLIS: u32 = 15_000;

// Hundredths of a credit per OUTPUT second, as observed 2026-09-18.
// Regular Wan intentionally uses the promotional rates (30% off until 2026-09-23).
// TODO(2026-09-24): Recheck live pricing using costs_wan_3p0.md at the crate root.
// That note records advertised replacement rates, precision needs, and test updates.
// Keep these promotional rates until a reviewed update; no automatic date switch.
// Prime 480p is 18.86/s, so 30s costs 565.8, not 568.8 credits.
// No separate enterprise credit discount has been supplied for these models.
const WAN_CREDIT_HUNDREDTHS_PER_SECOND: [u64; 3] = [998, 1996, 3991];
const PRIME_CREDIT_HUNDREDTHS_PER_SECOND: [u64; 3] = [1886, 3883, 7766];

/// Shared by all six Wan operations. Defaults to 16:9.
/// `adaptive` is REST-only for reference-to-video and is not a Studio option.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KinoviWan3p0AspectRatio {
  #[default]
  Landscape16x9,
  Portrait9x16,
  Landscape4x3,
  Portrait3x4,
  Square1x1,
}

/// Shared by regular and Prime Wan. Defaults to 720p, pinned on the wire.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KinoviWan3p0OutputResolution {
  FourEightyP,
  #[default]
  SevenTwentyP,
  TenEightyP,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Wan3p0Model {
  TextToVideo,
  ImageToVideo,
  RefToVideo,
  PrimeTextToVideo,
  PrimeImageToVideo,
  PrimeRefToVideo,
}

/// Consumer tRPC shape, verified against the 2026-09-18 captures. Images
/// use `uploadedUrls`; videos use BOTH `videoUrl` and `videoUrls`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Wan3p0ApiParams {
  model: &'static str,
  content_mode: &'static str,
  face_blur_mode: &'static str,
  prompt: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  mode: Option<&'static str>,
  aspect_ratio: &'static str,
  duration: String,
  output_resolution: &'static str,
  #[serde(skip_serializing_if = "Option::is_none")]
  seed: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  audio: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(super) uploaded_urls: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  video_url: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(super) video_urls: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(super) audio_urls: Option<Vec<String>>,
}

impl Wan3p0Model {
  pub(super) fn calculate_costs(
    self,
    resolution: Option<KinoviWan3p0OutputResolution>,
    duration_seconds: u8,
    tier: KinoviPricingTier,
  ) -> KinoviFractionalGenerationCost {
    let rates = match self {
      Self::TextToVideo | Self::ImageToVideo | Self::RefToVideo => WAN_CREDIT_HUNDREDTHS_PER_SECOND,
      Self::PrimeTextToVideo | Self::PrimeImageToVideo | Self::PrimeRefToVideo => PRIME_CREDIT_HUNDREDTHS_PER_SECOND,
    };
    let index = match resolution.unwrap_or_default() {
      KinoviWan3p0OutputResolution::FourEightyP => 0,
      KinoviWan3p0OutputResolution::SevenTwentyP => 1,
      KinoviWan3p0OutputResolution::TenEightyP => 2,
    };
    let credit_hundredths = rates[index] * u64::from(duration_seconds);
    tier.cost_from_credits(credit_hundredths as f64 / 100.0)
  }
}

impl Wan3p0ApiParams {
  pub(super) fn new(
    model: Wan3p0Model,
    prompt: String,
    aspect_ratio: Option<KinoviWan3p0AspectRatio>,
    resolution: Option<KinoviWan3p0OutputResolution>,
    duration_seconds: u8,
    seed: Option<u32>,
    audio: Option<bool>,
  ) -> Result<Self, KinoviWebError> {
    if !(MIN_DURATION_SECONDS..=MAX_DURATION_SECONDS).contains(&duration_seconds) {
      return Err(invalid_field("duration_seconds", duration_seconds, "must be between 2 and 30"));
    }
    if let Some(seed) = seed {
      if seed > MAX_SEED {
        return Err(invalid_field("maybe_seed", seed, "must be between 0 and 2147483647"));
      }
    }
    if prompt.chars().count() > 20_000 {
      return Err(invalid_field("prompt", prompt.chars().count(), "must not exceed 20000 characters"));
    }

    let (model, mode) = match model {
      Wan3p0Model::TextToVideo => ("wan3.0-text-to-video", None),
      Wan3p0Model::ImageToVideo => ("wan3.0-image-to-video", Some("keyframe")),
      Wan3p0Model::RefToVideo => ("wan3.0-ref-to-video", Some("reference")),
      Wan3p0Model::PrimeTextToVideo => ("wan3.0-prime-text-to-video", None),
      Wan3p0Model::PrimeImageToVideo => ("wan3.0-prime-image-to-video", Some("keyframe")),
      Wan3p0Model::PrimeRefToVideo => ("wan3.0-prime-ref-to-video", Some("reference")),
    };
    Ok(Self {
      model,
      content_mode: "normal",
      face_blur_mode: "off",
      prompt,
      mode,
      aspect_ratio: match aspect_ratio.unwrap_or_default() {
        KinoviWan3p0AspectRatio::Landscape16x9 => "16:9",
        KinoviWan3p0AspectRatio::Portrait9x16 => "9:16",
        KinoviWan3p0AspectRatio::Landscape4x3 => "4:3",
        KinoviWan3p0AspectRatio::Portrait3x4 => "3:4",
        KinoviWan3p0AspectRatio::Square1x1 => "1:1",
      },
      duration: format!("{duration_seconds}s"),
      output_resolution: match resolution.unwrap_or_default() {
        KinoviWan3p0OutputResolution::FourEightyP => "480p",
        KinoviWan3p0OutputResolution::SevenTwentyP => "720p",
        KinoviWan3p0OutputResolution::TenEightyP => "1080p",
      },
      seed,
      audio,
      uploaded_urls: None,
      video_url: None,
      video_urls: None,
      audio_urls: None,
    })
  }

  pub(super) fn validate_media(
    &mut self,
    duration_seconds: u8,
    maybe_total_reference_video_duration_millis: Option<u32>,
  ) -> Result<(), KinoviWebError> {
    self.uploaded_urls = self.uploaded_urls.take().filter(|urls| !urls.is_empty());
    self.video_urls = self.video_urls.take().filter(|urls| !urls.is_empty());
    self.audio_urls = self.audio_urls.take().filter(|urls| !urls.is_empty());
    let image_count = self.uploaded_urls.as_ref().map_or(0, Vec::len);
    let video_count = self.video_urls.as_ref().map_or(0, Vec::len);
    let audio_count = self.audio_urls.as_ref().map_or(0, Vec::len);
    let max_images = if self.mode == Some("keyframe") { 2 } else { 10 };
    for (field, count, maximum) in [
      ("maybe_reference_image_urls", image_count, max_images),
      ("maybe_reference_video_urls", video_count, 5),
      ("maybe_reference_audio_urls", audio_count, 5),
    ] {
      if count > maximum {
        return Err(invalid_field(field, count, format!("at most {maximum} items are supported")));
      }
    }
    if self.mode == Some("keyframe") && image_count == 0 {
      return Err(invalid_field("maybe_start_frame_url", "", "image-to-video requires a start frame"));
    }
    if self.prompt.trim().is_empty() && image_count + video_count + audio_count == 0 {
      return Err(invalid_field("prompt", "", "a prompt is required when no media is supplied"));
    }
    for urls in [&self.uploaded_urls, &self.video_urls, &self.audio_urls].into_iter().flatten() {
      if urls.iter().any(|url| url.trim().is_empty()) {
        return Err(invalid_field("media_url", "", "media URLs must not be empty"));
      }
    }
    if video_count > 0 {
      // Without measured metadata, only enforce the known lower bound:
      // each reference video must be at least 1s. Kinovi validates actual
      // media durations; the client does not download/probe URLs here.
      let minimum_millis = video_count as u32 * 1_000;
      let input_millis = maybe_total_reference_video_duration_millis.unwrap_or(minimum_millis);
      if !(minimum_millis..=MAX_REFERENCE_VIDEO_DURATION_MILLIS).contains(&input_millis) {
        return Err(invalid_field(
          "maybe_total_reference_video_duration_millis",
          input_millis,
          "reference videos must each be 1-15s and total at most 15s",
        ));
      }
      if input_millis + u32::from(duration_seconds) * 1_000 > 30_000 {
        return Err(invalid_field(
          "duration_seconds",
          duration_seconds,
          "reference-video duration plus output duration must not exceed 30s",
        ));
      }
      self.video_url = self.video_urls.as_ref().and_then(|urls| urls.first()).cloned();
    }
    Ok(())
  }
}

pub(super) fn invalid_field(field: &'static str, value: impl ToString, reason: impl Into<String>) -> KinoviWebError {
  KinoviWebClientError::InvalidRequestField {
    field,
    raw_value: value.to_string(),
    reason: reason.into(),
  }
  .into()
}
