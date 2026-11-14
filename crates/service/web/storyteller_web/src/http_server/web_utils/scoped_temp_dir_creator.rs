use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// This utility will always create TempDirs at a specific mount path.
/// This is useful for k8s deployments where we may need our TempDirs
/// to live on a specific volume.
#[derive(Clone)]
pub struct ScopedTempDirCreator {
  base_dir: PathBuf,
}

impl ScopedTempDirCreator {
  pub fn auto_setup() -> Self {
    // Production configuration
    let maybe_tmp = easyenv::get_env_pathbuf_optional("TEMP_DIR");

    // MacOS
    // TODO: Only run this on macOS
    let maybe_mac_tmp = easyenv::get_env_pathbuf_optional("TMPDIR");

    let directory = maybe_tmp
        .or(maybe_mac_tmp)
        .unwrap_or_else(|| PathBuf::from("/tmp"));

    Self::for_directory(directory)
  }

  pub fn for_directory<P: AsRef<Path>>(base_dir: P) -> Self {
    Self {
      base_dir: PathBuf::from(&base_dir.as_ref()),
    }
  }

  pub fn get_base_dir(&self) -> &Path {
    &self.base_dir
  }

  pub fn new_tempdir(&self, prefix: &str) -> std::io::Result<TempDir> {
    tempfile::Builder::new().prefix(prefix).tempdir_in(&self.base_dir)
  }
}
