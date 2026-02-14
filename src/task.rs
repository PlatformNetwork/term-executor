use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

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
    pub test_scripts: Vec<(String, String)>, // (filename, content)
    pub test_source_files: Vec<(String, String)>, // (path, content)
}

pub async fn download_and_extract(url: &str, dest: &Path) -> Result<()> {
    info!("Downloading task archive from {}", url);
    let client = reqwest::Client::new();
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
    // Try ZIP first
    if let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(data)) {
        debug!("Extracting ZIP archive ({} entries)", archive.len());
        archive
            .extract(dest)
            .context("Failed to extract ZIP archive")?;
        return Ok(());
    }

    // Try tar.gz
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

    let tests_dir = task_dir.join("tests");
    let mut test_scripts = Vec::new();
    let mut test_source_files = Vec::new();

    if tests_dir.exists() {
        for entry in std::fs::read_dir(&tests_dir).context("Failed to read tests/")? {
            let entry = entry?;
            let path = entry.path();
            let fname = entry.file_name().to_string_lossy().to_string();

            if path.is_file() {
                let content = std::fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read test file: {}", fname))?;

                if fname.ends_with(".sh") {
                    test_scripts.push((fname, content));
                } else {
                    test_source_files.push((fname, content));
                }
            }
        }
    }

    Ok(SweForgeTask {
        workspace,
        prompt,
        test_scripts,
        test_source_files,
    })
}

/// Find the task root directory (handles nested extraction)
pub fn find_task_root(base: &Path) -> Result<PathBuf> {
    // Check if workspace.yaml is directly in base
    if base.join("workspace.yaml").exists() {
        return Ok(base.to_path_buf());
    }

    // Check one level deep (archive might have a top-level directory)
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
        assert_eq!(config.repo, "https://github.com/psf/requests");
        assert!(config.base_commit.is_none());
        assert!(config.install.is_none());
    }
}
