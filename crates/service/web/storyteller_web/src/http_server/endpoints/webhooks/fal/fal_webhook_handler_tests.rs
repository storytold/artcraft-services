//! End-to-end tests of the FAL webhook signature check: real HTTP requests through the real
//! route and handler, with a test MySQL database behind them.
//!
//! These are DATABASE TESTS (see `mysql_testing`). Skip them on machines without MySQL with
//! `--features skip_database_tests`.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::http::StatusCode;
use actix_web::{test, web, App};
use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use fal_client::webhook_signature::fal_jwks_cache::FalJwksCache;
use fal_client::webhook_signature::fal_webhook_signature_headers::{REQUEST_ID_HEADER, SIGNATURE_HEADER, TIMESTAMP_HEADER, USER_ID_HEADER};
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};
use sha2::{Digest, Sha256};

use crate::http_server::endpoints::omni_gen::generate::video::tests::support::build_test_server_state;
use crate::http_server::endpoints::webhooks::fal::fal_webhook_handler::fal_webhook_handler;
use crate::http_server::endpoints::webhooks::fal::signature::fal_webhook_signature_mode::FalWebhookSignatureMode;
use crate::http_server::endpoints::webhooks::fal::signature::fal_webhook_signature_settings::FalWebhookSignatureSettings;

const WEBHOOK_PATH: &str = "/v1/webhooks/fal";

/// No job has this FAL request ID, so a webhook that gets past the signature check is answered
/// 404 by the job lookup. That shows the request reached the database, without needing a job.
const UNKNOWN_REQUEST_ID: &str = "00000000-0000-0000-0000-signature-test";

mod enforce {
  use super::*;

  #[actix_web::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn an_unsigned_webhook_is_rejected_before_the_job_lookup() {
    let fal = FakeFal::new();

    let status = post(&fal, FalWebhookSignatureMode::Enforce, unsigned_webhook()).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
  }

  #[actix_web::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_webhook_signed_by_another_key_is_rejected() {
    let fal = FakeFal::new();
    let forger = FakeFal::new();

    let status = post(&fal, FalWebhookSignatureMode::Enforce, forger.signed_webhook(&webhook_body())).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
  }

  #[actix_web::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn a_genuine_webhook_goes_on_to_the_job_lookup() {
    let fal = FakeFal::new();

    let status = post(&fal, FalWebhookSignatureMode::Enforce, fal.signed_webhook(&webhook_body())).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
  }
}

mod report {
  use super::*;

  #[actix_web::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn an_unsigned_webhook_is_still_processed() {
    let fal = FakeFal::new();

    let status = post(&fal, FalWebhookSignatureMode::Report, unsigned_webhook()).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
  }
}

/// A webhook request: its headers and raw body.
struct Webhook {
  headers: Vec<(&'static str, String)>,
  body: Vec<u8>,
}

/// Plays FAL: an ED25519 key pair, published as a JWKS, that signs webhooks the way FAL does.
struct FakeFal {
  key_pair: Ed25519KeyPair,
}

impl FakeFal {
  fn new() -> Self {
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).expect("generate key");
    Self { key_pair: Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("parse key") }
  }

  fn jwks(&self) -> String {
    let x = URL_SAFE.encode(self.key_pair.public_key().as_ref());
    format!(r#"{{"keys":[{{"kty":"OKP","crv":"Ed25519","x":"{x}","use":"sig"}}]}}"#)
  }

  fn signed_webhook(&self, body: &[u8]) -> Webhook {
    let user_id = "fal-test-user";
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_secs().to_string();
    let message = format!("{UNKNOWN_REQUEST_ID}\n{user_id}\n{timestamp}\n{}", hex::encode(Sha256::digest(body)));
    let signature = hex::encode(self.key_pair.sign(message.as_bytes()).as_ref());

    Webhook {
      headers: vec![
        (REQUEST_ID_HEADER, UNKNOWN_REQUEST_ID.to_string()),
        (USER_ID_HEADER, user_id.to_string()),
        (TIMESTAMP_HEADER, timestamp),
        (SIGNATURE_HEADER, signature),
      ],
      body: body.to_vec(),
    }
  }
}

fn webhook_body() -> Vec<u8> {
  format!(r#"{{"request_id":"{UNKNOWN_REQUEST_ID}","gateway_request_id":"{UNKNOWN_REQUEST_ID}","status":"OK","payload":{{"images":[]}}}}"#).into_bytes()
}

fn unsigned_webhook() -> Webhook {
  Webhook { headers: Vec::new(), body: webhook_body() }
}

/// POST `webhook` to the FAL webhook handler of a server that trusts `fal`'s key, in `mode`.
async fn post(fal: &FakeFal, mode: FalWebhookSignatureMode, webhook: Webhook) -> StatusCode {
  let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

  let pool = mysql_testing::pool::create_test_pool().await;
  let mut server_state = build_test_server_state(pool, "http://localhost".to_string());

  let jwks = fal.jwks();
  server_state.inference_providers.fal.webhook_signature = FalWebhookSignatureSettings {
    mode,
    jwks_cache: Arc::new(FalJwksCache::with_fetcher(Arc::new(move || {
      let jwks = jwks.clone();
      Box::pin(async move { Ok(jwks) })
    }))),
  };

  let app = test::init_service(
    App::new()
      .app_data(web::Data::new(Arc::new(server_state)))
      .route(WEBHOOK_PATH, web::post().to(fal_webhook_handler))
  ).await;

  let mut request = test::TestRequest::post().uri(WEBHOOK_PATH);
  for (name, value) in webhook.headers {
    request = request.insert_header((name, value));
  }

  test::call_service(&app, request.set_payload(webhook.body).to_request()).await.status()
}
