use crate::state::expanduser::expanduser;
use crate::state::os_platform::OsPlatform;
use anyhow::anyhow;
use directories::UserDirs;
use ml_weights_registry::weights_registry::weight_descriptor::WeightDescriptor;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};
use tempfile::{Builder, NamedTempFile, TempDir};

const DEFAULT_DATA_DIR : &str = "artcraft";
const ASSETS_SUBDIRECTORY : &str = "assets";
const WEIGHTS_SUBDIRECTORY : &str = "weights";

const TEMPORARY_SUBDIRECTORY : &str = "temp";

/// Note: Tauri appends ".log" to the end of the filename.
const LOG_FILE_NAME : &str = "application_debug";

/// The path to the application data directory, which includes "asset" and "weights" data.
#[derive(Clone)]
pub struct AppDataRoot {
  path: PathBuf,
  assets_dir: AppAssetsDir,
  weights_dir: AppWeightsDir,
  temp_dir: TemporaryDir,
  log_file_name: PathBuf,
  log_file_name_string: String,
}

#[derive(Clone)]
pub struct AppAssetsDir {
  path: PathBuf,
}

#[derive(Clone)]
pub struct AppWeightsDir {
  path: PathBuf,
}

#[derive(Clone)]
pub struct TemporaryDir {
  path: PathBuf,
}

impl AppDataRoot {
  pub fn create_default() -> anyhow::Result<Self> {
    let directory = get_default_data_dir()?;
    Self::create_existing(directory)
  }

  pub fn create_existing<P: AsRef<Path>>(dir: P) -> anyhow::Result<Self> {
    let mut dir = dir.as_ref().to_path_buf();

    match OsPlatform::get() {
      OsPlatform::Linux | OsPlatform::MacOs => {
        if let Some(d) = dir.as_os_str().to_str() {
          dir = expanduser(d)?;
        }
      },
      OsPlatform::Windows => {}
    }

    if !dir.is_dir() {
      println!("Creating directory {:?}", dir);
      std::fs::create_dir_all(&dir)?;
    }

    match dir.canonicalize() {
      Ok(d) => dir = d,
      Err(err) => {
        println!("Error canonicalizing {:?}: {}", dir, err);
      }
    }

    let assets_dir = AppAssetsDir::create_existing(dir.join(ASSETS_SUBDIRECTORY))?;
    let weights_dir = AppWeightsDir::create_existing(dir.join(WEIGHTS_SUBDIRECTORY))?;
    let temp_dir = TemporaryDir::create_existing(dir.join(TEMPORARY_SUBDIRECTORY))?;
    let log_file_name = dir.join(LOG_FILE_NAME);
    let log_file_name_string = log_file_name
        .to_str()
        .ok_or(anyhow!("couldn't convert log path to str"))?
        .to_string();

    Ok(Self {
      path: dir,
      assets_dir,
      weights_dir,
      temp_dir,
      log_file_name,
      log_file_name_string,
    })
  }

  pub fn assets_dir(&self) -> &AppAssetsDir {
    &self.assets_dir
  }

  pub fn weights_dir(&self) -> &AppWeightsDir {
    &self.weights_dir
  }

  pub fn temp_dir(&self) -> &TemporaryDir {
    &self.temp_dir
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  pub fn log_file_name(&self) -> &Path {
    &self.log_file_name
  }

  pub fn log_file_name_str(&self) -> &str {
    &self.log_file_name_string
  }
}

impl AppAssetsDir {
  pub fn create_existing<P: AsRef<Path>>(dir: P) -> anyhow::Result<Self> {
    let mut dir = dir.as_ref().to_path_buf();
    match dir.canonicalize() {
      Ok(d) => dir = d,
      Err(err) => {
        println!("Error canonicalizing {:?}: {}", dir, err);
      }
    }
    if !dir.exists() {
      println!("Creating directory {:?}", dir);
      std::fs::create_dir(&dir)?;
    }
    Ok(Self {
      path: dir,
    })
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  /// Return the current date asset directory
  /// If it doesn't exist, create it.
  pub fn make_or_get_current_date_dir(&self) -> anyhow::Result<PathBuf> {
    let format: DateTime<Local> = Local::now();
    let date_directory_name = format.format("%Y-%d-%m").to_string();
    let full_path = self.path.join(date_directory_name);
    if !full_path.exists() {
      std::fs::create_dir(&full_path)?;
    }
    Ok(full_path)
  }
}

impl AppWeightsDir {
  pub fn create_existing<P: AsRef<Path>>(dir: P) -> anyhow::Result<Self> {
    let mut dir = dir.as_ref().to_path_buf();
    match dir.canonicalize() {
      Ok(d) => dir = d,
      Err(err) => {
        println!("Error canonicalizing {:?}: {}", dir, err);
      }
    }
    if !dir.exists() {
      println!("Creating directory {:?}", dir);
      std::fs::create_dir(&dir)?;
    }
    Ok(Self {
      path: dir,
    })
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  pub fn weight_path(&self, descriptor: &WeightDescriptor) -> PathBuf {
    self.path.join(descriptor.filename)
  }
}

impl TemporaryDir {
  pub fn create_existing<P: AsRef<Path>>(dir: P) -> anyhow::Result<Self> {
    let mut dir = dir.as_ref().to_path_buf();
    match dir.canonicalize() {
      Ok(d) => dir = d,
      Err(err) => {
        println!("Error canonicalizing {:?}: {}", dir, err);
      }
    }
    if !dir.exists() {
      println!("Creating directory {:?}", dir);
      std::fs::create_dir(&dir)?;
    }
    Ok(Self {
      path: dir,
    })
  }

  pub fn with_prefix(&self, prefix: &str) -> anyhow::Result<TempDir> {
    let tempdir = tempfile::Builder::new().prefix(prefix).tempdir_in(&self.path)?;
    Ok(tempdir)
  }

  pub fn new_named_temp_file(&self) -> anyhow::Result<NamedTempFile> {
    let tempfile = Builder::new()
      .prefix("temp_")
      .suffix(".bin")
      .tempfile_in(&self.path)?;
    Ok(tempfile)
  }
}

// eg. /home/bob/artcraft, /Users/bob/artcraft, or C:\Users\Alice\artcraft
fn get_default_data_dir() -> anyhow::Result<PathBuf> {
  Ok(UserDirs::new()
      .ok_or_else(|| anyhow!("could not determine user home directory"))?
      .home_dir()
      .join(DEFAULT_DATA_DIR))
}
