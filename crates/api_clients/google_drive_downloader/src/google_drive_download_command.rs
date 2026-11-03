use std::process::Command;

use anyhow::anyhow;
use log::info;
use tempdir::TempDir;

use crockford::random_crockford_token;
use errors::AnyhowResult;
use subprocess_common::docker_options::DockerOptions;

/// This is a Python script that uses the `gdown` package to download from Google Drive.
/// We're using this because it's a hack that gets around OAuth gateways. All the Rust
/// crates require OAuth permissions. Ugh.
///
/// This script lives here: https://github.com/storytold/web-downloader
///
/// Filename: `download_internet_file.py`
/// Arguments:
///   --url (google drive, web, or youtube url)
///   --output_filename (local download filename)
#[derive(Clone)]
pub struct GoogleDriveDownloadCommand {
  /// The python downloader script.
  download_script: String,

  /// If present, change to this directory before doing anything.
  maybe_change_to_directory: Option<String>,

  /// If a virtual environment is necessary, this is the name of the activation script (which will be sourced.)
  maybe_venv_activation_script: Option<String>,

  /// If this is run under Docker (eg. in development), these are the options for Docker (GPU, mount, etc.)
  maybe_docker_options: Option<DockerOptions>,
}

impl GoogleDriveDownloadCommand {
  pub fn new(
    download_script: &str,
    maybe_change_to_directory: Option<&str>,
    maybe_venv_activation_script: Option<&str>,
    maybe_docker_options: Option<DockerOptions>
  ) -> Self {
    Self {
      download_script: download_script.to_string(),
      maybe_change_to_directory: maybe_change_to_directory.map(|s| s.to_string()),
      maybe_venv_activation_script: maybe_venv_activation_script.map(|s| s.to_string()),
      maybe_docker_options,
    }
  }

  pub fn new_production(download_script: &str) -> Self {
    Self {
      download_script: download_script.to_string(),
      maybe_change_to_directory: None,
      maybe_venv_activation_script: None,
      maybe_docker_options: None,
    }
  }

  pub async fn download_file_with_file_name(&self,
                             download_url: &str,
                             temp_dir: &TempDir,
                             file_name:&str) -> AnyhowResult<String>
  {
    let temp_dir_path = temp_dir.path()
        .to_str()
        .unwrap_or("/tmp")
        .to_string();

    let temp_filename = format!("{}/{}", temp_dir_path, file_name);

    self.run_downloader(download_url, &temp_filename)?;

    Ok(temp_filename)
  }

  /// Download file from Google Drive into the `TempDir`.
  /// Return the local filesystem filename.
  pub async fn download_file(&self,
                         download_url: &str,
                         temp_dir: &TempDir) -> AnyhowResult<String>
  {
    let temp_dir_path = temp_dir.path()
      .to_str()
      .unwrap_or("/tmp")
      .to_string();

    let temp_filename = random_crockford_token(10);
    let temp_filename = format!("{}/{}.bin", temp_dir_path, temp_filename);

    self.run_downloader(download_url, &temp_filename)?;

    Ok(temp_filename)
  }

  fn run_downloader(&self, download_url: &str, output_filename: &str) -> AnyhowResult<()> {
    info!("Downloading {} to: {}", download_url, output_filename);

    let mut command = self.build_command(download_url, output_filename);

    info!("Running command: {:?}", command);

    let result = command.output()?;

    info!("Downloader Result: {:?}", result);

    if !result.status.success() {
      let reason = String::from_utf8(result.stderr).unwrap_or("UNKNOWN".to_string());
      return Err(anyhow!("Failure to execute command: {:?}", reason))
    }

    Ok(())
  }

  /// Build the downloader process.
  ///
  /// The download script, venv and working directory are trusted operator configuration and
  /// are still composed into a shell script. The URL and output filename are request-derived,
  /// so they are never placed in shell source: they are passed as positional parameters and
  /// referenced as "$1" / "$2", which the shell expands without further interpretation.
  fn build_command(&self, download_url: &str, output_filename: &str) -> Command {
    let mut script = format!("{} --url \"$1\" --output_filename \"$2\"", &self.download_script);

    if let Some(venv_activation_script) = self.maybe_venv_activation_script.as_deref() {
      // NB: "." is source for non-bash shells
      script = format!(". {} && {}", venv_activation_script, &script);
    }

    if let Some(change_to_directory) = self.maybe_change_to_directory.as_deref() {
      script = format!("cd {} && {}", change_to_directory, &script);
    }

    match self.maybe_docker_options.as_ref() {
      Some(docker_options) => {
        let mut command = Command::new("docker");
        command.args(docker_options.to_command_args(&script, &[download_url, output_filename]));
        command
      }
      None => {
        let mut command = Command::new("sh");
        command.arg("-c")
            .arg(script)
            .arg("sh") // $0
            .arg(download_url)
            .arg(output_filename);
        command
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::ffi::OsStr;

  use super::GoogleDriveDownloadCommand;

  const HOSTILE_URL: &str = "https://example.com/$(touch /tmp/pwned)`id`\"; echo injected; \"";

  #[test]
  fn url_is_passed_as_literal_argument() {
    // `printf` stands in for the downloader script: it echoes each argument it receives.
    let downloader = GoogleDriveDownloadCommand::new_production("printf '%s\\n'");

    let output = downloader.build_command(HOSTILE_URL, "/tmp/out file.bin")
        .output()
        .expect("sh should run");

    assert!(output.status.success());
    assert_eq!(
      String::from_utf8(output.stdout).unwrap(),
      format!("--url\n{}\n--output_filename\n/tmp/out file.bin\n", HOSTILE_URL));
  }

  #[test]
  fn url_is_not_part_of_shell_source() {
    let downloader = GoogleDriveDownloadCommand::new(
      "./download_internet_file.py", Some("/opt/downloader"), Some("venv/bin/activate"), None);

    let command = downloader.build_command(HOSTILE_URL, "/tmp/out.bin");
    let args: Vec<&OsStr> = command.get_args().collect();

    assert_eq!(command.get_program(), "sh");
    assert_eq!(args[0], "-c");
    assert_eq!(args[1], "cd /opt/downloader && . venv/bin/activate && ./download_internet_file.py --url \"$1\" --output_filename \"$2\"");
    assert_eq!(&args[2..], &["sh", HOSTILE_URL, "/tmp/out.bin"]);
  }
}
