//! Regression tests pinning every exact origin the CORS config allowed before
//! first-party domains moved to the `domains` crate (commit a1147a98d2).
//!
//! The origins are string literals on purpose: they check the `domains`
//! constants against the real values instead of against themselves. Each row
//! is checked against its module in isolation (so a module can't pass because
//! another module allows the same origin) and against the full config.
//!
//! The table was generated from the `allowed_origin("...")` calls at
//! a1147a98d2. When you add or remove an origin, update this table too.

use actix_cors::Cors;
use server_environment::ServerEnvironment;

use crate::configs::artcraft_2d::add_artcraft_2d;
use crate::configs::artcraft_3d::add_artcraft_3d;
use crate::configs::artcraft_admin_dashboard::add_artcraft_admin_dashboard;
use crate::configs::artcraft_desktop::add_artcraft_desktop;
use crate::configs::artcraft_webapp::add_artcraft_webapp;
use crate::configs::artcraft_website::add_artcraft_website;
use crate::configs::fakeyou::add_fakeyou;
use crate::configs::fakeyou::add_fakeyou_dev_proxy;
use crate::configs::legacy::add_legacy_storyteller_stream;
use crate::configs::legacy::add_legacy_trumped;
use crate::configs::legacy::add_legacy_vocodes;
use crate::configs::legacy::add_power_stream;
use crate::configs::realseedance::add_realseedance;
use crate::configs::storyteller::add_storyteller;
use crate::configs::storyteller::add_storyteller_dev_proxy;
use crate::configs::storyteller_board::add_storyteller_board;
use crate::configs::storyteller_render::add_storyteller_render;
use crate::configs::storyteller_studio::add_storyteller_studio;
use crate::configs::tauri::add_tauri;
use crate::cors::build_cors_config;
use crate::testing::{assert_exact_https_origin, is_origin_allowed, isolated_cors};

use Env::{Both, DevelopmentOnly, ProductionOnly};

type AddOrigins = fn(Cors, bool) -> Cors;

/// Which environments an origin is configured for.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Env {
  Both,
  ProductionOnly,
  DevelopmentOnly,
}

const ALLOWLIST: &[(AddOrigins, Env, &str)] = &[
  // add_artcraft_2d
  (add_artcraft_2d,               Both,            "https://2d.storyteller.ai"),
  (add_artcraft_2d,               Both,            "https://2d.getartcraft.com"),
  (add_artcraft_2d,               Both,            "http://localhost:5741"),
  (add_artcraft_2d,               Both,            "http://localhost:5742"),
  (add_artcraft_2d,               Both,            "http://localhost:5743"),

  // add_artcraft_3d
  (add_artcraft_3d,               Both,            "https://3d.storyteller.ai"),
  (add_artcraft_3d,               Both,            "https://3d.getartcraft.com"),
  (add_artcraft_3d,               Both,            "http://localhost:5173"),
  (add_artcraft_3d,               Both,            "http://localhost:5174"),
  (add_artcraft_3d,               Both,            "http://localhost:5175"),
  (add_artcraft_3d,               Both,            "https://macaroni-1.tailce84f.ts.net"),
  (add_artcraft_3d,               Both,            "https://halide.tailce84f.ts.net"),
  (add_artcraft_3d,               Both,            "https://brandons-macbook-pro.taild62114.ts.net"),

  // add_artcraft_admin_dashboard
  (add_artcraft_admin_dashboard,  Both,            "https://artcraft-dashboard.netlify.app"),

  // add_artcraft_desktop
  (add_artcraft_desktop,          Both,            "https://desktop.getartcraft.com"),

  // add_artcraft_webapp
  (add_artcraft_webapp,           Both,            "https://app.getartcraft.com"),
  (add_artcraft_webapp,           Both,            "https://app-new.getartcraft.com"),

  // add_artcraft_website
  (add_artcraft_website,          Both,            "https://getartcraft.com"),
  (add_artcraft_website,          Both,            "https://www.getartcraft.com"),
  (add_artcraft_website,          Both,            "https://artcraft.ai"),
  (add_artcraft_website,          Both,            "https://www.artcraft.ai"),
  (add_artcraft_website,          Both,            "http://localhost:4200"),
  (add_artcraft_website,          Both,            "http://localhost:4201"),
  (add_artcraft_website,          Both,            "http://localhost:4202"),

  // add_fakeyou
  (add_fakeyou,                   Both,            "https://engine.fakeyou.com"),
  (add_fakeyou,                   ProductionOnly,  "https://api.fakeyou.com"),
  (add_fakeyou,                   ProductionOnly,  "https://fakeyou.com"),
  (add_fakeyou,                   ProductionOnly,  "https://staging.fakeyou.com"),
  (add_fakeyou,                   DevelopmentOnly, "http://dev.fakeyou.com"),
  (add_fakeyou,                   DevelopmentOnly, "http://dev.fakeyou.com:7000"),
  (add_fakeyou,                   DevelopmentOnly, "http://dev.fakeyou.com:7001"),
  (add_fakeyou,                   DevelopmentOnly, "https://dev.fakeyou.com"),
  (add_fakeyou,                   DevelopmentOnly, "https://dev.fakeyou.com:7000"),
  (add_fakeyou,                   DevelopmentOnly, "https://dev.fakeyou.com:7001"),

  // add_fakeyou_dev_proxy
  (add_fakeyou_dev_proxy,         Both,            "http://devproxy.fakeyou.com"),
  (add_fakeyou_dev_proxy,         Both,            "http://devproxy.fakeyou.com:5173"),
  (add_fakeyou_dev_proxy,         Both,            "http://devproxy.fakeyou.com:7000"),
  (add_fakeyou_dev_proxy,         Both,            "http://devproxy.fakeyou.com:7001"),
  (add_fakeyou_dev_proxy,         Both,            "http://devproxy.fakeyou.com:7002"),
  (add_fakeyou_dev_proxy,         Both,            "https://devproxy.fakeyou.com"),
  (add_fakeyou_dev_proxy,         Both,            "https://devproxy.fakeyou.com:5173"),
  (add_fakeyou_dev_proxy,         Both,            "https://devproxy.fakeyou.com:7000"),
  (add_fakeyou_dev_proxy,         Both,            "https://devproxy.fakeyou.com:7001"),
  (add_fakeyou_dev_proxy,         Both,            "https://devproxy.fakeyou.com:7002"),

  // add_legacy_storyteller_stream
  (add_legacy_storyteller_stream, ProductionOnly,  "http://api.storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "http://obs.storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "http://storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "http://ws.storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "https://api.storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "https://obs.storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "https://storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "https://ws.storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "http://staging.obs.storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "http://staging.storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "https://staging.obs.storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "https://staging.storyteller.stream"),
  (add_legacy_storyteller_stream, ProductionOnly,  "http://create.storyteller.ai"),
  (add_legacy_storyteller_stream, ProductionOnly,  "http://obs.storyteller.ai"),
  (add_legacy_storyteller_stream, ProductionOnly,  "http://ws.storyteller.ai"),
  (add_legacy_storyteller_stream, ProductionOnly,  "https://create.storyteller.ai"),
  (add_legacy_storyteller_stream, ProductionOnly,  "https://obs.storyteller.ai"),
  (add_legacy_storyteller_stream, ProductionOnly,  "https://ws.storyteller.ai"),

  // add_legacy_trumped
  (add_legacy_trumped,            ProductionOnly,  "https://trumped.com"),
  (add_legacy_trumped,            DevelopmentOnly, "http://dev.trumped.com"),
  (add_legacy_trumped,            DevelopmentOnly, "https://dev.trumped.com"),

  // add_legacy_vocodes
  (add_legacy_vocodes,            ProductionOnly,  "https://api.vo.codes"),
  (add_legacy_vocodes,            ProductionOnly,  "https://vo.codes"),
  (add_legacy_vocodes,            ProductionOnly,  "https://vocodes.com"),
  (add_legacy_vocodes,            DevelopmentOnly, "http://dev.api.vo.codes"),
  (add_legacy_vocodes,            DevelopmentOnly, "http://dev.vo.codes"),
  (add_legacy_vocodes,            DevelopmentOnly, "https://dev.api.vo.codes"),
  (add_legacy_vocodes,            DevelopmentOnly, "https://dev.vo.codes"),

  // add_power_stream
  (add_power_stream,              ProductionOnly,  "https://dash.power.stream"),
  (add_power_stream,              ProductionOnly,  "https://power.stream"),
  (add_power_stream,              DevelopmentOnly, "http://dev.dash.power.stream"),
  (add_power_stream,              DevelopmentOnly, "http://dev.power.stream"),
  (add_power_stream,              DevelopmentOnly, "https://dev.dash.power.stream"),
  (add_power_stream,              DevelopmentOnly, "https://dev.power.stream"),

  // add_realseedance
  (add_realseedance,              Both,            "https://realseedance.com"),
  (add_realseedance,              Both,            "https://www.realseedance.com"),

  // add_storyteller
  (add_storyteller,               ProductionOnly,  "https://engine.storyteller.ai"),
  (add_storyteller,               ProductionOnly,  "https://studio.storyteller.ai"),
  (add_storyteller,               ProductionOnly,  "https://api.storyteller.ai"),
  (add_storyteller,               ProductionOnly,  "https://storyteller.ai"),
  (add_storyteller,               ProductionOnly,  "https://staging.storyteller.ai"),
  (add_storyteller,               DevelopmentOnly, "http://dev.storyteller.ai"),
  (add_storyteller,               DevelopmentOnly, "http://dev.storyteller.ai:5173"),
  (add_storyteller,               DevelopmentOnly, "http://dev.storyteller.ai:7000"),
  (add_storyteller,               DevelopmentOnly, "http://dev.storyteller.ai:7001"),
  (add_storyteller,               DevelopmentOnly, "http://dev.storyteller.ai:7002"),
  (add_storyteller,               DevelopmentOnly, "https://dev.storyteller.ai"),
  (add_storyteller,               DevelopmentOnly, "https://dev.storyteller.ai:5173"),
  (add_storyteller,               DevelopmentOnly, "https://dev.storyteller.ai:7000"),
  (add_storyteller,               DevelopmentOnly, "https://dev.storyteller.ai:7001"),
  (add_storyteller,               DevelopmentOnly, "https://dev.storyteller.ai:7002"),

  // add_storyteller_dev_proxy
  (add_storyteller_dev_proxy,     Both,            "http://devproxy.storyteller.ai"),
  (add_storyteller_dev_proxy,     Both,            "http://devproxy.storyteller.ai:5173"),
  (add_storyteller_dev_proxy,     Both,            "http://devproxy.storyteller.ai:7000"),
  (add_storyteller_dev_proxy,     Both,            "http://devproxy.storyteller.ai:7001"),
  (add_storyteller_dev_proxy,     Both,            "http://devproxy.storyteller.ai:7002"),
  (add_storyteller_dev_proxy,     Both,            "https://devproxy.storyteller.ai"),
  (add_storyteller_dev_proxy,     Both,            "https://devproxy.storyteller.ai:5173"),
  (add_storyteller_dev_proxy,     Both,            "https://devproxy.storyteller.ai:7000"),
  (add_storyteller_dev_proxy,     Both,            "https://devproxy.storyteller.ai:7001"),
  (add_storyteller_dev_proxy,     Both,            "https://devproxy.storyteller.ai:7002"),

  // add_storyteller_board
  (add_storyteller_board,         Both,            "https://memeboard.ai"),
  (add_storyteller_board,         Both,            "https://dingboard.ai"),
  (add_storyteller_board,         Both,            "https://board.storyteller.ai"),
  (add_storyteller_board,         Both,            "http://localhost:5173/"),

  // add_storyteller_render
  (add_storyteller_render,        Both,            "https://render.storyteller.ai"),

  // add_storyteller_studio
  (add_storyteller_studio,        Both,            "https://studio.storyteller.ai"),
  (add_storyteller_studio,        Both,            "https://studio-staging.studio.storyteller.ai"),
  (add_storyteller_studio,        Both,            "https://studio-testing.studio.storyteller.ai"),
  (add_storyteller_studio,        Both,            "http://localhost:5173"),
  (add_storyteller_studio,        Both,            "https://animate.storyteller.ai"),

  // add_tauri
  (add_tauri,                     Both,            "http://tauri.localhost"),
  (add_tauri,                     Both,            "tauri://localhost"),
];

/// The origins whose `allowed_origin(...)` literal became a `domains` constant.
const MIGRATED_ORIGINS: &[&str] = &[
  "https://getartcraft.com",
  "https://www.getartcraft.com",
  "https://app.getartcraft.com",
  "https://app-new.getartcraft.com",
  "https://desktop.getartcraft.com",
  "https://2d.getartcraft.com",
  "https://3d.getartcraft.com",
  "https://artcraft.ai",
  "https://www.artcraft.ai",
  "https://fakeyou.com",
  "https://api.fakeyou.com",
  "https://engine.fakeyou.com",
  "https://staging.fakeyou.com",
  "https://storyteller.ai",
  "https://api.storyteller.ai",
  "https://engine.storyteller.ai",
  "https://staging.storyteller.ai",
  "https://studio.storyteller.ai",
  "https://studio-staging.studio.storyteller.ai",
  "https://studio-testing.studio.storyteller.ai",
  "https://animate.storyteller.ai",
  "https://board.storyteller.ai",
  "https://render.storyteller.ai",
  "https://2d.storyteller.ai",
  "https://3d.storyteller.ai",
];

mod isolated_module_tests {
  use super::*;

  #[actix_rt::test]
  async fn each_module_allows_its_origins_in_production() {
    let failures = allowed_failures(true, |env| env != DevelopmentOnly, false).await;
    assert!(failures.is_empty(), "production origins not allowed by their module: {:#?}", failures);
  }

  #[actix_rt::test]
  async fn each_module_allows_its_origins_in_development() {
    let failures = allowed_failures(false, |env| env != ProductionOnly, false).await;
    assert!(failures.is_empty(), "development origins not allowed by their module: {:#?}", failures);
  }

  #[actix_rt::test]
  async fn development_only_origins_are_rejected_by_their_module_in_production() {
    let failures = allowed_failures(true, |env| env == DevelopmentOnly, true).await;
    assert!(failures.is_empty(), "development-only origins allowed in production: {:#?}", failures);
  }

  #[actix_rt::test]
  async fn production_only_origins_are_rejected_by_their_module_in_development() {
    let failures = allowed_failures(false, |env| env == ProductionOnly, true).await;
    assert!(failures.is_empty(), "production-only origins allowed in development: {:#?}", failures);
  }

  /// Checks each matching row against its own module's isolated config and
  /// returns the origins that were allowed (`expect_rejected`) or not.
  async fn allowed_failures(
    is_production: bool,
    select: impl Fn(Env) -> bool,
    expect_rejected: bool,
  ) -> Vec<&'static str> {
    let mut failures = Vec::new();
    for &(add_origins, env, origin) in ALLOWLIST {
      if !select(env) {
        continue;
      }
      let cors = isolated_cors(add_origins, is_production);
      if is_origin_allowed(&cors, origin).await == expect_rejected {
        failures.push(origin);
      }
    }
    failures
  }
}

mod full_config_tests {
  use super::*;

  #[actix_rt::test]
  async fn production_config_allows_every_production_origin() {
    let cors = build_cors_config(ServerEnvironment::Production);
    let failures = full_config_failures(&cors, |env| env != DevelopmentOnly, false).await;
    assert!(failures.is_empty(), "production origins not allowed: {:#?}", failures);
  }

  #[actix_rt::test]
  async fn development_config_allows_every_development_origin() {
    let cors = build_cors_config(ServerEnvironment::Development);
    let failures = full_config_failures(&cors, |env| env != ProductionOnly, false).await;
    assert!(failures.is_empty(), "development origins not allowed: {:#?}", failures);
  }

  #[actix_rt::test]
  async fn production_config_rejects_development_only_origins() {
    let cors = build_cors_config(ServerEnvironment::Production);
    let failures = full_config_failures(&cors, |env| env == DevelopmentOnly, true).await;
    assert!(failures.is_empty(), "development-only origins allowed in production: {:#?}", failures);
  }

  /// Every origin that moved to a `domains` constant, checked for lookalikes
  /// against the full production config (not just the owning module).
  #[actix_rt::test]
  async fn production_config_rejects_lookalikes_of_migrated_origins() {
    let cors = build_cors_config(ServerEnvironment::Production);
    for origin in MIGRATED_ORIGINS {
      assert_exact_https_origin(&cors, origin).await;
    }
  }

  async fn full_config_failures(
    cors: &Cors,
    select: impl Fn(Env) -> bool,
    expect_rejected: bool,
  ) -> Vec<&'static str> {
    let mut failures = Vec::new();
    for &(_, env, origin) in ALLOWLIST {
      if select(env) && is_origin_allowed(cors, origin).await == expect_rejected {
        failures.push(origin);
      }
    }
    failures
  }
}
