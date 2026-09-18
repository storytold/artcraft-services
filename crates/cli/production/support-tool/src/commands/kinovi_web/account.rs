use clap::ValueEnum;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum KinoviAccount {
  Volcengine,
  BytePlus,
  BytePlusUltra,
}

impl KinoviAccount {
  /// Cookie configuration and legacy fallbacks used by seedance2-pro-job.
  pub fn cookie_env_vars(self) -> &'static [&'static str] {
    match self {
      Self::Volcengine => &["SEEDANCE2PRO_VOLCENGINE_COOKIES", "SEEDANCE2PRO_COOKIES"],
      Self::BytePlus => &["SEEDANCE2PRO_BYTEPLUS_COOKIES", "SEEDANCE2PRO_ALT_COOKIES"],
      Self::BytePlusUltra => &["SEEDANCE2PRO_BYTEPLUS_ULTRA_COOKIES"],
    }
  }
}
