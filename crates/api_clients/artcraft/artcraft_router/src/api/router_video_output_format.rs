use serde_derive::{Deserialize, Serialize};

/// Output video container. Supported by Seedance 2.5 on Kinovi and Artcraft.
/// Other models/providers ignore this option.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouterVideoOutputFormat {
  Mp4,
  Mov,
}
