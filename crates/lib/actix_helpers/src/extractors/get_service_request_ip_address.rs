use actix_web::dev::ServiceRequest;

use crate::extractors::get_ip_from_header::get_ip_from_header;

pub fn get_service_request_ip_address(request: &ServiceRequest) -> String {
  let headers = request.headers();
  let maybe_x_forwarded = get_ip_from_header(headers, "x-forwarded-for");
  let maybe_forwarded = get_ip_from_header(headers, "forwarded");

  let maybe_ip = maybe_x_forwarded.or(maybe_forwarded);

  match maybe_ip {
    Some(ip_address) => ip_address,
    None => {
      // If we're running without the upstream Rust proxy, we can grab 'x-forwarded-for', which is
      // populated by the DigitalOcean load balancer.
      // `SocketAddr::ip()` keeps native IPv6 addresses intact (splitting "[::1]:80" on ':' did not).
      request.peer_addr().map(|addr| addr.ip().to_string()).unwrap_or_default()
    },
  }
}
