use actix_http::header::HeaderValue;
use log::warn;
use url::{Host, Url};

pub fn cloudflare_pages_branch_domain_matches(origin: &HeaderValue, pages_hostname: &str) -> bool {
  let maybe_url = origin.to_str().map(|origin| Url::parse(origin));

  let url = match maybe_url {
    Ok(Ok(url)) => url,
    _ => {
      warn!("Invalid origin: {:?}", origin);
      return false;
    }
  };

  match url.host() {
    Some(Host::Domain(domain)) => {
      if domain == pages_hostname {
        return true;
      }

      let branch_suffix = format!(".{}", pages_hostname);
      domain.ends_with(&branch_suffix)
    }
    _ => false,
  }
}
