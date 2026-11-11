use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::dev::Service;
use actix_web::Error;
use futures_util::future::{Either, err, Ready};

use crate::middleware::disabled_endpoint_filter::disabled_endpoints::disabled_endpoints::DisabledEndpoints;
use crate::middleware::disabled_endpoint_filter::disabled_error::DisabledError;

//use std::task::{Context, Poll};

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.

pub struct DisabledEndpointFilterMiddleware<S> {
  pub (crate) service: S,
  pub (crate) disabled_endpoints: DisabledEndpoints,
}

impl<S, B> Service<ServiceRequest> for DisabledEndpointFilterMiddleware<S>
  where
      S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
      S::Future: 'static,
{
  type Response = ServiceResponse<B>;
  type Error = Error;
  type Future = Either<S::Future, Ready<Result<Self::Response, Self::Error>>>;

  //// alternatively(?), actix_service::forward_ready!(service);
  //fn poll_ready(&self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
  //  self.service.poll_ready(cx)
  //}

  actix_service::forward_ready!(service);

  fn call(&self, request: ServiceRequest) -> Self::Future {
    // NB: Match on the path actix routes with (percent-decoded), not the raw URI path,
    // so an encoded spelling like `/a%2Fb` or `/%61pi` can't slip past the filter.
    let endpoint = request.match_info().as_str().to_string();

    // NB: Fail open.
    // We don't want our service to explode because we can't read our configs.
    let is_disabled = self.disabled_endpoints.endpoint_is_disabled(&endpoint);

    if is_disabled {
      Either::Right(err(Error::from(DisabledError {})))
    } else {
      Either::Left(self.service.call(request))
    }
  }
}
