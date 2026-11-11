//! Investigation metrics for GitHub issue #1069 (Sora latency by plan).
//!
//! Emits Datadog counters/distributions only — no behavior change.
//! See `_docs/sora_latency_investigation_1069.md`.

use chrono::{DateTime, Utc};
use enums::common::artcraft_subscription_slug::ArtcraftSubscriptionSlug;
use enums::common::generation::common_model_type::CommonModelType;
use enums::common::generation::common_resolution::CommonResolution;
use enums::common::generation::common_video_model::CommonVideoModel;
use enums::common::payments_namespace::PaymentsNamespace;
use log::warn;
use metrics::collector::MetricsCollector;
use mysql_queries::queries::users::user_subscriptions::find_subscription_for_owner_user::{
  find_subscription_for_owner_user_using_connection,
  find_subscription_for_owner_user_using_transaction,
};
use sqlx::pool::PoolConnection;
use sqlx::MySql;
use tokens::tokens::generic_inference_jobs::InferenceJobToken;
use tokens::tokens::users::UserToken;

pub const METRIC_ENQUEUE: &str = "artcraft.sora.enqueue";
pub const METRIC_JOB_DURATION_MS: &str = "artcraft.sora.job_duration_ms";

/// Bucketed plan tag for Datadog (low cardinality).
pub fn plan_tag_from_product_slug(maybe_slug: Option<&str>) -> &'static str {
  match maybe_slug {
    None => "free",
    Some(slug) => match ArtcraftSubscriptionSlug::from_str(slug) {
      Ok(ArtcraftSubscriptionSlug::ArtcraftBasic) => "artcraft_basic",
      Ok(ArtcraftSubscriptionSlug::ArtcraftPro) => "artcraft_pro",
      Ok(ArtcraftSubscriptionSlug::ArtcraftMax) => "artcraft_max",
      Err(_) => "unknown",
    },
  }
}

pub fn model_tag_from_video_model(model: CommonVideoModel) -> Option<&'static str> {
  match model {
    CommonVideoModel::Sora2 => Some("sora_2"),
    CommonVideoModel::Sora2Pro => Some("sora_2_pro"),
    _ => None,
  }
}

pub fn model_tag_from_model_type_str(maybe_model_type: Option<&str>) -> Option<&'static str> {
  match maybe_model_type {
    Some(s) => match CommonModelType::from_str(s) {
      Ok(CommonModelType::Sora2) => Some("sora_2"),
      Ok(CommonModelType::Sora2Pro) => Some("sora_2_pro"),
      _ => None,
    },
    None => None,
  }
}

pub fn resolution_tag(maybe_resolution: Option<CommonResolution>) -> &'static str {
  match maybe_resolution {
    Some(CommonResolution::SevenTwentyP) => "720p",
    Some(CommonResolution::TenEightyP) => "1080p",
    Some(_) | None => "other",
  }
}

pub fn duration_tag(maybe_duration_seconds: Option<u16>) -> &'static str {
  match maybe_duration_seconds {
    Some(4) => "4",
    Some(8) => "8",
    Some(12) => "12",
    _ => "other",
  }
}

fn build_tags(
  model: &str,
  plan: &str,
  resolution: &str,
  duration_s: &str,
  maybe_outcome: Option<&str>,
) -> Vec<String> {
  let mut tags = vec![
    format!("model:{}", model),
    format!("plan:{}", plan),
    format!("resolution:{}", resolution),
    format!("duration_s:{}", duration_s),
  ];
  if let Some(outcome) = maybe_outcome {
    tags.push(format!("outcome:{}", outcome));
  }
  tags
}

pub fn record_sora_enqueue(
  collector: &MetricsCollector,
  model: &str,
  plan: &str,
  resolution: &str,
  duration_s: &str,
) {
  collector.increment(
    METRIC_ENQUEUE,
    build_tags(model, plan, resolution, duration_s, None),
  );
}

pub fn record_sora_job_duration_ms(
  collector: &MetricsCollector,
  model: &str,
  plan: &str,
  resolution: &str,
  duration_s: &str,
  outcome: &str,
  duration_ms: f64,
) {
  collector.observe(
    METRIC_JOB_DURATION_MS,
    duration_ms,
    build_tags(model, plan, resolution, duration_s, Some(outcome)),
  );
}

fn plan_tag_from_lookup_result(
  user_token: &UserToken,
  result: Result<
    Option<mysql_queries::queries::users::user_subscriptions::find_subscription_for_owner_user::UserSubscription>,
    mysql_queries::errors::select_optional_record_error::SelectOptionalRecordError,
  >,
) -> &'static str {
  match result {
    Ok(Some(sub)) => plan_tag_from_product_slug(Some(sub.subscription_product_slug.as_str())),
    Ok(None) => "free",
    Err(err) => {
      warn!(
        "Sora latency metrics: failed to look up subscription for {}: {:?}",
        user_token.as_str(),
        err
      );
      "unknown"
    }
  }
}

/// Resolve Artcraft plan slug for metrics (`free` when no active subscription).
pub async fn resolve_plan_tag_for_user(
  user_token: &UserToken,
  mysql_connection: &mut PoolConnection<MySql>,
) -> &'static str {
  plan_tag_from_lookup_result(
    user_token,
    find_subscription_for_owner_user_using_connection(
      user_token,
      PaymentsNamespace::Artcraft,
      mysql_connection,
    )
    .await,
  )
}

/// Same as [`resolve_plan_tag_for_user`] but on an open transaction (enqueue path).
pub async fn resolve_plan_tag_for_user_in_transaction(
  user_token: &UserToken,
  transaction: &mut sqlx::Transaction<'_, MySql>,
) -> &'static str {
  plan_tag_from_lookup_result(
    user_token,
    find_subscription_for_owner_user_using_transaction(
      user_token,
      PaymentsNamespace::Artcraft,
      transaction,
    )
    .await,
  )
}

/// Runtime (non-macro) lookup so we do not need a new SQLx prepare entry.
pub async fn fetch_maybe_model_type_for_job(
  job_token: &InferenceJobToken,
  mysql_connection: &mut PoolConnection<MySql>,
) -> Option<String> {
  let result: Result<Option<String>, sqlx::Error> = sqlx::query_scalar(
    "SELECT maybe_model_type FROM generic_inference_jobs WHERE token = ?",
  )
  .bind(job_token.as_str())
  .fetch_optional(&mut **mysql_connection)
  .await;

  match result {
    Ok(value) => value,
    Err(err) => {
      warn!(
        "Sora latency metrics: failed to load maybe_model_type for {}: {:?}",
        job_token.as_str(),
        err
      );
      None
    }
  }
}

pub fn job_duration_ms_since(created_at: DateTime<Utc>) -> f64 {
  let now = Utc::now();
  let millis = (now - created_at).num_milliseconds();
  if millis < 0 {
    0.0
  } else {
    millis as f64
  }
}

/// Best-effort completion metric for Fal webhooks on Sora jobs.
pub async fn maybe_record_sora_fal_job_completion(
  collector: &MetricsCollector,
  job_token: &InferenceJobToken,
  maybe_creator_user_token: Option<&UserToken>,
  created_at: DateTime<Utc>,
  outcome: &str,
  mysql_connection: &mut PoolConnection<MySql>,
) {
  let maybe_model_type = fetch_maybe_model_type_for_job(job_token, mysql_connection).await;
  let Some(model) = model_tag_from_model_type_str(maybe_model_type.as_deref()) else {
    return;
  };

  let plan = match maybe_creator_user_token {
    Some(user_token) => resolve_plan_tag_for_user(user_token, mysql_connection).await,
    None => "unknown",
  };

  record_sora_job_duration_ms(
    collector,
    model,
    plan,
    "other",
    "other",
    outcome,
    job_duration_ms_since(created_at),
  );
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn plan_tag_maps_known_slugs_and_defaults() {
    assert_eq!(plan_tag_from_product_slug(None), "free");
    assert_eq!(plan_tag_from_product_slug(Some("artcraft_basic")), "artcraft_basic");
    assert_eq!(plan_tag_from_product_slug(Some("artcraft_pro")), "artcraft_pro");
    assert_eq!(plan_tag_from_product_slug(Some("artcraft_max")), "artcraft_max");
    assert_eq!(plan_tag_from_product_slug(Some("fakeyou_plus")), "unknown");
  }

  #[test]
  fn model_tag_only_for_sora() {
    assert_eq!(model_tag_from_video_model(CommonVideoModel::Sora2), Some("sora_2"));
    assert_eq!(model_tag_from_video_model(CommonVideoModel::Sora2Pro), Some("sora_2_pro"));
    assert_eq!(model_tag_from_video_model(CommonVideoModel::Kling2p6Pro), None);
    assert_eq!(model_tag_from_model_type_str(Some("sora_2")), Some("sora_2"));
    assert_eq!(model_tag_from_model_type_str(Some("sora_2_pro")), Some("sora_2_pro"));
    assert_eq!(model_tag_from_model_type_str(Some("kling_2p6_pro")), None);
    assert_eq!(model_tag_from_model_type_str(None), None);
  }

  #[test]
  fn resolution_and_duration_buckets() {
    assert_eq!(resolution_tag(Some(CommonResolution::SevenTwentyP)), "720p");
    assert_eq!(resolution_tag(Some(CommonResolution::TenEightyP)), "1080p");
    assert_eq!(resolution_tag(Some(CommonResolution::FourEightyP)), "other");
    assert_eq!(resolution_tag(None), "other");
    assert_eq!(duration_tag(Some(4)), "4");
    assert_eq!(duration_tag(Some(8)), "8");
    assert_eq!(duration_tag(Some(12)), "12");
    assert_eq!(duration_tag(Some(5)), "other");
    assert_eq!(duration_tag(None), "other");
  }

  #[test]
  fn job_duration_non_negative() {
    let past = Utc::now() - chrono::Duration::seconds(3);
    assert!(job_duration_ms_since(past) >= 0.0);
  }
}
