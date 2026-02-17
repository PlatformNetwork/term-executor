use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

const MAX_ARCHIVE_SIZE: usize = 500 * 1024 * 1024;

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
    pub id: String,
    pub workspace: WorkspaceConfig,
    pub prompt: String,
    pub test_scripts: Vec<(String, String)>,
    pub test_source_files: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct ExtractedArchive {
    pub tasks: Vec<SweForgeTask>,
    pub agent_code: String,
    pub agent_language: String,
}

pub fn extract_archive_bytes(data: &[u8], dest: &Path) -> Result<()> {
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

pub async fn extract_uploaded_archive(data: &[u8], dest: &Path) -> Result<ExtractedArchive> {
    if data.len() > MAX_ARCHIVE_SIZE {
        anyhow::bail!(
            "Archive too large: {} bytes (max {})",
            data.len(),
            MAX_ARCHIVE_SIZE
        );
    }

    info!("Extracting {} bytes archive...", data.len());

    tokio::fs::create_dir_all(dest)
        .await
        .context("Failed to create extraction directory")?;

    let dest_owned = dest.to_path_buf();
    let data_vec = data.to_vec();
    tokio::task::spawn_blocking(move || extract_archive_bytes(&data_vec, &dest_owned))
        .await
        .context("Extract task panicked")??;

    let root = find_archive_root(dest)?;

    let agent_code = load_agent_code(&root)?;
    let agent_language = detect_agent_language(&root);
    let tasks = load_tasks(&root)?;

    info!(
        "Extracted {} tasks, agent language: {}",
        tasks.len(),
        agent_language
    );

    Ok(ExtractedArchive {
        tasks,
        agent_code,
        agent_language,
    })
}

fn find_archive_root(base: &Path) -> Result<PathBuf> {
    if base.join("tasks").exists() || base.join("agent_code").exists() {
        return Ok(base.to_path_buf());
    }

    for entry in std::fs::read_dir(base).context("Failed to read extracted directory")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir()
            && (path.join("tasks").exists() || path.join("agent_code").exists())
        {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "No tasks/ or agent_code/ found in archive at {}",
        base.display()
    )
}

fn load_agent_code(root: &Path) -> Result<String> {
    let agent_dir = root.join("agent_code");
    if !agent_dir.exists() {
        anyhow::bail!("agent_code/ directory not found in archive");
    }

    let mut agent_content = String::new();
    let mut files: Vec<_> = std::fs::read_dir(&agent_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();
    files.sort_by_key(|e| e.file_name());

    for entry in &files {
        let content = std::fs::read_to_string(entry.path())
            .with_context(|| format!("Failed to read agent file: {:?}", entry.path()))?;
        if files.len() == 1 {
            agent_content = content;
        } else {
            agent_content.push_str(&format!(
                "# --- {} ---\n",
                entry.file_name().to_string_lossy()
            ));
            agent_content.push_str(&content);
            agent_content.push('\n');
        }
    }

    if agent_content.is_empty() {
        anyhow::bail!("agent_code/ directory is empty");
    }

    Ok(agent_content)
}

fn detect_agent_language(root: &Path) -> String {
    let agent_dir = root.join("agent_code");
    if let Ok(entries) = std::fs::read_dir(&agent_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".py") {
                return "python".to_string();
            }
            if name.ends_with(".js") {
                return "javascript".to_string();
            }
            if name.ends_with(".ts") {
                return "typescript".to_string();
            }
            if name.ends_with(".sh") {
                return "shell".to_string();
            }
            if name.ends_with(".rs") {
                return "rust".to_string();
            }
            if name.ends_with(".go") {
                return "go".to_string();
            }
        }
    }
    "python".to_string()
}

fn load_tasks(root: &Path) -> Result<Vec<SweForgeTask>> {
    let tasks_dir = root.join("tasks");
    if !tasks_dir.exists() {
        anyhow::bail!("tasks/ directory not found in archive");
    }

    let mut tasks = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(&tasks_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let task_dir = entry.path();
        match parse_task(&task_dir) {
            Ok(task) => tasks.push(task),
            Err(e) => {
                tracing::warn!("Skipping task dir {}: {}", task_dir.display(), e);
            }
        }
    }

    if tasks.is_empty() {
        anyhow::bail!("No valid tasks found in tasks/ directory");
    }

    Ok(tasks)
}

pub fn parse_task(task_dir: &Path) -> Result<SweForgeTask> {
    let workspace_path = task_dir.join("workspace.yaml");
    let workspace_content =
        std::fs::read_to_string(&workspace_path).context("Missing workspace.yaml")?;
    let workspace: WorkspaceConfig =
        serde_yaml::from_str(&workspace_content).context("Invalid workspace.yaml")?;

    let prompt_path = task_dir.join("prompt.md");
    let prompt = std::fs::read_to_string(&prompt_path).context("Missing prompt.md")?;

    let id = task_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let mut test_scripts = Vec::new();
    let mut test_source_files = Vec::new();

    let tests_dir = task_dir.join("tests");
    if tests_dir.exists() {
        load_tests_recursive(
            &tests_dir,
            &tests_dir,
            &mut test_scripts,
            &mut test_source_files,
        )?;
    }

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
    }

    Ok(SweForgeTask {
        id,
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

#[allow(dead_code)]
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

    tokio::fs::create_dir_all(dest)
        .await
        .context("Failed to create extraction directory")?;

    let dest = dest.to_path_buf();
    let bytes_vec = bytes.to_vec();
    tokio::task::spawn_blocking(move || extract_archive_bytes(&bytes_vec, &dest))
        .await
        .context("Extract task panicked")??;

    Ok(())
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
    }

    #[test]
    fn test_detect_agent_language() {
        let tmp = tempfile::tempdir().unwrap();
        let agent_dir = tmp.path().join("agent_code");
        std::fs::create_dir_all(&agent_dir).unwrap();
        std::fs::write(agent_dir.join("main.py"), "print('hello')").unwrap();
        assert_eq!(detect_agent_language(tmp.path()), "python");
    }

    #[test]
    fn test_parse_task_with_checks() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(
            dir.join("workspace.yaml"),
            "repo: https://github.com/test/repo\nversion: v1.0\n",
        )
        .unwrap();
        std::fs::write(dir.join("prompt.md"), "Fix the bug").unwrap();
        std::fs::write(dir.join("checks.txt"), "pytest tests/\ncargo test\n").unwrap();

        let task = parse_task(dir).unwrap();
        assert_eq!(task.test_scripts.len(), 2);
    }
}
