use actix_cors::Cors;
use crate::util::cloudflare_pages_branch_domain_matches::cloudflare_pages_branch_domain_matches;
use crate::util::netlify_branch_domain_matches::netlify_branch_domain_matches;
use domains::artcraft::{ARTCRAFT_WEBAPP, ARTCRAFT_WEBAPP_NEW};

pub fn add_artcraft_webapp(cors: Cors, _is_production: bool) -> Cors {
  cors
      // Actual domains
      .allowed_origin(ARTCRAFT_WEBAPP.https_link_without_path())
      .allowed_origin(ARTCRAFT_WEBAPP_NEW.https_link_without_path())
      // Cloudflare Pages project (app-new)
      .allowed_origin_fn(|origin, _req_head| {
        cloudflare_pages_branch_domain_matches(origin, "app-new.pages.dev")
      })
      // Netlify project
      .allowed_origin_fn(|origin, _req_head| {
        netlify_branch_domain_matches(origin, "artcraft-webapp.netlify.app")
      })
}

#[cfg(test)]
mod tests {
  use crate::testing::{assert_cloudflare_pages_project, assert_exact_https_origin, assert_netlify_project, isolated_cors};

  use super::add_artcraft_webapp;

  #[actix_rt::test]
  async fn webapp_domain() {
    for is_production in [true, false] {
      let cors = isolated_cors(add_artcraft_webapp, is_production);
      assert_exact_https_origin(&cors, "https://app.getartcraft.com").await;
      assert_exact_https_origin(&cors, "https://app-new.getartcraft.com").await;
    }
  }

  #[actix_rt::test]
  async fn cloudflare_pages_project() {
    let cors = isolated_cors(add_artcraft_webapp, true);
    assert_cloudflare_pages_project(&cors, "app-new.pages.dev").await;
  }

  #[actix_rt::test]
  async fn netlify_project() {
    let cors = isolated_cors(add_artcraft_webapp, true);
    assert_netlify_project(&cors, "artcraft-webapp.netlify.app").await;
  }
}
