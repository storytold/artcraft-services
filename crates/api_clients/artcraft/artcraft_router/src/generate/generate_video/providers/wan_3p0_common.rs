//! Request planning shared by the ArtCraft and Kinovi Wan providers.

use crate::api::audio_list_ref::AudioListRef;
use crate::api::character_list_ref::CharacterListRef;
use crate::api::image_list_ref::ImageListRef;
use crate::api::router_aspect_ratio::RouterAspectRatio;
use crate::api::router_resolution::RouterResolution;
use crate::api::video_list_ref::VideoListRef;
use crate::client::request_mismatch_mitigation_strategy::RequestMismatchMitigationStrategy;
use crate::errors::artcraft_router_error::ArtcraftRouterError;
use crate::errors::client_error::ClientError;
use crate::generate::generate_video::generate_video_request_builder::GenerateVideoRequestBuilder;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wan3p0Modality {
  Text,
  Image,
  Reference,
}

/// Materialize the same settings for execution and ArtCraft cost estimation.
pub fn normalize_wan_3p0_builder(
  mut builder: GenerateVideoRequestBuilder,
) -> Result<GenerateVideoRequestBuilder, ArtcraftRouterError> {
  let strict = matches!(builder.request_mismatch_mitigation_strategy, RequestMismatchMitigationStrategy::ErrorOut);
  builder.reference_images = builder.reference_images.filter(|images| match images {
    ImageListRef::MediaFileTokens(items) => !items.is_empty(),
    ImageListRef::Urls(items) => !items.is_empty(),
  });
  builder.reference_videos = builder.reference_videos.filter(|videos| match videos {
    VideoListRef::MediaFileTokens(items) => !items.is_empty(),
    VideoListRef::Urls(items) => !items.is_empty(),
  });
  builder.reference_audio = builder.reference_audio.filter(|audio| match audio {
    AudioListRef::MediaFileTokens(items) => !items.is_empty(),
    AudioListRef::Urls(items) => !items.is_empty(),
  });
  builder.reference_character_tokens = builder.reference_character_tokens.filter(|characters| match characters {
    CharacterListRef::CharacterTokens(items) => !items.is_empty(),
  });
  let has_keyframes = builder.start_frame.is_some() || builder.end_frame.is_some();
  if has_keyframes && modality(&builder) == Wan3p0Modality::Reference {
    return Err(unsupported("reference_images", "Wan cannot combine keyframes with reference media"));
  }
  if builder.start_frame.is_none() && builder.end_frame.is_some() {
    if strict {
      return Err(unsupported("end_frame", "Wan requires a starting frame"));
    }
    // Kinovi requires the first keyframe. With adaptation enabled, use the
    // only supplied image as that keyframe, preserving image-to-video routing.
    builder.start_frame = builder.end_frame.take();
  }
  if builder.reference_character_tokens.is_some() {
    return Err(unsupported("reference_character_tokens", "Wan character references are not supported"));
  }
  let image_count = builder.reference_images.as_ref().map_or(0, |images| match images {
    ImageListRef::MediaFileTokens(items) => items.len(),
    ImageListRef::Urls(items) => items.len(),
  });
  let video_count = builder.reference_videos.as_ref().map_or(0, |videos| match videos {
    VideoListRef::MediaFileTokens(items) => items.len(),
    VideoListRef::Urls(items) => items.len(),
  });
  let audio_count = builder.reference_audio.as_ref().map_or(0, |audio| match audio {
    AudioListRef::MediaFileTokens(items) => items.len(),
    AudioListRef::Urls(items) => items.len(),
  });
  for (field, count, limit) in [("reference_images", image_count, 10), ("reference_videos", video_count, 5), ("reference_audio", audio_count, 5)] {
    if count > limit {
      return Err(unsupported(field, format!("Wan supports at most {limit} references")));
    }
  }
  let duration = builder.duration_seconds.unwrap_or(5);
  if strict && !(2..=30).contains(&duration) {
    return Err(unsupported("duration_seconds", "Wan supports 2 through 30 seconds"));
  }
  builder.duration_seconds = Some(duration.clamp(2, 30));
  match builder.video_batch_count.unwrap_or(1) {
    0 => return Err(ArtcraftRouterError::Client(ClientError::UserRequestedZeroGenerations)),
    1 => {},
    _ if strict => return Err(unsupported("video_batch_count", "Wan supports one video per request")),
    _ => {},
  }
  builder.video_batch_count = Some(1);
  builder.aspect_ratio = Some(match builder.aspect_ratio {
    None | Some(RouterAspectRatio::Auto | RouterAspectRatio::Auto2k | RouterAspectRatio::Auto3k | RouterAspectRatio::Auto4k | RouterAspectRatio::Wide) => RouterAspectRatio::WideSixteenByNine,
    Some(RouterAspectRatio::Tall) => RouterAspectRatio::TallNineBySixteen,
    Some(RouterAspectRatio::SquareHd) => RouterAspectRatio::Square,
    Some(ratio @ (RouterAspectRatio::WideSixteenByNine | RouterAspectRatio::TallNineBySixteen | RouterAspectRatio::WideFourByThree | RouterAspectRatio::TallThreeByFour | RouterAspectRatio::Square)) => ratio,
    Some(ratio) if strict => return Err(unsupported("aspect_ratio", format!("{ratio:?}"))),
    Some(RouterAspectRatio::WideTwentyOneByNine) => RouterAspectRatio::WideSixteenByNine,
    Some(RouterAspectRatio::WideFiveByFour | RouterAspectRatio::WideThreeByTwo) => RouterAspectRatio::WideFourByThree,
    Some(RouterAspectRatio::TallFourByFive | RouterAspectRatio::TallTwoByThree) => RouterAspectRatio::TallThreeByFour,
    Some(RouterAspectRatio::TallNineByTwentyOne) => RouterAspectRatio::TallNineBySixteen,
  });
  builder.resolution = Some(match builder.resolution {
    None => RouterResolution::SevenTwentyP,
    Some(resolution @ (RouterResolution::FourEightyP | RouterResolution::SevenTwentyP | RouterResolution::TenEightyP)) => resolution,
    Some(resolution) if strict => return Err(unsupported("resolution", format!("{resolution:?}"))),
    Some(RouterResolution::HalfK) => match builder.request_mismatch_mitigation_strategy {
      RequestMismatchMitigationStrategy::PayMoreUpgrade => RouterResolution::SevenTwentyP,
      _ => RouterResolution::FourEightyP,
    },
    Some(RouterResolution::OneK) => match builder.request_mismatch_mitigation_strategy {
      RequestMismatchMitigationStrategy::PayLessDowngrade => RouterResolution::SevenTwentyP,
      _ => RouterResolution::TenEightyP,
    },
    Some(_) => RouterResolution::TenEightyP,
  });
  if video_count > 0 {
    // Without measurements, each reference still consumes at least one second.
    let seconds = builder.total_reference_video_input_seconds.unwrap_or(video_count as u16);
    if seconds < video_count as u16 || seconds > 15 || seconds + builder.duration_seconds.unwrap() > 30 {
      return Err(unsupported("duration_seconds", "Wan reference videos must total at most 15s, and reference plus output duration must not exceed 30s"));
    }
  }
  Ok(builder)
}

pub fn modality(builder: &GenerateVideoRequestBuilder) -> Wan3p0Modality {
  if builder.reference_images.is_some() || builder.reference_videos.is_some() || builder.reference_audio.is_some() {
    Wan3p0Modality::Reference
  } else if builder.start_frame.is_some() || builder.end_frame.is_some() {
    Wan3p0Modality::Image
  } else {
    Wan3p0Modality::Text
  }
}

fn unsupported(field: &'static str, value: impl Into<String>) -> ArtcraftRouterError {
  ArtcraftRouterError::Client(ClientError::ModelDoesNotSupportOption { field, value: value.into() })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::image_ref::ImageRef;
  use crate::api::router_video_model::RouterVideoModel;

  fn builder() -> GenerateVideoRequestBuilder {
    GenerateVideoRequestBuilder {
      model: RouterVideoModel::Wan3p0,
      request_mismatch_mitigation_strategy: RequestMismatchMitigationStrategy::PayMoreUpgrade,
      ..Default::default()
    }
  }

  #[test]
  fn modality_selects_text_keyframe_or_reference_endpoint() {
    let text = builder();
    assert_eq!(modality(&text), Wan3p0Modality::Text);

    let keyframe = GenerateVideoRequestBuilder {
      start_frame: Some(ImageRef::Url("https://example.com/start.png".to_string())),
      ..text.clone()
    };
    assert_eq!(modality(&keyframe), Wan3p0Modality::Image);

    let reference = GenerateVideoRequestBuilder {
      reference_images: Some(ImageListRef::Urls(vec!["https://example.com/ref.png".to_string()])),
      ..text
    };
    assert_eq!(modality(&reference), Wan3p0Modality::Reference);
  }

  #[test]
  fn normalization_defaults_and_adapts_settings_for_kinovi() {
    let normalized = normalize_wan_3p0_builder(GenerateVideoRequestBuilder {
      model: RouterVideoModel::Wan3p0Prime,
      duration_seconds: Some(99),
      video_batch_count: Some(4),
      aspect_ratio: Some(RouterAspectRatio::WideTwentyOneByNine),
      resolution: Some(RouterResolution::FourK),
      request_mismatch_mitigation_strategy: RequestMismatchMitigationStrategy::PayMoreUpgrade,
      ..Default::default()
    })
    .unwrap();

    assert_eq!(normalized.duration_seconds, Some(30));
    assert_eq!(normalized.video_batch_count, Some(1));
    assert!(matches!(normalized.aspect_ratio, Some(RouterAspectRatio::WideSixteenByNine)));
    assert!(matches!(normalized.resolution, Some(RouterResolution::TenEightyP)));
  }

  #[test]
  fn strict_mode_rejects_unsupported_shape() {
    let result = normalize_wan_3p0_builder(GenerateVideoRequestBuilder {
      resolution: Some(RouterResolution::FourK),
      request_mismatch_mitigation_strategy: RequestMismatchMitigationStrategy::ErrorOut,
      ..builder()
    });
    assert!(result.is_err());
  }

  #[test]
  fn reference_video_budget_uses_measured_total_when_available() {
    let result = normalize_wan_3p0_builder(GenerateVideoRequestBuilder {
      duration_seconds: Some(20),
      total_reference_video_input_seconds: Some(11),
      reference_videos: Some(VideoListRef::Urls(vec!["https://example.com/ref.mp4".to_string()])),
      ..builder()
    });
    assert!(result.is_err());

    let result = normalize_wan_3p0_builder(GenerateVideoRequestBuilder {
      duration_seconds: Some(20),
      total_reference_video_input_seconds: Some(10),
      reference_videos: Some(VideoListRef::Urls(vec!["https://example.com/ref.mp4".to_string()])),
      ..builder()
    });
    assert!(result.is_ok());
  }
}
