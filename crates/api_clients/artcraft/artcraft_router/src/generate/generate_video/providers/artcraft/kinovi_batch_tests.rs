//! Public ArtCraft batch planning and retail price snapshots for Kinovi models.

use artcraft_api_defs::omni_gen::cost_and_generate_requests::omni_gen_video_cost_and_generate_request::OmniGenVideoCostAndGenerateRequest;
use tokens::tokens::media_files::MediaFileToken;

use crate::api::router_provider::RouterProvider;
use crate::api::router_resolution::RouterResolution;
use crate::api::router_video_model::RouterVideoModel;
use crate::api::video_list_ref::VideoListRef;
use crate::client::request_mismatch_mitigation_strategy::RequestMismatchMitigationStrategy;
use crate::errors::artcraft_router_error::ArtcraftRouterError;
use crate::errors::client_error::ClientError;
use crate::generate::generate_video::generate_video_request_builder::GenerateVideoRequestBuilder;
use crate::generate::generate_video::video_generation_draft_or_request::VideoGenerationDraftOrRequest;
use crate::generate::generate_video::video_generation_request::VideoGenerationRequest;

const MODELS: [RouterVideoModel; 12] = [
  RouterVideoModel::Seedance2p5,
  RouterVideoModel::Seedance2p0,
  RouterVideoModel::Seedance2p0Fast,
  RouterVideoModel::Seedance2p0Mini,
  RouterVideoModel::Seedance2p5Ultra,
  RouterVideoModel::Seedance2p0BytePlus,
  RouterVideoModel::Seedance2p0BytePlusFast,
  RouterVideoModel::Seedance2p0BytePlusMini,
  RouterVideoModel::Seedance2p0BytePlusUltra,
  RouterVideoModel::Seedance2p0BytePlusUltraFast,
  RouterVideoModel::Seedance2p0BytePlusUltraMini,
  RouterVideoModel::HappyHorse1p0,
];

mod batch_planning {
  use super::*;

  #[test]
  fn batches_one_through_four_survive_http_serialization() {
    for model in MODELS {
      for strategy in [
        RequestMismatchMitigationStrategy::ErrorOut,
        RequestMismatchMitigationStrategy::PayMoreUpgrade,
        RequestMismatchMitigationStrategy::PayLessDowngrade,
      ] {
        for batch in [None, Some(1), Some(2), Some(3), Some(4)] {
          let mut builder = builder(model, RouterResolution::SevenTwentyP, batch);
          builder.request_mismatch_mitigation_strategy = strategy;
          let request = http_request(builder.build2().unwrap());
          let json = serde_json::to_value(&request).unwrap();
          assert_eq!(json["video_batch_count"], batch.unwrap_or(1), "{model:?}");
          let decoded: OmniGenVideoCostAndGenerateRequest = serde_json::from_value(json).unwrap();
          assert_eq!(decoded.video_batch_count, Some(batch.unwrap_or(1)));
        }
      }
    }
  }

  #[test]
  fn zero_is_rejected_under_every_strategy() {
    for model in MODELS {
      for strategy in [
        RequestMismatchMitigationStrategy::ErrorOut,
        RequestMismatchMitigationStrategy::PayMoreUpgrade,
        RequestMismatchMitigationStrategy::PayLessDowngrade,
      ] {
        let mut builder = builder(model, RouterResolution::SevenTwentyP, Some(0));
        builder.request_mismatch_mitigation_strategy = strategy;
        assert!(matches!(builder.build2(), Err(ArtcraftRouterError::Client(
          ClientError::UserRequestedZeroGenerations
        ))), "{model:?}");
      }
    }
  }

  #[test]
  fn batches_above_four_follow_the_mitigation_strategy() {
    for model in MODELS {
      for batch in [5, 6, 7, 8, u16::MAX] {
        let builder = builder(model, RouterResolution::SevenTwentyP, Some(batch));
        assert!(matches!(builder.clone().build2(), Err(ArtcraftRouterError::Client(
          ClientError::ModelDoesNotSupportOption { field: "video_batch_count", .. }
        ))), "{model:?}, batch {batch}");
        for strategy in [
          RequestMismatchMitigationStrategy::PayMoreUpgrade,
          RequestMismatchMitigationStrategy::PayLessDowngrade,
        ] {
          let mut builder = builder.clone();
          builder.request_mismatch_mitigation_strategy = strategy;
          let request = http_request(builder.build2().unwrap());
          assert_eq!(request.video_batch_count, Some(4), "{model:?}, batch {batch}");
        }
      }
    }
  }
}

mod retail_prices {
  use super::*;
  use RouterResolution::{FourEightyP, FourK, SevenTwentyP, TenEightyP};
  use RouterVideoModel::{HappyHorse1p0, Seedance2p0, Seedance2p0Fast, Seedance2p0Mini, Seedance2p5,
    Seedance2p5Ultra, Seedance2p0BytePlus, Seedance2p0BytePlusFast, Seedance2p0BytePlusMini, Seedance2p0BytePlusUltra, Seedance2p0BytePlusUltraFast, Seedance2p0BytePlusUltraMini,
  };

  #[test]
  fn five_second_prices_for_every_resolution_and_batch() {
    // Literal USD cents / ArtCraft credits for batches [1, 2, 3, 4].
    // Pin the final rounded totals, not a multiple of an already-rounded price.
    for (model, resolution, expected) in [
      (Seedance2p5, FourEightyP, [59, 118, 177, 236]),
      (Seedance2p5, SevenTwentyP, [134, 268, 401, 535]),
      (Seedance2p5, TenEightyP, [309, 617, 925, 1233]),
      (Seedance2p5Ultra, FourEightyP, [70, 140, 209, 279]),
      (Seedance2p5Ultra, SevenTwentyP, [158, 316, 474, 632]),
      (Seedance2p5Ultra, TenEightyP, [337, 673, 1009, 1345]),
      (Seedance2p0, FourEightyP, [39, 78, 117, 155]),
      (Seedance2p0, SevenTwentyP, [93, 185, 278, 370]),
      (Seedance2p0, TenEightyP, [233, 466, 699, 933]),
      (Seedance2p0, FourK, [463, 925, 1388, 1850]),
      (Seedance2p0Fast, FourEightyP, [36, 71, 107, 143]),
      (Seedance2p0Fast, SevenTwentyP, [64, 127, 191, 255]),
      (Seedance2p0Mini, FourEightyP, [18, 35, 52, 69]),
      (Seedance2p0Mini, SevenTwentyP, [45, 89, 134, 178]),
      (Seedance2p0BytePlus, FourEightyP, [50, 100, 150, 200]),
      (Seedance2p0BytePlus, SevenTwentyP, [125, 250, 375, 500]),
      (Seedance2p0BytePlus, TenEightyP, [250, 500, 750, 1000]),
      (Seedance2p0BytePlus, FourK, [475, 950, 1425, 1900]),
      (Seedance2p0BytePlusUltra, FourEightyP, [50, 100, 150, 200]),
      (Seedance2p0BytePlusUltra, SevenTwentyP, [125, 250, 375, 500]),
      (Seedance2p0BytePlusUltra, TenEightyP, [250, 500, 750, 1000]),
      (Seedance2p0BytePlusUltra, FourK, [475, 950, 1425, 1900]),
      (Seedance2p0BytePlusFast, FourEightyP, [45, 90, 135, 180]),
      (Seedance2p0BytePlusFast, SevenTwentyP, [100, 200, 300, 400]),
      (Seedance2p0BytePlusUltraFast, FourEightyP, [45, 90, 135, 180]),
      (Seedance2p0BytePlusUltraFast, SevenTwentyP, [100, 200, 300, 400]),
      (Seedance2p0BytePlusMini, FourEightyP, [18, 36, 54, 71]),
      (Seedance2p0BytePlusMini, SevenTwentyP, [46, 91, 137, 182]),
      (Seedance2p0BytePlusUltraMini, FourEightyP, [18, 36, 54, 71]),
      (Seedance2p0BytePlusUltraMini, SevenTwentyP, [46, 91, 137, 182]),
      (HappyHorse1p0, SevenTwentyP, [85, 171, 256, 342]),
      (HappyHorse1p0, TenEightyP, [171, 342, 513, 684]),
    ] {
      assert_batch_prices(builder(model, resolution, None), expected);
    }
  }

  #[test]
  fn video_reference_prices_for_every_resolution_and_batch() {
    // Five output seconds, plus ten measured input seconds for Seedance 2.5.
    for (model, resolution, expected) in [
      (Seedance2p5, FourEightyP, [109, 218, 326, 435]),
      (Seedance2p5, SevenTwentyP, [238, 476, 713, 951]),
      (Seedance2p5, TenEightyP, [578, 1155, 1733, 2310]),
      (Seedance2p5Ultra, FourEightyP, [129, 257, 386, 514]),
      (Seedance2p5Ultra, SevenTwentyP, [281, 562, 843, 1124]),
      (Seedance2p5Ultra, TenEightyP, [628, 1256, 1884, 2511]),
      (Seedance2p0, FourEightyP, [45, 89, 133, 177]),
      (Seedance2p0, SevenTwentyP, [111, 221, 331, 441]),
      (Seedance2p0, TenEightyP, [256, 511, 767, 1022]),
      (Seedance2p0, FourK, [569, 1138, 1707, 2276]),
      (Seedance2p0Fast, FourEightyP, [43, 85, 127, 169]),
      (Seedance2p0Fast, SevenTwentyP, [78, 156, 234, 312]),
      // Preserve Mini's existing floating-point round-up behavior.
      (Seedance2p0Mini, FourEightyP, [22, 44, 66, 88]),
      (Seedance2p0Mini, SevenTwentyP, [54, 108, 161, 215]),
      (Seedance2p0BytePlus, FourEightyP, [51, 102, 153, 204]),
      (Seedance2p0BytePlus, SevenTwentyP, [129, 257, 386, 514]),
      (Seedance2p0BytePlus, TenEightyP, [289, 578, 867, 1156]),
      (Seedance2p0BytePlus, FourK, [595, 1190, 1785, 2380]),
      (Seedance2p0BytePlusUltra, FourEightyP, [51, 102, 153, 204]),
      (Seedance2p0BytePlusUltra, SevenTwentyP, [129, 257, 386, 514]),
      (Seedance2p0BytePlusUltra, TenEightyP, [289, 578, 867, 1156]),
      (Seedance2p0BytePlusUltra, FourK, [595, 1190, 1785, 2380]),
      (Seedance2p0BytePlusFast, FourEightyP, [49, 97, 145, 193]),
      (Seedance2p0BytePlusFast, SevenTwentyP, [103, 205, 308, 410]),
      (Seedance2p0BytePlusUltraFast, FourEightyP, [49, 97, 145, 193]),
      (Seedance2p0BytePlusUltraFast, SevenTwentyP, [103, 205, 308, 410]),
      (Seedance2p0BytePlusMini, FourEightyP, [23, 45, 68, 90]),
      (Seedance2p0BytePlusMini, SevenTwentyP, [55, 110, 165, 219]),
      (Seedance2p0BytePlusUltraMini, FourEightyP, [23, 45, 68, 90]),
      (Seedance2p0BytePlusUltraMini, SevenTwentyP, [55, 110, 165, 219]),
    ] {
      let mut builder = builder(model, resolution, None);
      builder.reference_videos = Some(VideoListRef::MediaFileTokens(vec![
        MediaFileToken::new("mf_reference".to_string()),
      ]));
      builder.total_reference_video_input_seconds = Some(10);
      assert_batch_prices(builder, expected);
    }
  }

  #[test]
  fn seedance_2p5_input_duration_clamping_and_fallback_scale_with_batch() {
    for (model, resolution, minimum, maximum) in [
      (Seedance2p5, FourEightyP, [66, 131, 196, 261], [254, 507, 761, 1014]),
      (Seedance2p5, SevenTwentyP, [143, 286, 428, 571], [555, 1110, 1664, 2219]),
      (Seedance2p5, TenEightyP, [347, 693, 1040, 1386], [1348, 2695, 4043, 5390]),
      (Seedance2p5Ultra, FourEightyP, [78, 155, 232, 309], [300, 600, 899, 1199]),
      (Seedance2p5Ultra, SevenTwentyP, [169, 338, 506, 675], [656, 1311, 1967, 2622]),
      (Seedance2p5Ultra, TenEightyP, [377, 754, 1130, 1507], [1465, 2930, 4394, 5859]),
    ] {
      for (input_seconds, expected) in [
        (Some(1), minimum), (Some(4), minimum),
        (None, maximum), (Some(0), maximum), (Some(30), maximum), (Some(99), maximum),
      ] {
        let mut builder = builder(model, resolution, None);
        builder.reference_videos = Some(VideoListRef::MediaFileTokens(vec![
          MediaFileToken::new("mf_reference".to_string()),
        ]));
        builder.total_reference_video_input_seconds = input_seconds;
        assert_batch_prices(builder, expected);
      }
    }
  }
}

fn builder(model: RouterVideoModel, resolution: RouterResolution, batch: Option<u16>) -> GenerateVideoRequestBuilder {
  GenerateVideoRequestBuilder {
    provider: RouterProvider::Artcraft,
    model,
    resolution: Some(resolution),
    duration_seconds: Some(5),
    video_batch_count: batch,
    request_mismatch_mitigation_strategy: RequestMismatchMitigationStrategy::ErrorOut,
    ..Default::default()
  }
}

fn assert_batch_prices(builder: GenerateVideoRequestBuilder, expected: [u64; 4]) {
  for (index, expected_price) in expected.into_iter().enumerate() {
    let batch = index as u16 + 1;
    let mut builder = builder.clone();
    builder.video_batch_count = Some(batch);
    let context = format!("{:?}, {:?}, batch {batch}", builder.model, builder.resolution);
    let request = builder.build2().unwrap();
    let cost = request.estimate_cost().unwrap();
    assert_eq!(cost.cost_in_credits, Some(expected_price), "{context}");
    assert_eq!(cost.cost_in_usd_cents, Some(expected_price), "{context}");
    assert_eq!(http_request(request).video_batch_count, Some(batch), "{context}");
  }
}

fn http_request(request: VideoGenerationDraftOrRequest) -> OmniGenVideoCostAndGenerateRequest {
  match request {
    VideoGenerationDraftOrRequest::Request(request) => match request {
      VideoGenerationRequest::ArtcraftSeedance2p5(state) => state.request,
      VideoGenerationRequest::ArtcraftSeedance2p0(state) => state.request,
      VideoGenerationRequest::ArtcraftSeedance2p0Fast(state) => state.request,
      VideoGenerationRequest::ArtcraftSeedance2p0Mini(state) => state.request,
      VideoGenerationRequest::ArtcraftSeedance2p5Ultra(state) => state.request,
      VideoGenerationRequest::ArtcraftSeedance2p0BytePlus(state) => state.request,
      VideoGenerationRequest::ArtcraftSeedance2p0BytePlusFast(state) => state.request,
      VideoGenerationRequest::ArtcraftSeedance2p0BytePlusMini(state) => state.request,
      VideoGenerationRequest::ArtcraftSeedance2p0BytePlusUltra(state) => state.request,
      VideoGenerationRequest::ArtcraftSeedance2p0BytePlusUltraFast(state) => state.request,
      VideoGenerationRequest::ArtcraftSeedance2p0BytePlusUltraMini(state) => state.request,
      VideoGenerationRequest::ArtcraftHappyHorse1p0(state) => state.request,
      other => panic!("Unexpected request: {other:?}"),
    },
    other => panic!("Expected ArtCraft request: {other:?}"),
  }
}
