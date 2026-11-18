use actix_cors::Cors;
use crate::util::netlify_branch_domain_matches::netlify_branch_domain_matches;

pub fn add_artcraft_admin_dashboard(cors: Cors, _is_production: bool) -> Cors {
  cors
      // Production domain (Cloudflare Worker).
      .allowed_origin("https://dashboard.artcraft.team")
      // Legacy Netlify project, kept as the rollback target.
      .allowed_origin("https://artcraft-dashboard.netlify.app")
      .allowed_origin_fn(|origin, _req_head| {
        netlify_branch_domain_matches(origin, "artcraft-dashboard.netlify.app")
      })
}

#[cfg(test)]
mod tests {
  use crate::testing::{assert_exact_https_origin, assert_netlify_project, isolated_cors};

  use super::add_artcraft_admin_dashboard;

  #[actix_rt::test]
  async fn dashboard_domain() {
    for is_production in [true, false] {
      let cors = isolated_cors(add_artcraft_admin_dashboard, is_production);
      assert_exact_https_origin(&cors, "https://dashboard.artcraft.team").await;
    }
  }

  #[actix_rt::test]
  async fn netlify_project() {
    for is_production in [true, false] {
      let cors = isolated_cors(add_artcraft_admin_dashboard, is_production);
      assert_netlify_project(&cors, "artcraft-dashboard.netlify.app").await;
    }
  }
}
