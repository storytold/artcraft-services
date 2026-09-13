use serde_derive::{Deserialize, Serialize};

/// Output video container. Currently supported by Kinovi Seedance 2.5 only.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouterVideoOutputFormat {
  Mp4,
  Mov,
}
