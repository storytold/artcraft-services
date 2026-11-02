use actix_http::body::MessageBody;
use actix_service::ServiceFactory;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{web, App, Error, HttpResponse};

use crate::http_server::endpoints::oauth::authorize::{oauth_authorize_get, oauth_authorize_post};
use crate::http_server::endpoints::oauth::clients::oauth_register_client_handler;
use crate::http_server::endpoints::oauth::grants::{oauth_grant_revoke, oauth_grants_get};
use crate::http_server::endpoints::oauth::revoke::oauth_revoke_handler;
use crate::http_server::endpoints::oauth::token::oauth_token_handler;

pub fn add_oauth_routes<T, B>(app: App<T>) -> App<T>
where
  B: MessageBody,
  T: ServiceFactory<ServiceRequest, Config = (), Response = ServiceResponse<B>, Error = Error, InitError = ()>,
{
  app.service(web::scope("/v1/oauth").service(web::resource("/token").route(web::post().to(oauth_token_handler)).route(web::head().to(|| HttpResponse::Ok()))).service(web::resource("/revoke").route(web::post().to(oauth_revoke_handler)).route(web::head().to(|| HttpResponse::Ok()))).service(web::resource("/clients").route(web::post().to(oauth_register_client_handler)).route(web::head().to(|| HttpResponse::Ok()))).service(web::resource("/authorize").route(web::get().to(oauth_authorize_get)).route(web::post().to(oauth_authorize_post)).route(web::head().to(|| HttpResponse::Ok()))).service(web::resource("/grants").route(web::get().to(oauth_grants_get)).route(web::head().to(|| HttpResponse::Ok()))).service(web::resource("/grants/{client_token}/revoke").route(web::post().to(oauth_grant_revoke)).route(web::head().to(|| HttpResponse::Ok()))))
}
