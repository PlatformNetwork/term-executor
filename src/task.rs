use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

const MAX_ARCHIVE_SIZE: usize = 100 * 1024 * 1024; // 100MB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub repo: String,
    pub version: String,
    #[serde(default)]
    pub base_commit: Option<String>,
    #[serde(default)]
    pub install: Option<Vec<String>>,
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug)]
pub struct SweForgeTask {
    pub workspace: WorkspaceConfig,
    pub prompt: String,
    pub test_scripts: Vec<(String, String)>,
    pub test_source_files: Vec<(String, String)>,
}

pub async fn download_and_extract(url: &str, dest: &Path) -> Result<()> {
    info!("Downloading task archive from {}", url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let resp = client
        .get(url)
        .send()
        .await
        .context("Failed to download task archive")?;

    if !resp.status().is_success() {
        anyhow::bail!(
            "Task archive download failed: HTTP {}",
            resp.status().as_u16()
        );
    }

    let bytes = resp.bytes().await.context("Failed to read response body")?;

    if bytes.len() > MAX_ARCHIVE_SIZE {
        anyhow::bail!(
            "Task archive too large: {} bytes (max {})",
            bytes.len(),
            MAX_ARCHIVE_SIZE
        );
    }

    info!("Downloaded {} bytes, extracting...", bytes.len());

    tokio::fs::create_dir_all(dest)
        .await
        .context("Failed to create extraction directory")?;

    let dest = dest.to_path_buf();
    let bytes_vec = bytes.to_vec();
    tokio::task::spawn_blocking(move || extract_archive(&bytes_vec, &dest))
        .await
        .context("Extract task panicked")??;

    Ok(())
}

fn extract_archive(data: &[u8], dest: &Path) -> Result<()> {
    if let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(data)) {
        debug!("Extracting ZIP archive ({} entries)", archive.len());
        archive
            .extract(dest)
            .context("Failed to extract ZIP archive")?;
        return Ok(());
    }

    let gz = flate2::read::GzDecoder::new(data);
    let mut archive = tar::Archive::new(gz);
    archive
        .unpack(dest)
        .context("Failed to extract tar.gz archive")?;
    debug!("Extracted tar.gz archive");

    Ok(())
}

pub fn parse_task(task_dir: &Path) -> Result<SweForgeTask> {
    let workspace_path = task_dir.join("workspace.yaml");
    let workspace_content =
        std::fs::read_to_string(&workspace_path).context("Missing workspace.yaml")?;
    let workspace: WorkspaceConfig =
        serde_yaml::from_str(&workspace_content).context("Invalid workspace.yaml")?;

    let prompt_path = task_dir.join("prompt.md");
    let prompt = std::fs::read_to_string(&prompt_path).context("Missing prompt.md")?;

    let mut test_scripts = Vec::new();
    let mut test_source_files = Vec::new();

    // Load from tests/ directory
    let tests_dir = task_dir.join("tests");
    if tests_dir.exists() {
        load_tests_recursive(
            &tests_dir,
            &tests_dir,
            &mut test_scripts,
            &mut test_source_files,
        )?;
    }

    // Load from checks.txt (alternative flat format)
    let checks_path = task_dir.join("checks.txt");
    if checks_path.exists() && test_scripts.is_empty() {
        let checks = std::fs::read_to_string(&checks_path).context("Failed to read checks.txt")?;
        for (i, line) in checks.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let name = format!("check_{}.sh", i);
            let content = format!("#!/bin/sh\nset -e\n{}\n", line);
            test_scripts.push((name, content));
        }
        if !test_scripts.is_empty() {
            info!(
                "Loaded {} test commands from checks.txt",
                test_scripts.len()
            );
        }
    }

    Ok(SweForgeTask {
        workspace,
        prompt,
        test_scripts,
        test_source_files,
    })
}

fn load_tests_recursive(
    base: &Path,
    dir: &Path,
    scripts: &mut Vec<(String, String)>,
    source_files: &mut Vec<(String, String)>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir).context("Failed to read tests directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            load_tests_recursive(base, &path, scripts, source_files)?;
            continue;
        }

        if !path.is_file() {
            continue;
        }

        let relative = path.strip_prefix(base).unwrap_or(&path);
        let fname = relative.to_string_lossy().to_string();

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read test file: {}", fname))?;

        if fname.ends_with(".sh") {
            scripts.push((fname, content));
        } else {
            source_files.push((fname, content));
        }
    }
    Ok(())
}

pub fn find_task_root(base: &Path) -> Result<PathBuf> {
    if base.join("workspace.yaml").exists() {
        return Ok(base.to_path_buf());
    }

    for entry in std::fs::read_dir(base).context("Failed to read extracted directory")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join("workspace.yaml").exists() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "No workspace.yaml found in extracted task archive at {}",
        base.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workspace_yaml() {
        let yaml = r#"
repo: "https://github.com/psf/requests"
version: "v2.31.0"
base_commit: "abc123"
install:
  - "pip install -e ."
language: "python"
"#;
        let config: WorkspaceConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.repo, "https://github.com/psf/requests");
        assert_eq!(config.version, "v2.31.0");
        assert_eq!(config.base_commit.as_deref(), Some("abc123"));
        assert_eq!(config.install.as_ref().unwrap().len(), 1);
        assert_eq!(config.language.as_deref(), Some("python"));
    }

    #[test]
    fn test_parse_workspace_minimal() {
        let yaml = r#"
repo: "https://github.com/psf/requests"
version: "v2.31.0"
"#;
        let config: WorkspaceConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.base_commit.is_none());
        assert!(config.install.is_none());
    }

    #[test]
    fn test_parse_task_with_checks_txt() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        std::fs::write(
            dir.join("workspace.yaml"),
            "repo: https://github.com/test/repo\nversion: v1.0\n",
        )
        .unwrap();
        std::fs::write(dir.join("prompt.md"), "Fix the bug").unwrap();
        std::fs::write(
            dir.join("checks.txt"),
            "# comment\npython -m pytest tests/\ncargo test\n",
        )
        .unwrap();

        let task = parse_task(dir).unwrap();
        assert_eq!(task.test_scripts.len(), 2);
        assert!(task.test_scripts[0].1.contains("pytest"));
        assert!(task.test_scripts[1].1.contains("cargo test"));
    }

    #[test]
    fn test_find_task_root_direct() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("workspace.yaml"), "repo: x\nversion: v1\n").unwrap();
        let root = find_task_root(tmp.path()).unwrap();
        assert_eq!(root, tmp.path());
    }

    #[test]
    fn test_find_task_root_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("task-dir");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("workspace.yaml"), "repo: x\nversion: v1\n").unwrap();
        let root = find_task_root(tmp.path()).unwrap();
        assert_eq!(root, nested);
    }

    #[test]
    fn test_find_task_root_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(find_task_root(tmp.path()).is_err());
    }
}
