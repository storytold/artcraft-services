use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::timeout;

use super::generate_wan_3p0_image_to_video::*;
use super::generate_wan_3p0_prime_image_to_video::*;
use super::generate_wan_3p0_prime_ref_to_video::*;
use super::generate_wan_3p0_prime_text_to_video::*;
use super::generate_wan_3p0_ref_to_video::*;
use super::generate_wan_3p0_text_to_video::*;
use super::wan_3p0::{KinoviWan3p0AspectRatio, KinoviWan3p0OutputResolution, Wan3p0ApiParams};
use crate::creds::kinovi_web_session::KinoviWebSession;
use crate::error::kinovi_web_client_error::KinoviWebClientError;
use crate::error::kinovi_web_error::KinoviWebError;
use crate::pricing::kinovi_cost_calculator_trait::KinoviCostCalculatorTrait;
use crate::pricing::kinovi_pricing_tier::KinoviPricingTier;
use crate::requests::kinovi_host::KinoviHost;

const RESOLUTIONS: [KinoviWan3p0OutputResolution; 3] = [
  KinoviWan3p0OutputResolution::FourEightyP,
  KinoviWan3p0OutputResolution::SevenTwentyP,
  KinoviWan3p0OutputResolution::TenEightyP,
];

// Literal price snapshots from the supplied observations and Kinovi's model
// pages, checked 2026-09-18. Keep independent of the calculator's rate tables.
// Columns are 480p, 720p, 1080p; all three modalities have the same rates.
// WAN_CREDITS and WAN_CENTS intentionally assert promotional prices until a
// reviewed update after 2026-09-23. See costs_wan_3p0.md at the crate root for
// advertised replacement rates and all expectations that need to change.
const WAN_CREDITS: [(u8, [f64; 3]); 9] = [
  (2, [19.96, 39.92, 79.82]),
  (5, [49.9, 99.8, 199.55]),
  (8, [79.84, 159.68, 319.28]),
  (10, [99.8, 199.6, 399.1]),
  (15, [149.7, 299.4, 598.65]),
  (20, [199.6, 399.2, 798.2]),
  (22, [219.56, 439.12, 878.02]),
  (25, [249.5, 499.0, 997.75]),
  (30, [299.4, 598.8, 1197.3]),
];
const PRIME_CREDITS: [(u8, [f64; 3]); 9] = [
  (2, [37.72, 77.66, 155.32]),
  (5, [94.3, 194.15, 388.3]),
  (8, [150.88, 310.64, 621.28]),
  (10, [188.6, 388.3, 776.6]),
  (15, [282.9, 582.45, 1164.9]),
  (20, [377.2, 776.6, 1553.2]),
  (22, [414.92, 854.26, 1708.52]),
  (25, [471.5, 970.75, 1941.5]),
  // The supplied Prime reference 480p/30s figure said 568.8; docs say 565.8.
  (30, [565.8, 1164.9, 2329.8]),
];
// Thirty-second USD cents, (rounded down, rounded up), consumer then enterprise.
const WAN_CENTS: [[(u64, u64); 3]; 2] = [[(155, 156), (310, 311), (620, 621)], [(123, 124), (246, 247), (492, 493)]];
const PRIME_CENTS: [[(u64, u64); 3]; 2] =
  [[(293, 294), (603, 604), (1207, 1208)], [(232, 233), (479, 480), (958, 959)]];
// Only sanitized apiParams are included; the original captures contain cookies.
const CAPTURES: [&str; 7] = [
  include_str!("wan_3p0_test_data/01_wan_ref_to_video_without_ref.json"),
  include_str!("wan_3p0_test_data/02_wan_text_to_video.json"),
  include_str!("wan_3p0_test_data/03_wan_image_to_video.json"),
  include_str!("wan_3p0_test_data/04_wan_ref_to_video_full.json"),
  include_str!("wan_3p0_test_data/05_wan_prime_ref_to_video.json"),
  include_str!("wan_3p0_test_data/06_wan_prime_text_to_video.json"),
  include_str!("wan_3p0_test_data/07_wan_prime_image_to_video.json"),
];

macro_rules! operation_tests {
  ($module:ident, $request:ident, $args:ident, $generate:ident, $model:literal,
    $credits:ident, $cents:ident, {$($field:ident: $value:expr),* $(,)?}) => {
    mod $module {
      use super::*;

      #[test]
      fn literal_credit_prices_at_every_resolution_and_observed_duration() {
        for tier in [KinoviPricingTier::Consumer, KinoviPricingTier::Enterprise] {
          for (duration, expected) in $credits {
            for (resolution, credits) in RESOLUTIONS.into_iter().zip(expected) {
              // Cost estimates work before media has been uploaded.
              let request = $request {
                duration_seconds: duration,
                maybe_output_resolution: Some(resolution),
                ..Default::default()
              };
              assert_eq!(request.calculate_costs(tier).kinovi_credits, credits,
                "{tier:?}, {resolution:?}, {duration}s");
            }
          }
        }
      }

      #[test]
      fn literal_usd_cent_prices_for_both_tiers() {
        for (tier, expected) in [KinoviPricingTier::Consumer, KinoviPricingTier::Enterprise]
          .into_iter().zip($cents)
        {
          for (resolution, (down, up)) in RESOLUTIONS.into_iter().zip(expected) {
            let request = $request {
              duration_seconds: 30,
              maybe_output_resolution: Some(resolution),
              ..Default::default()
            };
            let cost = request.calculate_costs(tier);
            assert_eq!((cost.usd_cents_rounded_down, cost.usd_cents_rounded_up), (down, up));
          }
        }
      }

      #[test]
      fn defaults_match_the_priced_resolution() {
        let request = valid_request();
        assert_eq!(request.calculate_consumer_costs().kinovi_credits, $credits[1].1[1]);
        let params = serde_json::to_value(request.build_api_params().unwrap()).unwrap();
        assert_eq!(params["model"], $model);
        assert_eq!(params["duration"], "5s");
        assert_eq!(params["outputResolution"], "720p");
        assert_eq!(params["aspectRatio"], "16:9");
        assert!(params.get("seed").is_none());
        assert!(params.get("audio").is_none());
      }

      #[test]
      fn all_whole_second_durations_and_aspect_ratios_serialize() {
        for duration in 2..=30 {
          let mut request = valid_request();
          request.duration_seconds = duration;
          let params = serde_json::to_value(request.build_api_params().unwrap()).unwrap();
          assert_eq!(params["duration"], format!("{duration}s"));
        }
        for (maybe_aspect_ratio, expected) in [
          (KinoviWan3p0AspectRatio::Landscape16x9, "16:9"),
          (KinoviWan3p0AspectRatio::Portrait9x16, "9:16"),
          (KinoviWan3p0AspectRatio::Landscape4x3, "4:3"),
          (KinoviWan3p0AspectRatio::Portrait3x4, "3:4"),
          (KinoviWan3p0AspectRatio::Square1x1, "1:1"),
        ] {
          let mut request = valid_request();
          request.maybe_aspect_ratio = Some(maybe_aspect_ratio);
          let params = serde_json::to_value(request.build_api_params().unwrap()).unwrap();
          assert_eq!(params["aspectRatio"], expected);
        }
        for (resolution, expected) in RESOLUTIONS.into_iter().zip(["480p", "720p", "1080p"]) {
          let mut request = valid_request();
          request.maybe_output_resolution = Some(resolution);
          let params = serde_json::to_value(request.build_api_params().unwrap()).unwrap();
          assert_eq!(params["outputResolution"], expected);
        }
      }

      #[test]
      fn seed_and_audio_flags_preserve_values_without_changing_price() {
        for (seed, audio) in [(0, false), (2_147_483_647, true)] {
          let mut request = valid_request();
          request.maybe_seed = Some(seed);
          request.maybe_generate_audio = Some(audio);
          assert_eq!(request.calculate_consumer_costs().kinovi_credits, $credits[1].1[1]);
          let params = serde_json::to_value(request.build_api_params().unwrap()).unwrap();
          assert_eq!(params["seed"], seed);
          assert_eq!(params["audio"], audio);
        }
      }

      #[test]
      fn invalid_duration_seed_and_prompt_are_rejected() {
        for duration in [0, 1, 31, 255] {
          let mut request = valid_request();
          request.duration_seconds = duration;
          assert_invalid(request.build_api_params(), "duration_seconds");
        }
        for seed in [2_147_483_648, u32::MAX] {
          let mut request = valid_request();
          request.maybe_seed = Some(seed);
          assert_invalid(request.build_api_params(), "maybe_seed");
        }
        let mut request = valid_request();
        request.prompt = "猫".repeat(20_000);
        assert!(request.clone().build_api_params().is_ok());
        request.prompt.push('猫');
        assert_invalid(request.build_api_params(), "prompt");
      }

      #[tokio::test]
      async fn public_generate_sends_workflow_request_and_returns_task_ids() {
        let (host, server) = stub_server().await;
        let session = KinoviWebSession::from_cookies_string("test-session=local".to_owned());
        let request = valid_request();
        let params = serde_json::to_value(request.clone().build_api_params().unwrap()).unwrap();
        let result = timeout(Duration::from_secs(10), $generate($args {
          request,
          session: &session,
          maybe_host_override: Some(host),
        })).await.unwrap().unwrap();
        assert_eq!(result.task_id, "wan-task");
        assert_eq!(result.order_id, "wan-order");
        assert_eq!(result.task_ids, Some(vec!["wan-task".to_owned()]));
        assert_eq!(result.order_ids, Some(vec!["wan-order".to_owned()]));
        let (headers, body) = timeout(Duration::from_secs(10), server).await.unwrap().unwrap();
        assert!(headers.starts_with("POST /api/trpc/workflow.runTask?batch=1 HTTP/1.1\r\n"));
        assert_eq!(body, json!({"0": {"json": {
          "businessType": "wan3-video-generation", "apiParams": params,
        }}}));
      }

      fn valid_request() -> $request {
        $request {
          prompt: "A lighthouse on the coast".to_owned(),
          $($field: $value,)*
          ..Default::default()
        }
      }
    }
  };
}

operation_tests!(
  text,
  GenerateWan3p0TextToVideoRequest,
  GenerateWan3p0TextToVideoArgs,
  generate_wan_3p0_text_to_video,
  "wan3.0-text-to-video",
  WAN_CREDITS,
  WAN_CENTS,
  {}
);
operation_tests!(image, GenerateWan3p0ImageToVideoRequest, GenerateWan3p0ImageToVideoArgs,
  generate_wan_3p0_image_to_video, "wan3.0-image-to-video", WAN_CREDITS, WAN_CENTS,
  { maybe_start_frame_url: Some("https://example.com/start.png".to_owned()) });
operation_tests!(
  reference,
  GenerateWan3p0RefToVideoRequest,
  GenerateWan3p0RefToVideoArgs,
  generate_wan_3p0_ref_to_video,
  "wan3.0-ref-to-video",
  WAN_CREDITS,
  WAN_CENTS,
  {}
);
operation_tests!(
  prime_text,
  GenerateWan3p0PrimeTextToVideoRequest,
  GenerateWan3p0PrimeTextToVideoArgs,
  generate_wan_3p0_prime_text_to_video,
  "wan3.0-prime-text-to-video",
  PRIME_CREDITS,
  PRIME_CENTS,
  {}
);
operation_tests!(prime_image, GenerateWan3p0PrimeImageToVideoRequest, GenerateWan3p0PrimeImageToVideoArgs,
  generate_wan_3p0_prime_image_to_video, "wan3.0-prime-image-to-video", PRIME_CREDITS, PRIME_CENTS,
  { maybe_start_frame_url: Some("https://example.com/start.png".to_owned()) });
operation_tests!(
  prime_reference,
  GenerateWan3p0PrimeRefToVideoRequest,
  GenerateWan3p0PrimeRefToVideoArgs,
  generate_wan_3p0_prime_ref_to_video,
  "wan3.0-prime-ref-to-video",
  PRIME_CREDITS,
  PRIME_CENTS,
  {}
);

#[test]
fn payloads_match_all_seven_consumer_captures() {
  for fixture in CAPTURES {
    let expected: Value = serde_json::from_str(fixture).unwrap();
    assert_eq!(params_from_capture(&expected), expected);
  }
  // Capture 03's filename says Wan image, but its actual model is Prime.
  // Exercise regular image too, using the same documented keyframe shape.
  let mut regular_image: Value = serde_json::from_str(CAPTURES[2]).unwrap();
  regular_image["model"] = json!("wan3.0-image-to-video");
  assert_eq!(params_from_capture(&regular_image), regular_image);
}

macro_rules! reference_media_tests {
  ($module:ident, $request:ident, $credits:literal) => {
    mod $module {
      use super::*;

      #[test]
      fn references_are_optional_and_empty_lists_are_omitted() {
        let request = $request {
          prompt: "A ship at sea".to_owned(),
          duration_seconds: 30,
          maybe_reference_image_urls: Some(vec![]),
          maybe_reference_video_urls: Some(vec![]),
          maybe_reference_audio_urls: Some(vec![]),
          ..Default::default()
        };
        let params = serde_json::to_value(request.build_api_params().unwrap()).unwrap();
        for field in ["uploadedUrls", "videoUrl", "videoUrls", "audioUrls"] {
          assert!(params.get(field).is_none(), "{field}");
        }
        assert_invalid($request::default().build_api_params(), "prompt");
      }

      #[test]
      fn reference_duration_and_output_share_thirty_seconds() {
        let request = $request {
          duration_seconds: 20,
          maybe_reference_video_urls: Some(urls(1)),
          maybe_total_reference_video_duration_millis: Some(10_000),
          ..Default::default()
        };
        assert!(request.clone().build_api_params().is_ok());
        for millis in [0, 999, 10_001, 15_001, u32::MAX] {
          let mut invalid = request.clone();
          invalid.maybe_total_reference_video_duration_millis = Some(millis);
          assert!(invalid.build_api_params().is_err(), "{millis}ms");
        }
        let mut request = request;
        request.duration_seconds = 15;
        request.maybe_total_reference_video_duration_millis = Some(15_000);
        assert!(request.clone().build_api_params().is_ok());
        request.duration_seconds = 16;
        assert_invalid(request.clone().build_api_params(), "duration_seconds");
        request.maybe_total_reference_video_duration_millis = None;
        request.duration_seconds = 29;
        assert!(request.clone().build_api_params().is_ok());
        request.duration_seconds = 30;
        assert_invalid(request.build_api_params(), "duration_seconds");
      }

      #[test]
      fn all_reference_types_have_independent_limits_and_no_surcharge() {
        let request = $request {
          duration_seconds: 10,
          maybe_reference_image_urls: Some(urls(10)),
          maybe_reference_video_urls: Some(urls(5)),
          maybe_reference_audio_urls: Some(urls(5)),
          maybe_total_reference_video_duration_millis: Some(15_000),
          ..Default::default()
        };
        assert_eq!(request.calculate_consumer_costs().kinovi_credits, $credits);
        let params = serde_json::to_value(request.clone().build_api_params().unwrap()).unwrap();
        assert_eq!(params["uploadedUrls"], json!(urls(10)));
        assert_eq!(params["videoUrls"], json!(urls(5)));
        assert_eq!(params["videoUrl"], urls(1)[0]);
        assert_eq!(params["audioUrls"], json!(urls(5)));
        assert!(params.get("maybeTotalReferenceVideoDurationMillis").is_none());
        assert!(params.get("imageUrls").is_none());
        let mut invalid = request.clone();
        invalid.maybe_reference_image_urls = Some(urls(11));
        assert_invalid(invalid.build_api_params(), "maybe_reference_image_urls");
        let mut invalid = request.clone();
        invalid.maybe_reference_video_urls = Some(urls(6));
        assert_invalid(invalid.build_api_params(), "maybe_reference_video_urls");
        let mut invalid = request.clone();
        invalid.maybe_reference_audio_urls = Some(urls(6));
        assert_invalid(invalid.build_api_params(), "maybe_reference_audio_urls");
        let mut invalid = request.clone();
        invalid.maybe_total_reference_video_duration_millis = Some(4_999);
        assert_invalid(invalid.build_api_params(), "maybe_total_reference_video_duration_millis");
        let mut invalid = request;
        invalid.maybe_reference_audio_urls = Some(vec!["  ".to_owned()]);
        assert_invalid(invalid.build_api_params(), "media_url");
      }
    }
  };
}

reference_media_tests!(reference_media, GenerateWan3p0RefToVideoRequest, 199.6);
reference_media_tests!(prime_reference_media, GenerateWan3p0PrimeRefToVideoRequest, 388.3);

#[test]
fn keyframes_require_a_start_image_and_preserve_start_end_order() {
  macro_rules! check {
    ($request:ident, $credits:literal) => {
      let mut request = $request {
        prompt: "Waves".to_owned(),
        ..Default::default()
      };
      assert_invalid(request.clone().build_api_params(), "maybe_start_frame_url");
      request.maybe_end_frame_url = Some("https://example.com/end.png".to_owned());
      assert_invalid(request.clone().build_api_params(), "maybe_end_frame_url");
      request.maybe_start_frame_url = Some("https://example.com/start.png".to_owned());
      request.prompt.clear();
      assert_eq!(request.calculate_consumer_costs().kinovi_credits, $credits);
      let params = serde_json::to_value(request.build_api_params().unwrap()).unwrap();
      assert_eq!(params["uploadedUrls"], json!(["https://example.com/start.png", "https://example.com/end.png",]));
      assert!(params.get("imageUrls").is_none());
    };
  }
  check!(GenerateWan3p0ImageToVideoRequest, 99.8);
  check!(GenerateWan3p0PrimeImageToVideoRequest, 194.15);
}

#[test]
fn text_operations_require_a_prompt() {
  assert_invalid(GenerateWan3p0TextToVideoRequest::default().build_api_params(), "prompt");
  assert_invalid(GenerateWan3p0PrimeTextToVideoRequest::default().build_api_params(), "prompt");
}

fn params_from_capture(value: &Value) -> Value {
  macro_rules! build {
    ($request:ident, {$($field:ident: $extra:expr),* $(,)?}) => {
      $request {
        prompt: value["prompt"].as_str().unwrap().to_owned(),
        maybe_aspect_ratio: Some(match value["aspectRatio"].as_str().unwrap() {
          "16:9" => KinoviWan3p0AspectRatio::Landscape16x9,
          "4:3" => KinoviWan3p0AspectRatio::Landscape4x3,
          other => panic!("unexpected fixture aspect ratio: {other}"),
        }),
        maybe_output_resolution: Some(match value["outputResolution"].as_str().unwrap() {
          "480p" => KinoviWan3p0OutputResolution::FourEightyP,
          "720p" => KinoviWan3p0OutputResolution::SevenTwentyP,
          "1080p" => KinoviWan3p0OutputResolution::TenEightyP,
          other => panic!("unexpected fixture resolution: {other}"),
        }),
        duration_seconds: value["duration"].as_str().unwrap().trim_end_matches('s').parse().unwrap(),
        maybe_seed: value["seed"].as_u64().map(|seed| seed as u32),
        $($field: $extra,)*
        ..Default::default()
      }.build_api_params().unwrap()
    };
  }
  macro_rules! image {
    ($request:ident) => {
      build!($request, {
        maybe_start_frame_url: value["uploadedUrls"][0].as_str().map(str::to_owned),
        maybe_end_frame_url: value["uploadedUrls"][1].as_str().map(str::to_owned),
      })
    };
  }
  macro_rules! reference {
    ($request:ident) => {
      build!($request, {
        maybe_reference_image_urls: capture_urls(value, "uploadedUrls"),
        maybe_reference_video_urls: capture_urls(value, "videoUrls"),
        maybe_reference_audio_urls: capture_urls(value, "audioUrls"),
      })
    };
  }
  let params = match value["model"].as_str().unwrap() {
    "wan3.0-text-to-video" => build!(GenerateWan3p0TextToVideoRequest, {}),
    "wan3.0-image-to-video" => image!(GenerateWan3p0ImageToVideoRequest),
    "wan3.0-ref-to-video" => reference!(GenerateWan3p0RefToVideoRequest),
    "wan3.0-prime-text-to-video" => build!(GenerateWan3p0PrimeTextToVideoRequest, {}),
    "wan3.0-prime-image-to-video" => image!(GenerateWan3p0PrimeImageToVideoRequest),
    "wan3.0-prime-ref-to-video" => reference!(GenerateWan3p0PrimeRefToVideoRequest),
    other => panic!("unexpected fixture model: {other}"),
  };
  serde_json::to_value(params).unwrap()
}

async fn stub_server() -> (KinoviHost, JoinHandle<(String, Value)>) {
  let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = format!("http://{}", listener.local_addr().unwrap());
  let server = tokio::spawn(async move {
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut bytes = Vec::new();
    let (headers, body_start, body_len) = loop {
      let mut chunk = [0; 4096];
      let length = stream.read(&mut chunk).await.unwrap();
      assert_ne!(length, 0, "request ended before headers");
      bytes.extend_from_slice(&chunk[..length]);
      if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
        let headers = String::from_utf8(bytes[..end + 4].to_vec()).unwrap();
        let body_len: usize = headers
          .lines()
          .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse().unwrap())
          })
          .unwrap();
        break (headers, end + 4, body_len);
      }
    };
    while bytes.len() < body_start + body_len {
      let mut chunk = [0; 4096];
      let length = stream.read(&mut chunk).await.unwrap();
      assert_ne!(length, 0, "request ended before body");
      bytes.extend_from_slice(&chunk[..length]);
    }
    let body = serde_json::from_slice(&bytes[body_start..body_start + body_len]).unwrap();
    let response = json!([{"result": {"data": {"json": {
      "taskId": "wan-task", "orderId": "wan-order",
      "taskIds": ["wan-task"], "orderIds": ["wan-order"], "violationWarning": false,
    }}}}])
    .to_string();
    let response = format!(
      "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
      response.len(),
      response,
    );
    stream.write_all(response.as_bytes()).await.unwrap();
    (headers, body)
  });
  (
    KinoviHost::CustomHost {
      api_host: address.clone(),
      cdn_host: address,
    },
    server,
  )
}

fn capture_urls(value: &Value, field: &str) -> Option<Vec<String>> {
  value[field]
    .as_array()
    .map(|urls| urls.iter().map(|url| url.as_str().unwrap().to_owned()).collect())
}

fn urls(count: usize) -> Vec<String> {
  (0..count).map(|index| format!("https://example.com/reference-{index}")).collect()
}

fn assert_invalid(result: Result<Wan3p0ApiParams, KinoviWebError>, field: &str) {
  match result.unwrap_err() {
    KinoviWebError::Client(KinoviWebClientError::InvalidRequestField {
      field: actual,
      ..
    }) => {
      assert_eq!(actual, field);
    },
    other => panic!("expected invalid {field}, got {other:?}"),
  }
}
