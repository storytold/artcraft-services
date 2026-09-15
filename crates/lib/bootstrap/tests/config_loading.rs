use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use bootstrap::bootstrap::{bootstrap, BootstrapArgs};
use tempfile::TempDir;

const CONFIG_DIRECTORY: &str = "crates/service/job/test_job/config";
const BACKUP_DIRECTORY: &str = "secrets/artcraft-backup";
const CHILD_PROCESS: &str = "BOOTSTRAP_CONFIG_TEST_CHILD";
const VALUE: &str = "BOOTSTRAP_CONFIG_TEST_VALUE";
const MISSING_VALUE: &str = "BOOTSTRAP_CONFIG_TEST_MISSING_VALUE";
const COMMON_VALUE: &str = "BOOTSTRAP_CONFIG_TEST_COMMON_VALUE";

mod development {
  use super::*;

  #[test]
  fn loads_checkout_configs_and_backup_secrets_from_roots_and_subdirectories() {
    for name in ["artcraft", "artcraft-services"] {
      let (_temporary, root) = checkout(name);
      write_config(&root, CONFIG_DIRECTORY, "common", &format!("{COMMON_VALUE}=common\n"));
      write_config(&root, BACKUP_DIRECTORY, "development-secrets", &format!("{VALUE}=backup\n"));

      // The other checkout must not supply this process's secrets.
      let sibling_name = if name == "artcraft" { "artcraft-services" } else { "artcraft" };
      let sibling = root.parent().unwrap().join(sibling_name);
      write_config(&sibling, BACKUP_DIRECTORY, "development-secrets", &format!("{VALUE}=wrong-checkout\n"));

      // Nested directories can themselves be named artcraft without being the repo root.
      let nested = root.join("crates/api_clients/artcraft/test_client");
      fs::create_dir_all(&nested).unwrap();
      for directory in [&root, &root.join(CONFIG_DIRECTORY), &nested] {
        run_bootstrap(directory, "development", "backup", "common", false);
      }
    }
  }

  #[test]
  fn environment_and_local_configs_take_precedence_but_backup_fills_missing_values() {
    let (_temporary, root) = checkout("artcraft-services");
    write_config(&root, CONFIG_DIRECTORY, "development-secrets", &format!("{VALUE}=local\n"));
    write_config(&root, BACKUP_DIRECTORY, "development-secrets",
      &format!("{VALUE}=backup\n{MISSING_VALUE}=backup-only\n"));

    run_bootstrap(&root, "development", "local", "", true);
    run_bootstrap(&root, "development", "environment", "", true);
  }
}

mod deployment {
  use super::*;

  #[test]
  fn production_does_not_read_development_secrets() {
    let (_temporary, root) = checkout("artcraft-services");
    write_config(&root, CONFIG_DIRECTORY, "production", &format!("{VALUE}=production\n"));
    write_config(&root, BACKUP_DIRECTORY, "development-secrets", &format!("{MISSING_VALUE}=backup-only\n"));

    run_bootstrap(&root.join(CONFIG_DIRECTORY), "production", "production", "", false);
  }

  #[test]
  fn configured_paths_still_work_outside_named_checkouts() {
    let temporary = TempDir::new().unwrap();
    let root = temporary.path().join("deployment");
    write_config(&root, "config", "development-secrets", &format!("{VALUE}=deployment\n"));
    write_config(&root, BACKUP_DIRECTORY, "development-secrets", &format!("{MISSING_VALUE}=backup-only\n"));

    run_bootstrap(&root, "development", "deployment", "", false);
  }
}

// Run bootstrap in a subprocess so environment and logger changes cannot affect other tests.
#[test]
fn bootstrap_child_process() {
  if env::var_os(CHILD_PROCESS).is_none() {
    return;
  }

  bootstrap(BootstrapArgs {
    app_name: "bootstrap-test",
    default_logging_override: None,
    config_search_directories: &[".", "./config", CONFIG_DIRECTORY],
    ignore_legacy_dot_env_file: true,
  }).unwrap();

  assert_eq!(env::var(VALUE).unwrap(), env::var("EXPECTED_VALUE").unwrap());
  assert_eq!(env::var(COMMON_VALUE).unwrap_or_default(), env::var("EXPECTED_COMMON_VALUE").unwrap());
  if env::var("EXPECT_BACKUP_VALUE").unwrap() == "true" {
    assert_eq!(env::var(MISSING_VALUE).unwrap(), "backup-only");
  } else {
    assert!(env::var_os(MISSING_VALUE).is_none());
  }
}

fn run_bootstrap(directory: &Path, environment: &str, expected: &str, expected_common: &str, expect_backup: bool) {
  let mut command = Command::new(env::current_exe().unwrap());
  command.args(["--exact", "bootstrap_child_process", "--nocapture"])
      .current_dir(directory)
      .env_clear()
      .env(CHILD_PROCESS, "1")
      .env("RUST_LOG", "error")
      .env("SERVER_ENVIRONMENT", environment)
      .env("EXPECTED_VALUE", expected)
      .env("EXPECTED_COMMON_VALUE", expected_common)
      .env("EXPECT_BACKUP_VALUE", expect_backup.to_string());

  if expected == "environment" {
    command.env(VALUE, "environment");
  }

  let output = command.output().unwrap();
  assert!(output.status.success(), "bootstrap failed in {directory:?}:\n{}\n{}",
    String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

fn checkout(name: &str) -> (TempDir, PathBuf) {
  let temporary = TempDir::new().unwrap();
  let root = temporary.path().join(name);
  fs::create_dir_all(root.join(CONFIG_DIRECTORY)).unwrap();
  fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
  (temporary, root)
}

fn write_config(root: &Path, directory: &str, suffix: &str, contents: &str) {
  let directory = root.join(directory);
  fs::create_dir_all(&directory).unwrap();
  fs::write(directory.join(format!("bootstrap-test.{suffix}.env")), contents).unwrap();
}
