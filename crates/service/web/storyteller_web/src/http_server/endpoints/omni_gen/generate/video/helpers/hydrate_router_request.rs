use crate::http_server::common_responses::common_web_error::CommonWebError;
use artcraft_api_defs::omni_gen::cost_and_generate_requests::omni_gen_video_cost_and_generate_request::OmniGenVideoCostAndGenerateRequest;
use artcraft_router::api::audio_list_ref::AudioListRef;
use artcraft_router::api::character_list_ref::CharacterListRef;
use artcraft_router::api::router_aspect_ratio::RouterAspectRatio;
use artcraft_router::api::router_bitrate::RouterBitrate;
use artcraft_router::api::router_resolution::RouterResolution;
use artcraft_router::api::router_video_model::RouterVideoModel;
use artcraft_router::api::router_video_output_format::RouterVideoOutputFormat;
use artcraft_router::api::image_list_ref::ImageListRef;
use artcraft_router::api::image_ref::ImageRef;
use artcraft_router::api::router_provider::RouterProvider;
use artcraft_router::api::video_list_ref::VideoListRef;
use artcraft_router::client::request_mismatch_mitigation_strategy::RequestMismatchMitigationStrategy;
use artcraft_router::generate::generate_video::generate_video_request_builder::GenerateVideoRequestBuilder;
use enums::common::generation::common_aspect_ratio::CommonAspectRatio as CommonAspectRatioEnum;
use enums::common::generation::common_bitrate::CommonBitrate as CommonBitrateEnum;
use enums::common::generation::common_resolution::CommonResolution as CommonResolutionEnum;
use enums::common::generation::common_video_model::CommonVideoModel as CommonVideoModelEnum;
use enums::common::generation::common_video_output_format::CommonVideoOutputFormat;

pub fn hydrate_to_router_request(
  request: &OmniGenVideoCostAndGenerateRequest,
) -> Result<GenerateVideoRequestBuilder, CommonWebError> {
  let api_model = request.model
    .as_ref()
    .ok_or_else(|| CommonWebError::BadInputWithSimpleMessage(
      "model is required".to_string(),
    ))?;

  let model = convert_model(api_model)?;

  let aspect_ratio = request.aspect_ratio
    .as_ref()
    .map(convert_aspect_ratio)
    .transpose()?;

  let resolution = request.resolution
    .as_ref()
    .map(convert_resolution)
    .transpose()?;

  let bitrate = request.bitrate
    .as_ref()
    .map(convert_bitrate)
    .transpose()?;

  Ok(GenerateVideoRequestBuilder {
    model,
    provider: RouterProvider::Artcraft,
    prompt: request.prompt.clone(),
    negative_prompt: request.negative_prompt.clone(),
    start_frame: request.start_frame_image_media_token.clone()
      .map(ImageRef::MediaFileToken),
    end_frame: request.end_frame_image_media_token.clone()
      .map(ImageRef::MediaFileToken),
    reference_images: request.reference_image_media_tokens.clone()
      .map(ImageListRef::MediaFileTokens),
    reference_videos: request.reference_video_media_tokens.clone()
      .map(VideoListRef::MediaFileTokens),
    reference_audio: request.reference_audio_media_tokens.clone()
      .map(AudioListRef::MediaFileTokens),
    reference_character_tokens: request.reference_character_tokens.clone()
      .map(CharacterListRef::CharacterTokens),
    resolution,
    aspect_ratio,
    bitrate,
    maybe_output_format: if matches!(model, RouterVideoModel::Seedance2p5 | RouterVideoModel::Seedance2p5Ultra) {
      Some(match request.maybe_output_format.unwrap_or(CommonVideoOutputFormat::Mp4) {
        CommonVideoOutputFormat::Mp4 => RouterVideoOutputFormat::Mp4,
        CommonVideoOutputFormat::Mov => RouterVideoOutputFormat::Mov,
      })
    } else {
      None
    },
    duration_seconds: request.duration_seconds,
    video_batch_count: request.video_batch_count,
    // Seedance 2.0 / 2.5 always generate sound. Kinovi honours an explicit
    // `generate_audio: false`, and the frontend was sending one, so the flag
    // is dropped here and never reaches Kinovi for these models.
    generate_audio: if is_seedance_2x(&model) { None } else { request.generate_audio },
    // Set by the handler after probing reference video durations (only
    // models that bill input seconds use it).
    total_reference_video_input_seconds: None,
    request_mismatch_mitigation_strategy: RequestMismatchMitigationStrategy::PayMoreUpgrade,
    idempotency_token: request.idempotency_token.clone(),
  })
}

fn is_seedance_2x(model: &RouterVideoModel) -> bool {
  matches!(
    model,
    RouterVideoModel::Seedance2p0
      | RouterVideoModel::Seedance2p0Fast
      | RouterVideoModel::Seedance2p0Mini
      | RouterVideoModel::Seedance2p0BytePlus
      | RouterVideoModel::Seedance2p0BytePlusFast
      | RouterVideoModel::Seedance2p0BytePlusMini
      | RouterVideoModel::Seedance2p0BytePlusUltra
      | RouterVideoModel::Seedance2p0BytePlusUltraFast
      | RouterVideoModel::Seedance2p0BytePlusUltraMini
      | RouterVideoModel::Seedance2p5Preview
      | RouterVideoModel::Seedance2p5
      | RouterVideoModel::Seedance2p5Ultra
  )
}

fn convert_model(
  model: &CommonVideoModelEnum,
) -> Result<RouterVideoModel, CommonWebError> {
  let json = serde_json::to_string(model)?;
  serde_json::from_str(&json).map_err(|e| {
    CommonWebError::BadInputWithSimpleMessage(
      format!("Unsupported video model: {}", e),
    )
  })
}

fn convert_aspect_ratio(
  ar: &CommonAspectRatioEnum,
) -> Result<RouterAspectRatio, CommonWebError> {
  let json = serde_json::to_string(ar)?;
  serde_json::from_str(&json).map_err(|e| {
    CommonWebError::BadInputWithSimpleMessage(
      format!("Unsupported aspect ratio: {}", e),
    )
  })
}

fn convert_resolution(
  res: &CommonResolutionEnum,
) -> Result<RouterResolution, CommonWebError> {
  let json = serde_json::to_string(res)?;
  serde_json::from_str(&json).map_err(|e| {
    CommonWebError::BadInputWithSimpleMessage(
      format!("Unsupported resolution: {}", e),
    )
  })
}

fn convert_bitrate(
  bitrate: &CommonBitrateEnum,
) -> Result<RouterBitrate, CommonWebError> {
  let json = serde_json::to_string(bitrate)?;
  serde_json::from_str(&json).map_err(|e| {
    CommonWebError::BadInputWithSimpleMessage(
      format!("Unsupported bitrate: {}", e),
    )
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  mod output_format {
    use super::*;
    use artcraft_router::client::router_client::RouterClient;
    use artcraft_router::client::router_kinovi_web_client::RouterKinoviWebClient;
    use artcraft_router::generate::generate_video::video_generation_draft_context::VideoGenerationDraftContext;
    use artcraft_router::generate::generate_video::video_generation_draft_or_request::VideoGenerationDraftOrRequest;
    use artcraft_router::generate::generate_video::video_generation_request::VideoGenerationRequest;
    use kinovi_web_client::creds::kinovi_web_session::KinoviWebSession;
    use kinovi_web_client::generate::video::generate_seedance_2p5::KinoviSeedance2p5OutputFormat;
    use serde_json::{json, Value};

    #[tokio::test]
    async fn artcraft_http_request_reaches_kinovi_with_format_and_without_audio_flag() {
      // No media inputs or send: finalization needs a client but makes no HTTP calls.
      let client = RouterClient::KinoviWeb(RouterKinoviWebClient::new(
        KinoviWebSession::from_cookies_string(String::new()),
      ));
      let context = VideoGenerationDraftContext { client: Some(&client), ..Default::default() };
      for model in ["seedance_2p5", "seedance_2p5_u"] {
        for output_format in [None, Some(Value::Null), Some(json!("mp4")), Some(json!("mov"))] {
          for generate_audio in [None, Some(true), Some(false)] {
            let mut body = json!({
              "model": model,
              "generate_audio": generate_audio,
            });
            if let Some(format) = &output_format {
              body["output_format"] = format.clone();
            }
            let request: OmniGenVideoCostAndGenerateRequest = serde_json::from_value(body).unwrap();
            let mut builder = hydrate_to_router_request(&request).unwrap();
            builder.provider = RouterProvider::KinoviWeb;
            // Mirror run_pipeline_v2: Ultra executes as the base model on
            // its own account, while billing retains the original model.
            if matches!(builder.model, RouterVideoModel::Seedance2p5Ultra) {
              builder.model = RouterVideoModel::Seedance2p5;
            }
            let draft = match builder.build2().unwrap() {
              VideoGenerationDraftOrRequest::Draft(draft) => draft,
              other => panic!("Unexpected request: {:?}", other),
            };
            let finalized = draft.finalize(context.clone()).await.unwrap();
            let request = match finalized {
              VideoGenerationRequest::KinoviSeedance2p5(state) => state.request,
              other => panic!("Unexpected request: {:?}", other),
            };
            let actual_format = request.maybe_output_format.map(|format| match format {
              KinoviSeedance2p5OutputFormat::Mp4 => "mp4",
              KinoviSeedance2p5OutputFormat::Mov => "mov",
            });
            let expected_format = output_format.as_ref().and_then(Value::as_str).unwrap_or("mp4");
            assert_eq!(actual_format, Some(expected_format), "model: {model}");
            assert_eq!(request.maybe_generate_audio, None, "generate_audio: {generate_audio:?}");
          }
        }
      }
    }

    #[test]
    fn unsupported_model_ignores_format() {
      for model in ["seedance_2p0", "seedance_2p5_preview"] {
        let request: OmniGenVideoCostAndGenerateRequest = serde_json::from_value(json!({
          "model": model, "output_format": "mov"
        })).unwrap();
        assert!(hydrate_to_router_request(&request).unwrap().maybe_output_format.is_none());
      }
    }

    #[test]
    fn invalid_format_is_rejected() {
      for output_format in [json!("webm"), json!("MP4"), json!(42)] {
        let request = serde_json::from_value::<OmniGenVideoCostAndGenerateRequest>(json!({
          "model": "seedance_2p5", "output_format": output_format,
        }));
        assert!(request.is_err());
      }
    }
  }

  mod generate_audio {
    use super::*;

    #[test]
    fn seedance_2x_drops_generate_audio() {
      for model in [
        CommonVideoModelEnum::Seedance2p0,
        CommonVideoModelEnum::Seedance2p0Fast,
        CommonVideoModelEnum::Seedance2p0Mini,
        CommonVideoModelEnum::Seedance2p5,
        CommonVideoModelEnum::Seedance2p5Ultra,
      ] {
        for generate_audio in [None, Some(true), Some(false)] {
          let request = OmniGenVideoCostAndGenerateRequest {
            model: Some(model),
            generate_audio,
            ..base_request()
          };
          let builder = hydrate_to_router_request(&request).expect("hydrate should succeed");
          assert_eq!(builder.generate_audio, None, "model: {model:?}, generate_audio: {generate_audio:?}");
        }
      }
    }

    #[test]
    fn other_models_keep_generate_audio() {
      for generate_audio in [None, Some(true), Some(false)] {
        let request = OmniGenVideoCostAndGenerateRequest {
          model: Some(CommonVideoModelEnum::Veo3p1),
          generate_audio,
          ..base_request()
        };
        let builder = hydrate_to_router_request(&request).expect("hydrate should succeed");
        assert_eq!(builder.generate_audio, generate_audio);
      }
    }
  }

  mod bitrate_hydration {
    use super::*;

    #[test]
    fn high_is_hydrated() {
      let request = OmniGenVideoCostAndGenerateRequest {
        bitrate: Some(CommonBitrateEnum::High),
        ..base_request()
      };
      let builder = hydrate_to_router_request(&request).expect("hydrate should succeed");
      assert_eq!(builder.bitrate, Some(RouterBitrate::High));
    }

    #[test]
    fn normal_is_hydrated() {
      let request = OmniGenVideoCostAndGenerateRequest {
        bitrate: Some(CommonBitrateEnum::Normal),
        ..base_request()
      };
      let builder = hydrate_to_router_request(&request).expect("hydrate should succeed");
      assert_eq!(builder.bitrate, Some(RouterBitrate::Normal));
    }

    #[test]
    fn none_stays_none() {
      let builder = hydrate_to_router_request(&base_request()).expect("hydrate should succeed");
      assert!(builder.bitrate.is_none());
    }
  }

  fn base_request() -> OmniGenVideoCostAndGenerateRequest {
    OmniGenVideoCostAndGenerateRequest {
      idempotency_token: None,
      model: Some(CommonVideoModelEnum::Seedance2p0),
      prompt: None,
      negative_prompt: None,
      start_frame_image_media_token: None,
      end_frame_image_media_token: None,
      reference_image_media_tokens: None,
      reference_video_media_tokens: None,
      reference_audio_media_tokens: None,
      reference_character_tokens: None,
      resolution: None,
      aspect_ratio: None,
      bitrate: None,
      maybe_output_format: None,
      quality: None,
      duration_seconds: None,
      video_batch_count: None,
      generate_audio: None,
      estimate_only: None,
    }
  }
}
