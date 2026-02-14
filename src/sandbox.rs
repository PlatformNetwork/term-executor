use anyhow::{Context, Result};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tracing::warn;

const MAX_OUTPUT_DEFAULT: usize = 1024 * 1024; // 1MB

pub struct SandboxOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub struct SandboxConfig {
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub memory_limit_mb: Option<u64>,
    pub nice: Option<i32>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(300),
            max_output_bytes: MAX_OUTPUT_DEFAULT,
            memory_limit_mb: None,
            nice: Some(10),
        }
    }
}

fn truncate_output(s: &[u8], max: usize) -> String {
    if s.len() <= max {
        String::from_utf8_lossy(s).to_string()
    } else {
        let truncated = String::from_utf8_lossy(&s[..max]).to_string();
        format!("{}\n\n... [truncated at {} bytes, total {}]", truncated, max, s.len())
    }
}

/// Build a shell command string with optional resource limits.
fn wrap_command(cmd: &str, cfg: &SandboxConfig) -> String {
    let mut parts = Vec::new();

    if let Some(nice) = cfg.nice {
        parts.push(format!("nice -n {}", nice));
    }

    if let Some(mem_mb) = cfg.memory_limit_mb {
        let kb = mem_mb * 1024;
        parts.push(format!("ulimit -v {} 2>/dev/null;", kb));
    }

    parts.push(cmd.to_string());
    parts.join(" ")
}

pub async fn run(
    cmd: &str,
    args: &[&str],
    cwd: &Path,
    cfg: &SandboxConfig,
    env: Option<&[(&str, &str)]>,
) -> Result<SandboxOutput> {
    let full_cmd = if args.is_empty() {
        cmd.to_string()
    } else {
        format!("{} {}", cmd, args.join(" "))
    };

    let wrapped = wrap_command(&full_cmd, cfg);

    let mut command = Command::new("sh");
    command.arg("-c").arg(&wrapped).current_dir(cwd);
    // Create new process group so we can kill the tree
    command.process_group(0);

    if let Some(env_vars) = env {
        for (k, v) in env_vars {
            command.env(k, v);
        }
    }

    command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let child = command.spawn().context("Failed to spawn process")?;

    let output = match tokio::time::timeout(cfg.timeout, child.wait_with_output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => anyhow::bail!("Process error: {}", e),
        Err(_) => {
            warn!("Command timed out after {}s: {}", cfg.timeout.as_secs(), &full_cmd[..full_cmd.len().min(100)]);
            anyhow::bail!("Command timed out after {}s", cfg.timeout.as_secs());
        }
    };

    Ok(SandboxOutput {
        stdout: truncate_output(&output.stdout, cfg.max_output_bytes),
        stderr: truncate_output(&output.stderr, cfg.max_output_bytes),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// Shorthand for running a shell string with default config.
pub async fn shell(
    shell_cmd: &str,
    cwd: &Path,
    timeout: Duration,
    env: Option<&[(&str, &str)]>,
) -> Result<SandboxOutput> {
    let cfg = SandboxConfig {
        timeout,
        ..Default::default()
    };
    run(shell_cmd, &[], cwd, &cfg, env).await
}

/// Get disk usage of a directory in bytes.
pub async fn disk_usage(path: &Path) -> Result<u64> {
    let output = Command::new("du")
        .args(["-sb", &path.to_string_lossy()])
        .output()
        .await
        .context("Failed to run du")?;

    let out = String::from_utf8_lossy(&output.stdout);
    let bytes: u64 = out
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    Ok(bytes)
}

/// Check if disk usage exceeds quota.
pub async fn check_disk_quota(path: &Path, quota_mb: u64) -> Result<bool> {
    let used = disk_usage(path).await?;
    let quota_bytes = quota_mb * 1024 * 1024;
    Ok(used <= quota_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_echo() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = SandboxConfig::default();
        let out = run("echo", &["hello"], tmp.path(), &cfg, None).await.unwrap();
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_run_timeout() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = SandboxConfig {
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let result = run("sleep", &["10"], tmp.path(), &cfg, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_exit_code() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = SandboxConfig::default();
        let out = run("false", &[], tmp.path(), &cfg, None).await.unwrap();
        assert_ne!(out.exit_code, 0);
    }

    #[tokio::test]
    async fn test_shell_shorthand() {
        let tmp = tempfile::tempdir().unwrap();
        let out = shell("echo world", tmp.path(), Duration::from_secs(5), None).await.unwrap();
        assert!(out.stdout.contains("world"));
    }

    #[tokio::test]
    async fn test_truncate_output() {
        let data = vec![b'A'; 2000];
        let result = truncate_output(&data, 100);
        assert!(result.contains("truncated"));
        assert!(result.len() < 2000);
    }

    #[tokio::test]
    async fn test_disk_usage() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        let usage = disk_usage(tmp.path()).await.unwrap();
        assert!(usage > 0);
    }

    #[tokio::test]
    async fn test_disk_quota_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let ok = check_disk_quota(tmp.path(), 1024).await.unwrap();
        assert!(ok);
    }

    #[test]
    fn test_wrap_command_nice() {
        let cfg = SandboxConfig { nice: Some(15), ..Default::default() };
        let wrapped = wrap_command("echo hi", &cfg);
        assert!(wrapped.contains("nice -n 15"));
    }
}
