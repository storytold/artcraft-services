use actix_cors::Cors;

pub fn add_power_stream(cors: Cors, is_production: bool) -> Cors {
  // TODO: Remove non-SSL "http://" from production in safe rollout
  if is_production {
    cors
        .allowed_origin("https://dash.power.stream")
        .allowed_origin("https://power.stream")
  } else {
    cors
        .allowed_origin("http://dev.dash.power.stream")
        .allowed_origin("http://dev.power.stream")
        .allowed_origin("https://dev.dash.power.stream")
        .allowed_origin("https://dev.power.stream")
  }
}

pub fn add_legacy_storyteller_stream(cors: Cors, is_production: bool) -> Cors {
  // NB: HTTPS only. A credentialed origin served over plain HTTP can be replaced by a network attacker.
  if is_production {
    cors
        // Storyteller.stream (Production)
        .allowed_origin("https://api.storyteller.stream")
        .allowed_origin("https://obs.storyteller.stream")
        .allowed_origin("https://storyteller.stream")
        .allowed_origin("https://ws.storyteller.stream")
        // Storyteller.stream (Staging)
        .allowed_origin("https://staging.obs.storyteller.stream")
        .allowed_origin("https://staging.storyteller.stream")
        // Legacy "create.storyteller.ai" (Production)
        .allowed_origin("https://create.storyteller.ai")
        .allowed_origin("https://obs.storyteller.ai")
        .allowed_origin("https://ws.storyteller.ai")
  } else {
    cors // NB: None!
  }
}

pub fn add_legacy_vocodes(cors: Cors, is_production: bool) -> Cors {
  if is_production {
    cors
        // Vocodes (Production)
        .allowed_origin("https://api.vo.codes")
        .allowed_origin("https://vo.codes")
        .allowed_origin("https://vocodes.com")
  } else {
    cors
        // Vocodes (Development)
        .allowed_origin("http://dev.api.vo.codes")
        .allowed_origin("http://dev.vo.codes")
        .allowed_origin("https://dev.api.vo.codes")
        .allowed_origin("https://dev.vo.codes")
  }
}

pub fn add_legacy_trumped(cors: Cors, is_production: bool) -> Cors {
  if is_production {
    cors
        // Trumped (Production)
        .allowed_origin("https://trumped.com")
  } else {
    cors
        // Trumped (Development)
        .allowed_origin("http://dev.trumped.com")
        .allowed_origin("https://dev.trumped.com")
  }
}


#[cfg(test)]
mod tests {
  use crate::testing::{assert_origin_invalid, assert_origin_ok, isolated_cors};

  use super::{add_legacy_storyteller_stream, add_legacy_trumped, add_legacy_vocodes, add_power_stream};

  // NB: The full per-environment origin lists are pinned in allowlist_regression_tests.

  #[actix_rt::test]
  async fn legacy_storyteller_stream_has_no_development_origins() {
    let cors = isolated_cors(add_legacy_storyteller_stream, false);
    assert_origin_invalid(&cors, "https://storyteller.stream").await;
    assert_origin_invalid(&cors, "https://create.storyteller.ai").await;
  }

  #[actix_rt::test]
  async fn legacy_storyteller_stream_rejects_plaintext_origins_in_production() {
    let cors = isolated_cors(add_legacy_storyteller_stream, true);
    assert_origin_ok(&cors, "https://create.storyteller.ai").await;
    assert_origin_invalid(&cors, "http://api.storyteller.stream").await;
    assert_origin_invalid(&cors, "http://obs.storyteller.stream").await;
    assert_origin_invalid(&cors, "http://storyteller.stream").await;
    assert_origin_invalid(&cors, "http://ws.storyteller.stream").await;
    assert_origin_invalid(&cors, "http://staging.obs.storyteller.stream").await;
    assert_origin_invalid(&cors, "http://staging.storyteller.stream").await;
    assert_origin_invalid(&cors, "http://create.storyteller.ai").await;
    assert_origin_invalid(&cors, "http://obs.storyteller.ai").await;
    assert_origin_invalid(&cors, "http://ws.storyteller.ai").await;
  }

  #[actix_rt::test]
  async fn production_hosts_are_exact() {
    let power_stream = isolated_cors(add_power_stream, true);
    assert_origin_ok(&power_stream, "https://power.stream").await;
    assert_origin_invalid(&power_stream, "http://power.stream").await;
    assert_origin_invalid(&power_stream, "https://power.stream.evil.example").await;

    let vocodes = isolated_cors(add_legacy_vocodes, true);
    assert_origin_ok(&vocodes, "https://vo.codes").await;
    assert_origin_invalid(&vocodes, "https://evil.vo.codes").await;

    let trumped = isolated_cors(add_legacy_trumped, true);
    assert_origin_ok(&trumped, "https://trumped.com").await;
    assert_origin_invalid(&trumped, "https://trumped.com:8443").await;
  }
}
