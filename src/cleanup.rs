use std::path::Path;
use tracing::{info, warn};

/// Remove a session's work directory.
pub async fn remove_work_dir(path: &Path) {
    if !path.exists() {
        return;
    }
    if let Err(e) = tokio::fs::remove_dir_all(path).await {
        warn!("Failed to cleanup {}: {}", path.display(), e);
    }
}

/// Kill all processes in a process group (best-effort).
#[allow(dead_code)]
pub async fn kill_process_group(pgid: u32) {
    let _ = tokio::process::Command::new("kill")
        .args(["-9", &format!("-{}", pgid)])
        .output()
        .await;
}

/// Scan workspace base for stale session directories older than max_age_secs.
pub async fn reap_stale_sessions(base: &Path, max_age_secs: u64) {
    let mut entries = match tokio::fs::read_dir(base).await {
        Ok(e) => e,
        Err(_) => return,
    };

    let now = std::time::SystemTime::now();
    let mut reaped = 0u32;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let metadata = match tokio::fs::metadata(&path).await {
            Ok(m) => m,
            Err(_) => continue,
        };
        let modified = match metadata.modified() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let age = now.duration_since(modified).unwrap_or_default();
        if age.as_secs() > max_age_secs {
            remove_work_dir(&path).await;
            reaped += 1;
        }
    }

    if reaped > 0 {
        info!("Reaped {} stale session directories", reaped);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_remove_work_dir_nonexistent() {
        remove_work_dir(Path::new("/tmp/nonexistent_test_dir_xyz")).await;
        // should not panic
    }

    #[tokio::test]
    async fn test_remove_work_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("session-test");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("file.txt"), "data")
            .await
            .unwrap();
        assert!(dir.exists());
        remove_work_dir(&dir).await;
        assert!(!dir.exists());
    }
}
