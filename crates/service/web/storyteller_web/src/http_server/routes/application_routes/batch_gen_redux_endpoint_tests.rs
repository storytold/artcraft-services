//! Real `/v1/media_files/batch_gen_redux/{token}` route, signed sessions, and migrated local
//! database fixtures. One batch holds a public and a private file of the same creator.
use std::fmt::Debug;
use std::sync::Arc;

use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::{test, web, App};
use bucket_paths::legacy::typified_paths::public::media_files::bucket_file_path::MediaFileBucketPath;
use enums::by_table::media_files::media_file_class::MediaFileClass;
use enums::by_table::media_files::media_file_origin_category::MediaFileOriginCategory;
use enums::by_table::media_files::media_file_origin_product_category::MediaFileOriginProductCategory;
use enums::by_table::media_files::media_file_type::MediaFileType;
use enums::common::visibility::Visibility;
use mysql_queries::queries::media_files::create::insert_builder::media_file_insert_builder::MediaFileInsertBuilder;
use mysql_testing::fixtures::users::{create_test_user, TestUser};
use mysql_testing::pool::create_test_pool;
use serde_json::Value;
use sqlx::MySqlPool;
use tokens::tokens::batch_generations::BatchGenerationToken;
use tokens::tokens::media_files::MediaFileToken;
use tokens::tokens::users::UserToken;

use super::media_files_routes::add_media_file_routes;
use crate::http_server::endpoints::omni_gen::generate::video::tests::support::build_test_server_state;
use crate::state::server_state::ServerState;

#[actix_web::test]
#[cfg_attr(feature = "skip_database_tests", ignore)]
async fn a_batch_lists_its_private_files_only_to_their_creator() {
  let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
  let pool = create_test_pool().await;
  let alice = create_test_user(&pool).await.unwrap();
  let bob = create_test_user(&pool).await.unwrap();
  let batch = BatchGenerationToken::generate();
  let public_file = create_batch_media_file(&pool, &alice.user_token, &batch, Visibility::Public).await;
  let private_file = create_batch_media_file(&pool, &alice.user_token, &batch, Visibility::Private).await;

  let state = Arc::new(build_test_server_state(pool.clone(), "http://127.0.0.1:1".to_string()));
  let app = test::init_service(add_media_file_routes(
    App::new().app_data(web::Data::new(state.clone())),
  ))
  .await;

  let anonymous = listed_tokens(&app, &batch, None).await;
  assert_eq!(anonymous, vec![public_file.as_str().to_string()], "anonymous viewer");

  let other_user = listed_tokens(&app, &batch, Some(&session(&state, &bob))).await;
  assert_eq!(other_user, vec![public_file.as_str().to_string()], "another signed-in user");

  let mut creator = listed_tokens(&app, &batch, Some(&session(&state, &alice))).await;
  creator.sort();
  let mut both = vec![public_file.as_str().to_string(), private_file.as_str().to_string()];
  both.sort();
  assert_eq!(creator, both, "the creator");
}

/// Insert a media file of `owner` with the given visibility into `batch`.
async fn create_batch_media_file(
  pool: &MySqlPool,
  owner: &UserToken,
  batch: &BatchGenerationToken,
  visibility: Visibility,
) -> MediaFileToken {
  let bucket_path = MediaFileBucketPath::generate_new(None, Some("png"));

  MediaFileInsertBuilder::new()
    .creator_user(owner)
    .creator_ip_address("127.0.0.1")
    .creator_set_visibility(visibility)
    .maybe_batch_generation_token(Some(batch))
    .media_file_class(MediaFileClass::Image)
    .media_file_type(MediaFileType::Png)
    .is_user_upload(false)
    .is_intermediate_system_file(false)
    .media_file_origin_category(MediaFileOriginCategory::Inference)
    .media_file_origin_product_category(MediaFileOriginProductCategory::Unknown)
    .mime_type("image/png")
    .file_size_bytes(1_024)
    .checksum_sha2(&"0".repeat(64))
    .public_bucket_directory_hash(&bucket_path)
    .insert_pool(pool)
    .await
    .unwrap()
}

/// The tokens the redux batch endpoint lists, read with `maybe_session` (none = anonymous).
async fn listed_tokens<S, B>(app: &S, batch: &BatchGenerationToken, maybe_session: Option<&str>) -> Vec<String>
where
  S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
  B: MessageBody,
  B::Error: Debug,
{
  let mut request = test::TestRequest::get().uri(&format!("/v1/media_files/batch_gen_redux/{}", batch.as_str()));
  if let Some(session) = maybe_session {
    request = request.insert_header(("session", session));
  }
  let response = test::call_service(app, request.to_request()).await;
  assert_eq!(response.status(), StatusCode::OK);
  let body: Value = test::read_body_json(response).await;
  body["media_files"]
    .as_array()
    .unwrap()
    .iter()
    .map(|file| file["token"].as_str().unwrap().to_string())
    .collect()
}

fn session(state: &ServerState, user: &TestUser) -> String {
  state
    .session_cookie_manager
    .create_cookie(&user.session_token, &user.user_token)
    .unwrap()
    .value()
    .to_string()
}
