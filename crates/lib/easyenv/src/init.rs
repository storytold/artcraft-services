use std::path::{Path, PathBuf};

use log::error;

use crate::error::InitError;
use crate::init_env_logger;

/// Initialize dotenv (with `.env` file) and the env logger using the `RUST_LOG` variable setting.
pub fn init_all_with_default_logging(default_if_absent: Option<&str>) {
  init_dotenv();
  init_env_logger(default_if_absent)
}

/// Read env configs from a filename.
pub fn from_filename<P: AsRef<Path>>(filename: P) -> Result<(), InitError> {
  let _path = dotenvy::from_filename(&filename)
      .map_err(|err| {
        error!("Could not read env config from path {:?} : {:?}", filename.as_ref(), err);
        InitError::DotEnvError
      })?;
  Ok(())
}

/// Try loading in an environment file across many search paths (first found wins).
/// Returns an error if no file could be read or if in attempting to read a file there was an error.
pub fn read_env_config_from_filename_and_paths<P: AsRef<Path>, Q: AsRef<Path>>(filename: P, paths: &[Q]) -> Result<(), InitError> {
  if do_read_env_config_from_filename_and_paths(&filename, paths)? {
    Ok(())
  } else {
    error!("No env file existed for filename {:?} in the search paths {:?}",
      filename.as_ref(),
      paths.iter().map(|p| p.as_ref().to_path_buf()).collect::<Vec<PathBuf>>());
    Err(InitError::NoConfigFileFoundError)
  }
}

/// Try loading in an environment file across many search paths (first found wins).
/// Returns okay if no file was read, or an error if in attempting to read a file there was an error.
/// Returns boolean reflecting whether a file was read.
pub fn maybe_read_env_config_from_filename_and_paths<P: AsRef<Path>, Q: AsRef<Path>>(filename: P, paths: &[Q]) -> Result<bool, InitError> {
  Ok(do_read_env_config_from_filename_and_paths(filename, paths)?)
}

// Initialize dotenv with the default `.env` config file.
// Do not fail if the  file cannot be loaded.
fn init_dotenv() {
  match dotenvy::dotenv() {
    Ok(_) => println!("dotenv configs initialized"),
    Err(e) => println!("Could not initialize dotenv: {:?}", e),
  }
}

fn do_read_env_config_from_filename_and_paths<P: AsRef<Path>, Q: AsRef<Path>>(filename: P, paths: &[Q]) -> Result<bool, InitError> {
  for path in paths.iter() {
    let path = path.as_ref().join(filename.as_ref());

    if path.exists() && path.is_file() {
      log::info!("Attempting to read env vars from file: {:?}", path);
      let path = std::fs::canonicalize(path)
          .map_err(|err| {
            error!("Error canonicalizing path: {:?}", err);
            InitError::IoError
          })?;
      dotenvy::from_path(path)
          .map_err(|err| {
            error!("dotenv error reading config: {:?}", err);
            InitError::DotEnvError
          })?;
      return Ok(true);
    }
  }

  Ok(false)
}
