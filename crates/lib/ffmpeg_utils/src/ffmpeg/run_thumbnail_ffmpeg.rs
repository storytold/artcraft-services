use std::io::{Read, Seek, SeekFrom};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::bail;
use log::error;

use errors::AnyhowResult;

const THUMBNAIL_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_ERROR_LOG_BYTES: u64 = 16 * 1024;

/// Synchronous: callers in async runtimes must use spawn_blocking. A timeout
/// kills and reaps FFmpeg before returning, so it cannot outlive its job slot.
pub(super) fn run_thumbnail_ffmpeg(mut command: Command) -> AnyhowResult<()> {
  // Avoid holding arbitrarily large FFmpeg stderr output in the Rust heap.
  let mut stderr = tempfile::tempfile()?;
  let mut child = command.stdout(Stdio::null()).stderr(Stdio::from(stderr.try_clone()?)).spawn()?;
  let result = wait_for_exit(&mut child, THUMBNAIL_TIMEOUT);
  if !matches!(&result, Ok(status) if status.success()) {
    let start = stderr.metadata()?.len().saturating_sub(MAX_ERROR_LOG_BYTES);
    stderr.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    stderr.take(MAX_ERROR_LOG_BYTES).read_to_end(&mut bytes)?;
    error!("Thumbnail FFmpeg failed: {:?}; stderr: {}", result, String::from_utf8_lossy(&bytes));
  }
  let status = result?;
  if !status.success() {
    bail!("thumbnail FFmpeg failed: {status}");
  }
  Ok(())
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> AnyhowResult<ExitStatus> {
  let started_at = Instant::now();
  loop {
    match child.try_wait() {
      Ok(Some(status)) => return Ok(status),
      Ok(None) => {},
      Err(err) => {
        let _ = child.kill();
        let _ = child.wait();
        return Err(err.into());
      }
    }
    if started_at.elapsed() >= timeout {
      let _ = child.kill();
      child.wait()?;
      bail!("thumbnail FFmpeg exceeded timeout of {} seconds", timeout.as_secs_f64());
    }
    thread::sleep(Duration::from_millis(25));
  }
}

#[cfg(all(test, unix))]
mod tests {
  use super::*;

  #[test]
  fn timed_out_child_is_killed_and_reaped_before_returning() {
    let mut child = Command::new("sleep").arg("10").spawn().unwrap();
    let started_at = Instant::now();
    let result = wait_for_exit(&mut child, Duration::from_millis(50));
    assert!(result.unwrap_err().to_string().contains("exceeded timeout"));
    assert!(started_at.elapsed() < Duration::from_secs(3));
    assert!(!child.try_wait().unwrap().unwrap().success());
  }

  #[test]
  fn reports_normal_child_exit_status() {
    let mut child = Command::new("true").spawn().unwrap();
    assert!(wait_for_exit(&mut child, Duration::from_secs(3)).unwrap().success());
  }
}
