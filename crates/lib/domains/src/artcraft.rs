//! ArtCraft domains.

use crate::{Domain, domain};

/// Public marketing website (Next.js).
pub const ARTCRAFT_WEBSITE: Domain = domain!("getartcraft.com");
pub const ARTCRAFT_WEBSITE_WWW: Domain = domain!("www.getartcraft.com");

/// Browser web application.
pub const ARTCRAFT_WEBAPP: Domain = domain!("app.getartcraft.com");
pub const ARTCRAFT_WEBAPP_NEW: Domain = domain!("app-new.getartcraft.com");

/// Desktop app landing page.
pub const ARTCRAFT_DESKTOP: Domain = domain!("desktop.getartcraft.com");

/// HTTP API.
pub const ARTCRAFT_API: Domain = domain!("api.getartcraft.com");

pub const ARTCRAFT_2D: Domain = domain!("2d.getartcraft.com");
pub const ARTCRAFT_3D: Domain = domain!("3d.getartcraft.com");

/// Alternate marketing domain.
pub const ARTCRAFT_AI: Domain = domain!("artcraft.ai");
pub const ARTCRAFT_AI_WWW: Domain = domain!("www.artcraft.ai");

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn website() {
    assert_eq!(ARTCRAFT_WEBSITE.https_link_without_path(), "https://getartcraft.com");
    assert_eq!(ARTCRAFT_WEBSITE_WWW.https_link_without_path(), "https://www.getartcraft.com");
  }

  #[test]
  fn webapp() {
    assert_eq!(ARTCRAFT_WEBAPP.https_link_without_path(), "https://app.getartcraft.com");
    assert_eq!(ARTCRAFT_WEBAPP_NEW.https_link_without_path(), "https://app-new.getartcraft.com");
  }

  #[test]
  fn desktop() {
    assert_eq!(ARTCRAFT_DESKTOP.bare_domain(), "desktop.getartcraft.com");
    assert_eq!(ARTCRAFT_DESKTOP.https_link_without_path(), "https://desktop.getartcraft.com");
    assert_eq!(ARTCRAFT_DESKTOP.https_link_with_root_path(), "https://desktop.getartcraft.com/");
  }

  #[test]
  fn api() {
    assert_eq!(ARTCRAFT_API.https_link_without_path(), "https://api.getartcraft.com");
  }

  #[test]
  fn two_d_and_three_d() {
    assert_eq!(ARTCRAFT_2D.https_link_without_path(), "https://2d.getartcraft.com");
    assert_eq!(ARTCRAFT_3D.https_link_without_path(), "https://3d.getartcraft.com");
  }

  #[test]
  fn artcraft_ai() {
    assert_eq!(ARTCRAFT_AI.https_link_without_path(), "https://artcraft.ai");
    assert_eq!(ARTCRAFT_AI_WWW.https_link_without_path(), "https://www.artcraft.ai");
  }

  #[cfg(feature = "url")]
  #[test]
  fn desktop_url() {
    assert_eq!(ARTCRAFT_DESKTOP.https_url().as_str(), "https://desktop.getartcraft.com/");
  }
}
