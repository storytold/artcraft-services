use std::path::Path;

use log::{info, warn};

use errors::{anyhow, AnyhowResult};
use server_environment::ServerEnvironment;

use crate::config_search_directories::{config_search_directories, find_repository_root};

pub struct BootstrapArgs<'a, P: AsRef<Path>> {
  /// The name of the application or service
  pub app_name: &'a str,

  /// Default rust log level.
  /// If the RUST_LOG env var is set, it will take precedence over this setting.
  pub default_logging_override: Option<&'a str>,

  /// Where to look for env conf files.
  /// Relative paths are also searched from an enclosing artcraft or artcraft-services checkout.
  pub config_search_directories: &'a [P],

  /// If true, ignore the root '.env' file.
  /// TODO(bt,2025-01-23): Make this the default behavior, then remove reading from '.env' entirely,
  ///  There are too many apps that share this file.
  pub ignore_legacy_dot_env_file: bool,
}

/// Information about how the application is deployed.
#[derive(Clone)]
pub struct ContainerEnvironment {
  /// Whether the app is running in development or production.
  /// Set with the env var `SERVER_ENVIRONMENT`.
  pub server_environment: ServerEnvironment,

  /// The worker hostname, using OS hostname detection.
  /// If not determined, it will be set to the synthetic
  /// value `{app_name}-unknown`.
  pub hostname: String,

  /// The kubernetes cluster name.
  /// Set with the env var `K8S_CLUSTER_NAME`.
  pub cluster_name: String,

  /// Whether the container is operating on-premises.
  /// False by default. Should be false if the container is
  /// deployed to the cloud or run locally in development.
  /// Should only be true for "data centers" we operate.
  /// Set with the env var `IS_ON_PREM`.
  pub is_on_prem: bool,
}

/// Environment variable for server environment.
const SERVER_ENVIRONMENT : &str = "SERVER_ENVIRONMENT";

pub fn bootstrap<P: AsRef<Path>>(args: BootstrapArgs<'_, P>) -> AnyhowResult<ContainerEnvironment> {
  if args.ignore_legacy_dot_env_file {
    easyenv::init_env_logger(args.default_logging_override);
  } else {
    easyenv::init_all_with_default_logging(args.default_logging_override);
  }

  info!("Bootstrapping application {}", &args.app_name);

  let server_environment = easyenv::get_env_string_optional(SERVER_ENVIRONMENT)
      .map(|environment| ServerEnvironment::from_str(&environment)
          .ok_or(anyhow!("couldn't parse environment: {:?}", &environment)))
      .transpose()?
      .unwrap_or(ServerEnvironment::Development);

  info!("Currently deployed in environment: {:?}",&server_environment);

  // TODO(bt, 2023-04-29): There was an old note in `inference-job` about setting special k8s
  //  variables so we can debug on-prem workers? Something gets conflated maybe and makes it hard
  //  to determine which on-prem worker generates the result? Not a big priority until we have
  //  on-prem workloads again.
  let server_hostname = hostname::get()
      .ok()
      .and_then(|h| h.into_string().ok())
      .unwrap_or_else(|| format!("{}-unknown", args.app_name));

  info!("With hostname: {:?}", &server_hostname);

  let cluster_name = easyenv::get_env_string_optional("K8S_CLUSTER_NAME")
      .unwrap_or("unknown-cluster".to_string());

  info!("With cluster name: {:?}", &cluster_name);

  // TODO(bt, 2023-04-29): These were old variables in `inference-job` that probably never got used,
  //  but adding these is a good idea for tracking which jobs are assigned where.
  // NB: These are non-standard env vars we're injecting ourselves.
  // let k8s_node_name = easyenv::get_env_string_optional("K8S_NODE_NAME");
  // let k8s_pod_name = easyenv::get_env_string_optional("K8S_POD_NAME");

  // NB: It'll be worthwhile to see how much compute is happening at our local on-premises cluster
  // Only our local workers will set this to true.
  let is_on_prem = easyenv::get_env_bool_or_default("IS_ON_PREM", false);

  info!("Is on premises? {}", is_on_prem);

  // TODO(bt, 2023-04-29): Determine if we want the "debug worker" flag, or if we'd rather
  //  have a "passive" flag + job routing system.
  // Debug workers only process special debug requests. They're silent otherwise.
  // Non-debug workers ignore debug requests. This is so we can deploy special code
  // to debug nodes (typically just one, perhaps even ephemerally).
  //let is_debug_worker = easyenv::get_env_bool_or_default("IS_DEBUG_WORKER", false);
  //info!("Is debug worker? {}", is_debug_worker);

  load_env_config_files(server_environment, &args)?;

  Ok(ContainerEnvironment {
    server_environment,
    hostname: server_hostname,
    cluster_name,
    is_on_prem,
  })
}

fn load_env_config_files<P: AsRef<Path>>(server_environment: ServerEnvironment, args: &BootstrapArgs<'_, P>) -> AnyhowResult<()> {
  let maybe_current_directory = std::env::current_dir().ok();
  let maybe_repository_root = maybe_current_directory.as_deref().and_then(find_repository_root);
  let search_directories = config_search_directories(args.config_search_directories, maybe_repository_root);
  let secrets_filename = format!("{}.development-secrets.env", &args.app_name);

  let env_config_file_names = match server_environment {
    ServerEnvironment::Development => vec![
      format!("{}.common.env", &args.app_name),
      format!("{}.development.env", &args.app_name),
      secrets_filename.clone(), // NB: .gitignore these files
    ],
    ServerEnvironment::Production => vec![
      format!("{}.common.env", &args.app_name),
      format!("{}.production.env", &args.app_name),
    ],
  };

  for env_config_file in env_config_file_names.into_iter() {
    info!("Loading environment variable config file: {}", &env_config_file);

    let mut was_read = easyenv::maybe_read_env_config_from_filename_and_paths(
      &env_config_file,
      &search_directories)?;

    // Development backups fill missing variables, even when a local config file exists.
    // dotenv preserves values already set in the environment or earlier config files.
    if server_environment == ServerEnvironment::Development && env_config_file == secrets_filename {
      if let Some(root) = maybe_repository_root {
        was_read |= easyenv::maybe_read_env_config_from_filename_and_paths(
          &env_config_file,
          &[root.join("secrets/artcraft-backup")])?;
      }
    }

    if was_read {
      info!("Environment config file `{}` was read.", &env_config_file);
    } else {
      warn!("Could not read environment config file: `{}`", &env_config_file);
    }
  }

  Ok(())
}
