//! Real weight routes, signed sessions, and migrated local database fixtures.
//! The test deletes and undeletes its own fresh weight, then checks whether
//! a logged-out visitor can see it.
use std::fmt::Debug;
use std::sync::Arc;

use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::{test, web, App};
use mysql_queries::queries::users::user_roles::set_user_role::set_user_role;
use mysql_testing::fixtures::model_weights::create_test_model_weight;
use mysql_testing::fixtures::users::{create_test_user, TestUser};
use mysql_testing::pool::create_test_pool;
use serde_json::{json, Value};
use tokens::tokens::model_weights::ModelWeightToken;

use super::weights_routes::add_weights_routes;
use crate::http_server::endpoints::omni_gen::generate::video::tests::support::build_test_server_state;
use crate::state::server_state::ServerState;

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn moderator_can_undo_a_moderator_delete() {
  let (state, pool) = test_state().await;
  let app = test::init_service(add_weights_routes(App::new().app_data(web::Data::new(state.clone())))).await;
  let owner = create_test_user(&pool).await.unwrap();
  let moderator = create_test_user(&pool).await.unwrap();
  set_user_role(&moderator.user_token, "mod", &pool).await.unwrap();
  let weight = create_test_model_weight(&pool, &owner.user_token).await.unwrap();

  set_deleted_as_mod(&app, &state, &moderator, &weight, true).await;
  assert_eq!(public_status(&app, &weight).await, StatusCode::NOT_FOUND);

  set_deleted_as_mod(&app, &state, &moderator, &weight, false).await;
  assert_eq!(public_status(&app, &weight).await, StatusCode::OK);
}

async fn test_state() -> (Arc<ServerState>, sqlx::MySqlPool) {
  let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
  let pool = create_test_pool().await;
  let state = build_test_server_state(pool.clone(), "http://127.0.0.1:1".to_string());
  (Arc::new(state), pool)
}

async fn set_deleted_as_mod<S, B>(
  app: &S,
  state: &ServerState,
  moderator: &TestUser,
  weight: &ModelWeightToken,
  set_delete: bool,
)
where
  S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
  B: MessageBody,
  B::Error: Debug,
{
  let session = state
    .session_cookie_manager
    .create_cookie(&moderator.session_token, &moderator.user_token)
    .unwrap()
    .value()
    .to_string();
  let request = test::TestRequest::delete()
    .uri(&format!("/v1/weights/weight/{}", weight.as_str()))
    .insert_header(("session", session))
    .set_json(json!({"set_delete": set_delete, "as_mod": true}));
  let response = test::call_service(app, request.to_request()).await;
  let status = response.status();
  let body: Value = test::read_body_json(response).await;
  assert_eq!(status, StatusCode::OK, "set_delete={set_delete} as mod failed: {body}");
}

/// Whether a logged-out visitor can see the weight.
async fn public_status<S, B>(app: &S, weight: &ModelWeightToken) -> StatusCode
where
  S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
  B: MessageBody,
  B::Error: Debug,
{
  let request = test::TestRequest::get().uri(&format!("/v1/weights/weight/{}", weight.as_str()));
  test::call_service(app, request.to_request()).await.status()
}
