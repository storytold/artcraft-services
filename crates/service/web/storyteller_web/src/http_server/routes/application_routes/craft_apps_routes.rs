use actix_http::body::MessageBody;
use actix_service::ServiceFactory;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{web, App, Error, HttpResponse};

use crate::http_server::endpoints::craft_apps::get_craft_apps_release_info_handler::get_craft_apps_release_info_handler;

pub fn add_craft_apps_routes<T, B>(app: App<T>) -> App<T>
where
  B: MessageBody,
  T: ServiceFactory<
    ServiceRequest,
    Config = (),
    Response = ServiceResponse<B>,
    Error = Error,
    InitError = (),
  >,
{
  app.service(
    web::scope("/v1/craft_apps")
      // Release info for ArtCraft Launcher (polled instead of GitHub).
      .service(
        web::resource("/release_info")
          .route(web::get().to(get_craft_apps_release_info_handler))
          .route(web::head().to(|| HttpResponse::Ok())),
      ),
  )
}
