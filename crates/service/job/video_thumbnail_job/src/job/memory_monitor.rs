use std::path::Path;
use std::time::Duration;

use log::{info, warn};
use serde_json::json;
use tokio::fs;

struct MemoryUsage {
  current_bytes: u64,
  maybe_peak_bytes: Option<u64>,
  maybe_limit_bytes: Option<u64>,
}

/// cgroup v2 accounts for the Rust worker, FFmpeg children, and charged file cache.
pub async fn monitor_memory(concurrency: usize) {
  let root = Path::new("/sys/fs/cgroup");
  let mut interval = tokio::time::interval(Duration::from_secs(10));
  loop {
    interval.tick().await;
    let Some(usage) = read_memory_usage(root).await else {
      info!("Thumbnail container memory measurements unavailable (requires readable cgroup v2 memory files)");
      return;
    };
    let fields = json!({
      "configured_concurrency": concurrency,
      "current_bytes": usage.current_bytes,
      "peak_bytes": usage.maybe_peak_bytes,
      "limit_bytes": usage.maybe_limit_bytes,
    });
    if usage.maybe_limit_bytes.is_some_and(|limit| usage.current_bytes >= limit.saturating_mul(4) / 5) {
      warn!("thumbnail_memory {}", fields);
    } else {
      info!("thumbnail_memory {}", fields);
    }
  }
}

async fn read_memory_usage(root: &Path) -> Option<MemoryUsage> {
  let current_bytes = read_number(&root.join("memory.current")).await?;
  let maybe_peak_bytes = read_number(&root.join("memory.peak")).await;
  let maybe_limit_bytes = read_number(&root.join("memory.max")).await;
  Some(MemoryUsage { current_bytes, maybe_peak_bytes, maybe_limit_bytes })
}

async fn read_number(path: &Path) -> Option<u64> {
  // "max" denotes no cgroup memory limit; old kernels may lack memory.peak.
  fs::read_to_string(path).await.ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
  use tempdir::TempDir;

  use super::*;

  #[tokio::test]
  async fn reads_container_usage_and_handles_unlimited_or_missing_optional_metrics() {
    let directory = TempDir::new("thumbnail_memory_test").unwrap();
    let root = directory.path();
    fs::write(root.join("memory.current"), "1234\n").await.unwrap();
    fs::write(root.join("memory.max"), "max\n").await.unwrap();
    let usage = read_memory_usage(root).await.unwrap();
    assert_eq!(usage.current_bytes, 1234);
    assert_eq!(usage.maybe_peak_bytes, None);
    assert_eq!(usage.maybe_limit_bytes, None);

    fs::write(root.join("memory.peak"), "2345\n").await.unwrap();
    fs::write(root.join("memory.max"), "4096\n").await.unwrap();
    let usage = read_memory_usage(root).await.unwrap();
    assert_eq!(usage.maybe_peak_bytes, Some(2345));
    assert_eq!(usage.maybe_limit_bytes, Some(4096));
  }
}
