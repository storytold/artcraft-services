use std::path::{Path, PathBuf};

/// Keep caller-supplied paths first, then resolve relative paths from this checkout.
pub(crate) fn config_search_directories<P: AsRef<Path>>(
  paths: &[P],
  maybe_repository_root: Option<&Path>,
) -> Vec<PathBuf> {
  let mut directories = paths.iter()
      .map(|path| path.as_ref().to_path_buf())
      .collect::<Vec<_>>();

  if let Some(root) = maybe_repository_root {
    for path in paths {
      if path.as_ref().is_relative() {
        let directory = root.join(path);
        if !directories.contains(&directory) {
          directories.push(directory);
        }
      }
    }
  }

  directories
}

/// Use the enclosing checkout, never a sibling repository's configuration.
pub(crate) fn find_repository_root(current_directory: &Path) -> Option<&Path> {
  current_directory.ancestors().find(|directory| {
    matches!(directory.file_name().and_then(|name| name.to_str()), Some("artcraft" | "artcraft-services"))
        && directory.join("Cargo.toml").is_file()
        && directory.join("crates").is_dir()
  })
}
