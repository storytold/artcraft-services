
/// Redis cache keys likely need to be shared across microservice boundaries, hence being defined here.
/// Note that the caching system might mangle these keys further.
pub struct RedisCacheKeys;

impl RedisCacheKeys {

  pub fn get_tts_model_endpoint(model_token: &str) -> String {
    format!("get_tts_model:{}", model_token)
  }

  /// The cached record depends on whether the viewer may see deleted models, so that is part of the key.
  pub fn get_tts_model_for_info_migration_endpoint(model_token: &str, can_see_deleted: bool) -> String {
    format!("get_tts_inf1:{}:{}", model_token, if can_see_deleted { "del" } else { "pub" })
  }

  pub fn get_tts_model_for_inference_migration_endpoint(model_token: &str) -> String {
    format!("get_tts_inf2:{}", model_token)
  }

  pub fn session_record_light(session_token: &str) -> String {
    format!("ses_light:{}", session_token)
  }

  pub fn session_record_user(session_token: &str) -> String {
    format!("ses_user:{}", session_token)
  }
}
