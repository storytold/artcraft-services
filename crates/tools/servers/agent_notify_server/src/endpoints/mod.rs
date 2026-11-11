pub mod alert_handlers;
pub mod loop_handlers;
pub mod root_handler;
pub mod state_handler;
pub mod stop_handler;

use actix_web::guard::GuardContext;

/// Rejects requests a browser made on behalf of another site, so a web page cannot drive the
/// local listener (CSRF). curl and the agent script send neither header; typing a URL into the
/// address bar sends `Sec-Fetch-Site: none` and still works.
pub fn not_cross_site(ctx: &GuardContext<'_>) -> bool {
  let headers = ctx.head().headers();
  let site = headers.get("sec-fetch-site").and_then(|v| v.to_str().ok());
  !headers.contains_key("origin") && !matches!(site, Some("cross-site" | "same-site"))
}

#[cfg(test)]
mod tests {
  use actix_web::http::StatusCode;
  use actix_web::{App, HttpResponse, guard, test, web};

  use super::not_cross_site;

  #[actix_web::test]
  async fn mutation_routes_reject_cross_site_browser_requests() {
    let app = test::init_service(App::new().route(
      "/stop",
      web::post().guard(guard::fn_guard(not_cross_site)).to(HttpResponse::Ok),
    ))
    .await;

    let status = |req| async { test::call_service(&app, req).await.status() };

    // Plain POST from curl / the agent script.
    assert_eq!(status(test::TestRequest::post().uri("/stop").to_request()).await, StatusCode::OK);
    // GET navigation (link, img, redirect) is not a mutation route anymore.
    assert_eq!(status(test::TestRequest::get().uri("/stop").to_request()).await, StatusCode::NOT_FOUND);
    // Cross-site form POST from another web page.
    let req = test::TestRequest::post()
      .uri("/stop")
      .insert_header(("origin", "https://evil.example"))
      .insert_header(("sec-fetch-site", "cross-site"))
      .to_request();
    assert_eq!(status(req).await, StatusCode::NOT_FOUND);
    let req = test::TestRequest::post()
      .uri("/stop")
      .insert_header(("sec-fetch-site", "same-site"))
      .to_request();
    assert_eq!(status(req).await, StatusCode::NOT_FOUND);
  }
}
