use enums::common::generation::common_video_model::CommonVideoModel;

use crate::http_server::common_responses::common_web_error::CommonWebError;

/// Keep the public batch limit aligned with ArtCraft billing before the
/// execution builder reaches Kinovi, whose Seedance API accepts larger batches.
pub(crate) fn validate_kinovi_batch_count(
  model: Option<CommonVideoModel>,
  video_batch_count: Option<u16>,
) -> Result<(), CommonWebError> {
  if matches!(model, Some(
    CommonVideoModel::Seedance2p5
      | CommonVideoModel::Seedance2p0
      | CommonVideoModel::Seedance2p0Fast
      | CommonVideoModel::Seedance2p0Mini
      | CommonVideoModel::HappyHorse1p0
  )) && !(1..=4).contains(&video_batch_count.unwrap_or(1)) {
    return Err(CommonWebError::BadInputWithSimpleMessage(
      "video_batch_count must be between 1 and 4 for this model".to_string(),
    ));
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use artcraft_api_defs::omni_api::generate_requests::omni_api_video_generate_request::OmniApiVideoGenerateRequest;
  use artcraft_api_defs::omni_gen::cost_and_generate_requests::omni_gen_video_cost_and_generate_request::OmniGenVideoCostAndGenerateRequest;
  use serde_json::json;

  use crate::http_server::endpoints::omni_api::shared_utils::video::validate_video_request::validate_video_request as validate_omni_api;
  use crate::http_server::endpoints::omni_gen::shared_utils::video::validate_video_request::validate_video_request as validate_omni_gen;

  use super::*;

  const MODELS: [CommonVideoModel; 5] = [
    CommonVideoModel::Seedance2p5,
    CommonVideoModel::Seedance2p0,
    CommonVideoModel::Seedance2p0Fast,
    CommonVideoModel::Seedance2p0Mini,
    CommonVideoModel::HappyHorse1p0,
  ];

  #[test]
  fn both_endpoints_accept_batches_one_through_four_and_default() {
    for model in MODELS {
      for batch in [None, Some(1), Some(2), Some(3), Some(4)] {
        for result in validate_both(model, batch) {
          assert!(result.is_ok(), "{:?}, batch {:?}: {:?}", model, batch, result);
        }
      }
    }
  }

  #[test]
  fn both_endpoints_reject_zero_and_batches_above_four() {
    for model in MODELS {
      for batch in [0, 5, 6, 7, 8, u16::MAX] {
        for result in validate_both(model, Some(batch)) {
          assert!(matches!(result, Err(CommonWebError::BadInputWithSimpleMessage(_))),
            "{:?}, batch {}", model, batch);
        }
      }
    }
  }

  #[test]
  fn other_models_keep_their_existing_validation() {
    for model in [
      CommonVideoModel::Seedance2p5Ultra,
      CommonVideoModel::Seedance2p5Preview,
      CommonVideoModel::Seedance2p0BytePlus,
      CommonVideoModel::Seedance2p0BytePlusFast,
      CommonVideoModel::Seedance2p0BytePlusMini,
      CommonVideoModel::Seedance2p0BytePlusUltra,
      CommonVideoModel::Seedance2p0BytePlusUltraFast,
      CommonVideoModel::Seedance2p0BytePlusUltraMini,
      CommonVideoModel::Veo3p1,
    ] {
      for result in validate_both(model, Some(8)) {
        assert!(result.is_ok(), "{:?}: {:?}", model, result);
      }
    }
  }

  fn validate_both(model: CommonVideoModel, batch: Option<u16>) -> [Result<(), CommonWebError>; 2] {
    let json = json!({"model": model, "video_batch_count": batch});
    let omni_gen: OmniGenVideoCostAndGenerateRequest = serde_json::from_value(json.clone()).unwrap();
    let omni_api: OmniApiVideoGenerateRequest = serde_json::from_value(json).unwrap();
    [validate_omni_gen(&omni_gen), validate_omni_api(&omni_api)]
  }
}
