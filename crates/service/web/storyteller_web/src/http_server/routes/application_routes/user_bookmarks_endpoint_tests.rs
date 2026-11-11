//! Real bookmark routes, signed sessions, and migrated local database fixtures.
//! Each test bookmarks its own fresh media file and reads the stored
//! `bookmark_count` back through the media file endpoint.
//!
//! These tests take the serial lock. The first bookmark of an entity locks a
//! gap (the `FOR UPDATE` lookup finds no rows) and then inserts into it, in
//! both `user_bookmarks` and `entity_stats`. While those tables are nearly
//! empty, parallel tests bookmarking different new entities share the same
//! gaps and can deadlock, which turns one request into a 500.
use std::fmt::Debug;
use std::sync::Arc;

use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::{test, web, App};
use mysql_testing::fixtures::media_files::create_test_video_media_file;
use mysql_testing::fixtures::users::{create_test_user, TestUser};
use mysql_testing::pool::create_test_pool;
use mysql_testing::serial::acquire_serial_test_lock;
use serde_json::{json, Value};
use tokens::tokens::media_files::MediaFileToken;

use super::media_files_routes::add_media_file_routes;
use super::user_bookmarks_routes::add_user_bookmarks_routes;
use crate::http_server::endpoints::omni_gen::generate::video::tests::support::build_test_server_state;
use crate::state::server_state::ServerState;

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn each_users_bookmark_is_counted() {
  let _serial = acquire_serial_test_lock().await;
  let (state, pool) = test_state().await;
  let app = test::init_service(add_user_bookmarks_routes(add_media_file_routes(
    App::new().app_data(web::Data::new(state.clone())),
  )))
  .await;
  let alice = create_test_user(&pool).await.unwrap();
  let bob = create_test_user(&pool).await.unwrap();
  let media_file = create_test_video_media_file(&pool, &alice.user_token, None).await.unwrap();

  create_bookmark(&app, &state, &alice, &media_file).await;
  create_bookmark(&app, &state, &bob, &media_file).await;

  assert_eq!(bookmark_count(&app, &media_file).await, 2);
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn deleting_a_bookmark_removes_only_that_users_count() {
  let _serial = acquire_serial_test_lock().await;
  let (state, pool) = test_state().await;
  let app = test::init_service(add_user_bookmarks_routes(add_media_file_routes(
    App::new().app_data(web::Data::new(state.clone())),
  )))
  .await;
  let alice = create_test_user(&pool).await.unwrap();
  let bob = create_test_user(&pool).await.unwrap();
  let media_file = create_test_video_media_file(&pool, &alice.user_token, None).await.unwrap();

  create_bookmark(&app, &state, &alice, &media_file).await;
  let bob_bookmark = create_bookmark(&app, &state, &bob, &media_file).await;
  delete_bookmark(&app, &state, &bob, &bob_bookmark).await;
  assert_eq!(bookmark_count(&app, &media_file).await, 1);

  // Deleting an already-deleted bookmark changes nothing.
  delete_bookmark(&app, &state, &bob, &bob_bookmark).await;
  assert_eq!(bookmark_count(&app, &media_file).await, 1);
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn deleting_someone_elses_bookmark_does_not_change_the_count() {
  let _serial = acquire_serial_test_lock().await;
  let (state, pool) = test_state().await;
  let app = test::init_service(add_user_bookmarks_routes(add_media_file_routes(
    App::new().app_data(web::Data::new(state.clone())),
  )))
  .await;
  let alice = create_test_user(&pool).await.unwrap();
  let mallory = create_test_user(&pool).await.unwrap();
  let media_file = create_test_video_media_file(&pool, &alice.user_token, None).await.unwrap();

  let alice_bookmark = create_bookmark(&app, &state, &alice, &media_file).await;
  delete_bookmark(&app, &state, &mallory, &alice_bookmark).await;

  assert_eq!(bookmark_count(&app, &media_file).await, 1);
}

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn repeated_bookmark_and_delete_count_once() {
  let _serial = acquire_serial_test_lock().await;
  let (state, pool) = test_state().await;
  let app = test::init_service(add_user_bookmarks_routes(add_media_file_routes(
    App::new().app_data(web::Data::new(state.clone())),
  )))
  .await;
  let alice = create_test_user(&pool).await.unwrap();
  let media_file = create_test_video_media_file(&pool, &alice.user_token, None).await.unwrap();

  // NB: The second create returns a freshly generated token rather than the stored one,
  // so delete with the token from the first.
  let bookmark = create_bookmark(&app, &state, &alice, &media_file).await;
  create_bookmark(&app, &state, &alice, &media_file).await;
  assert_eq!(bookmark_count(&app, &media_file).await, 1);

  delete_bookmark(&app, &state, &alice, &bookmark).await;
  delete_bookmark(&app, &state, &alice, &bookmark).await;
  assert_eq!(bookmark_count(&app, &media_file).await, 0);

  create_bookmark(&app, &state, &alice, &media_file).await;
  assert_eq!(bookmark_count(&app, &media_file).await, 1);
}

async fn test_state() -> (Arc<ServerState>, sqlx::MySqlPool) {
  let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
  let pool = create_test_pool().await;
  let state = build_test_server_state(pool.clone(), "http://127.0.0.1:1".to_string());
  (Arc::new(state), pool)
}

/// Bookmark `media_file` as `user` and return the bookmark token from the response.
async fn create_bookmark<S, B>(app: &S, state: &ServerState, user: &TestUser, media_file: &MediaFileToken) -> String
where
  S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
  B: MessageBody,
  B::Error: Debug,
{
  let body = json!({"entity_type": "media_file", "entity_token": media_file.as_str()});
  let (status, response) = post(app, "/v1/user_bookmarks/create", &session(state, user), body).await;
  assert_eq!(status, StatusCode::OK, "create bookmark failed: {response}");
  response["user_bookmark_token"].as_str().unwrap().to_string()
}

async fn delete_bookmark<S, B>(app: &S, state: &ServerState, user: &TestUser, bookmark_token: &str)
where
  S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
  B: MessageBody,
  B::Error: Debug,
{
  let path = format!("/v1/user_bookmarks/delete/{bookmark_token}");
  let (status, response) = post(app, &path, &session(state, user), json!({})).await;
  assert_eq!(status, StatusCode::OK, "delete bookmark failed: {response}");
}

/// The stored `entity_stats.bookmark_count`, as the media file endpoint reports it.
async fn bookmark_count<S, B>(app: &S, media_file: &MediaFileToken) -> u64
where
  S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
  B: MessageBody,
  B::Error: Debug,
{
  let request = test::TestRequest::get().uri(&format!("/v1/media_files/file/{}", media_file.as_str()));
  let response = test::call_service(app, request.to_request()).await;
  assert_eq!(response.status(), StatusCode::OK);
  let body: Value = test::read_body_json(response).await;
  body["media_file"]["stats"]["bookmark_count"].as_u64().unwrap()
}

fn session(state: &ServerState, user: &TestUser) -> String {
  state
    .session_cookie_manager
    .create_cookie(&user.session_token, &user.user_token)
    .unwrap()
    .value()
    .to_string()
}

async fn post<S, B>(app: &S, path: &str, session: &str, body: Value) -> (StatusCode, Value)
where
  S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
  B: MessageBody,
  B::Error: Debug,
{
  let request = test::TestRequest::post()
    .uri(path)
    .insert_header(("session", session))
    .set_json(body);
  let response = test::call_service(app, request.to_request()).await;
  let status = response.status();
  (status, test::read_body_json(response).await)
}
