use artcraft_api_defs::omni_gen::models::omni_gen_video_models::OmniGenVideoModelDetails;
use enums::common::generation::common_aspect_ratio::CommonAspectRatio;
use enums::common::generation::common_resolution::CommonResolution;
use enums::common::generation::common_video_model::CommonVideoModel;
use enums::common::generation::model_creator::ModelCreator;

pub fn wan_video_models() -> Vec<OmniGenVideoModelDetails> {
  [(CommonVideoModel::Wan3p0, "Wan 3.0"), (CommonVideoModel::Wan3p0Prime, "Wan 3.0 Prime")]
    .iter().copied()
    .map(|(model, name)| OmniGenVideoModelDetails {
      model,
      model_creator: Some(ModelCreator::Alibaba),
      full_name: Some(name.to_owned()),
      extra_info: Some("Reference videos and output must fit within 30 seconds combined. Keyframes and reference media are separate modes.".to_owned()),
      text_to_video_supported: Some(true),
      text_prompt_supported: Some(true),
      text_prompt_max_length: Some(20_000),
      starting_keyframe_supported: Some(true),
      ending_keyframe_supported: Some(true),
      image_references_supported: Some(true),
      image_references_max: Some(10),
      video_references_supported: Some(true),
      video_references_max: Some(5),
      video_references_max_total_duration_seconds: Some(15),
      audio_references_supported: Some(true),
      audio_references_max: Some(5),
      audio_references_max_total_duration_seconds: Some(15),
      show_generate_with_sound_toggle: Some(true),
      aspect_ratio_options: Some(vec![
        CommonAspectRatio::WideSixteenByNine, CommonAspectRatio::TallNineBySixteen,
        CommonAspectRatio::WideFourByThree, CommonAspectRatio::TallThreeByFour,
        CommonAspectRatio::Square,
      ]),
      aspect_ratio_default: Some(CommonAspectRatio::WideSixteenByNine),
      resolution_options: Some(vec![CommonResolution::FourEightyP, CommonResolution::SevenTwentyP, CommonResolution::TenEightyP]),
      resolution_default: Some(CommonResolution::SevenTwentyP),
      batch_size_options: Some(vec![1]),
      batch_size_default: Some(1),
      duration_seconds_min: Some(2),
      duration_seconds_max: Some(30),
      duration_seconds_default: Some(5),
      ..Default::default()
    }).collect()
}
