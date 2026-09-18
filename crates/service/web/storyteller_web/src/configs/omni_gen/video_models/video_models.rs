use super::by_type::wan_video_models::wan_video_models;
use artcraft_api_defs::omni_gen::models::omni_gen_video_models::{OmniGenVideoModelDetails, OmniGenVideoModelProviderDetails, OmniGenVideoModelsResponse, OmniGenVideoProviderModelDetails};
use enums::common::generation::common_video_model::CommonVideoModel;
use enums::common::generation_provider::GenerationProvider;
use once_cell::sync::Lazy;

use super::by_type::flux_video_models::flux_video_models;
use super::by_type::grok_video_models::grok_video_models;
use super::by_type::happy_horse_video_models::happy_horse_video_models;
use super::by_type::kling_video_models::{kling_disabled_video_models, kling_video_models};
use super::by_type::minimax_video_models::minimax_video_models;
use super::by_type::seedance_1x_video_models::{seedance_1p0_video_models, seedance_1p5_video_models};
use super::by_type::seedance_2p0_video_models::seedance_2p0_video_models;
use super::by_type::seedance_2p5_video_models::seedance_2p5_video_models;
use super::by_type::sora_video_models::sora_video_models;
use super::by_type::veo_video_models::veo_video_models;
use super::by_type::vidu_video_models::vidu_video_models;

pub const OMNI_GEN_VIDEO_MODELS_AND_PROVIDERS: Lazy<OmniGenVideoModelsResponse> = Lazy::new(|| {
  let models = build_omni_gen_video_models();
  let providers= build_omni_gen_video_model_providers();
  OmniGenVideoModelsResponse {
    success: true,
    models,
    providers,
  }
});

fn build_omni_gen_video_models() -> Vec<OmniGenVideoModelDetails> {
  let mut models = Vec::new();

  models.extend(flux_video_models());
  models.extend(grok_video_models());
  models.extend(happy_horse_video_models());
  models.extend(kling_video_models());
  models.extend(kling_disabled_video_models());
  models.extend(minimax_video_models());
  models.extend(seedance_1p0_video_models());
  models.extend(seedance_1p5_video_models());
  models.extend(seedance_2p0_video_models());
  models.extend(seedance_2p5_video_models());
  models.extend(sora_video_models());
  models.extend(veo_video_models());
  models.extend(vidu_video_models());
  models.extend(wan_video_models());

  models
}

fn build_omni_gen_video_model_providers() -> Vec<OmniGenVideoModelProviderDetails> {
  let mut providers = Vec::new();

  providers.push(OmniGenVideoModelProviderDetails {
    provider: GenerationProvider::Artcraft,
    models: vec![
      OmniGenVideoProviderModelDetails { model: CommonVideoModel::Wan3p0, overrides: None },
      OmniGenVideoProviderModelDetails { model: CommonVideoModel::Wan3p0Prime, overrides: None },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Seedance1p5Pro,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Seedance2p0,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Seedance10Lite,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Sora2,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Sora2Pro,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Veo2,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Veo3,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Veo3Fast,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Veo3p1,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Veo3p1Fast,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Veo3p1Lite,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::ViduQ3,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::ViduQ3Turbo,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::MinimaxH3,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::MinimaxH3Turbo,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::MinimaxH3Ultra,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Flux3,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Flux3Draft,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Kling16Pro,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Kling21Pro,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Kling21Master,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Kling2p5TurboPro,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Kling2p6Pro,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Kling3p0Pro,
        overrides: None,
      },
      OmniGenVideoProviderModelDetails {
        model: CommonVideoModel::Kling3p0Standard,
        overrides: None,
      },
    ],
  });

  providers
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn kinovi_batch_options_are_one_through_four() {
    let models = build_omni_gen_video_models();
    for model in [
      CommonVideoModel::Seedance2p5,
      CommonVideoModel::Seedance2p0,
      CommonVideoModel::Seedance2p0Fast,
      CommonVideoModel::Seedance2p0Mini,
      CommonVideoModel::Seedance2p5Ultra,
      CommonVideoModel::Seedance2p0BytePlus,
      CommonVideoModel::Seedance2p0BytePlusFast,
      CommonVideoModel::Seedance2p0BytePlusMini,
      CommonVideoModel::Seedance2p0BytePlusUltra,
      CommonVideoModel::Seedance2p0BytePlusUltraFast,
      CommonVideoModel::Seedance2p0BytePlusUltraMini,
      CommonVideoModel::HappyHorse1p0,
    ] {
      let details = models.iter().find(|details| details.model == model).unwrap();
      assert_eq!(details.batch_size_options.as_deref(), Some([1, 2, 3, 4].as_slice()), "{model:?}");
      assert_eq!(details.batch_size_default, Some(1), "{model:?}");
    }
  }
}
