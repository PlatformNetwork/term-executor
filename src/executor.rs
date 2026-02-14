use anyhow::{Context, Result};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::session::TaskTestResult;
use crate::task::SweForgeTask;

const DEFAULT_CMD_TIMEOUT: Duration = Duration::from_secs(300);
const CLONE_TIMEOUT: Duration = Duration::from_secs(120);

pub struct ExecOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

async fn run_cmd(
    cmd: &str,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
    env: Option<&[(&str, &str)]>,
) -> Result<ExecOutput> {
    let mut command = Command::new(cmd);
    command.args(args).current_dir(cwd);

    if let Some(env_vars) = env {
        for (k, v) in env_vars {
            command.env(k, v);
        }
    }

    let output = tokio::time::timeout(timeout, command.output())
        .await
        .context("Command timed out")?
        .context("Failed to execute command")?;

    Ok(ExecOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

pub async fn setup_workspace(task: &SweForgeTask, work_dir: &Path) -> Result<()> {
    let repo_dir = work_dir.join("repo");
    tokio::fs::create_dir_all(&repo_dir)
        .await
        .context("Failed to create repo dir")?;

    // Clone repo at specified version
    info!("Cloning {} @ {}", task.workspace.repo, task.workspace.version);
    let clone_result = run_cmd(
        "git",
        &[
            "clone",
            "--depth",
            "1",
            "--branch",
            &task.workspace.version,
            &task.workspace.repo,
            ".",
        ],
        &repo_dir,
        CLONE_TIMEOUT,
        None,
    )
    .await;

    match clone_result {
        Ok(out) if out.exit_code == 0 => {
            debug!("Clone succeeded");
        }
        Ok(out) => {
            // Tag clone failed, try full clone + checkout
            warn!(
                "Shallow clone failed (exit {}), trying full clone",
                out.exit_code
            );
            let _ = tokio::fs::remove_dir_all(&repo_dir).await;
            tokio::fs::create_dir_all(&repo_dir).await?;

            let full = run_cmd(
                "git",
                &["clone", &task.workspace.repo, "."],
                &repo_dir,
                CLONE_TIMEOUT,
                None,
            )
            .await?;

            if full.exit_code != 0 {
                anyhow::bail!("Git clone failed: {}", full.stderr);
            }

            let checkout = run_cmd(
                "git",
                &["checkout", &task.workspace.version],
                &repo_dir,
                Duration::from_secs(30),
                None,
            )
            .await?;

            if checkout.exit_code != 0 {
                anyhow::bail!("Git checkout failed: {}", checkout.stderr);
            }
        }
        Err(e) => return Err(e),
    }

    // If base_commit specified, reset to it
    if let Some(ref base) = task.workspace.base_commit {
        info!("Resetting to base commit {}", base);
        let reset = run_cmd(
            "git",
            &["reset", "--hard", base],
            &repo_dir,
            Duration::from_secs(30),
            None,
        )
        .await?;
        if reset.exit_code != 0 {
            anyhow::bail!("Git reset to base commit failed: {}", reset.stderr);
        }
    }

    // Run install commands
    if let Some(ref install_cmds) = task.workspace.install {
        for cmd in install_cmds {
            info!("Running install: {}", cmd);
            let result = run_cmd("sh", &["-c", cmd], &repo_dir, DEFAULT_CMD_TIMEOUT, None).await?;
            if result.exit_code != 0 {
                warn!("Install command failed (exit {}): {}", result.exit_code, result.stderr);
            }
        }
    }

    // Write test source files into repo
    let tests_dir = repo_dir.join("__tests__");
    tokio::fs::create_dir_all(&tests_dir).await?;
    for (fname, content) in &task.test_source_files {
        let path = tests_dir.join(fname);
        tokio::fs::write(&path, content).await?;
    }

    Ok(())
}

pub async fn run_agent(
    task: &SweForgeTask,
    work_dir: &Path,
    agent_code: &str,
    agent_language: &str,
    timeout: Duration,
    mut cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<String> {
    let repo_dir = work_dir.join("repo");
    let agent_dir = work_dir.join("agent");
    tokio::fs::create_dir_all(&agent_dir).await?;

    // Write agent code
    let agent_file = match agent_language {
        "python" | "py" => "agent.py",
        "typescript" | "ts" => "agent.ts",
        "javascript" | "js" => "agent.js",
        "rust" | "rs" => "agent.rs",
        "go" => "agent.go",
        _ => "agent.py",
    };
    let agent_path = agent_dir.join(agent_file);
    tokio::fs::write(&agent_path, agent_code).await?;

    // Write the prompt as instruction file for the agent
    let instruction_path = work_dir.join("instruction.md");
    tokio::fs::write(&instruction_path, &task.prompt).await?;

    // Run the agent
    info!("Running agent ({}, timeout={}s)", agent_language, timeout.as_secs());
    let run_cmd_str = match agent_language {
        "python" | "py" => format!("cd {} && python3 -B {}", repo_dir.display(), agent_path.display()),
        "typescript" | "ts" => format!("cd {} && npx tsx {}", repo_dir.display(), agent_path.display()),
        "javascript" | "js" => format!("cd {} && node {}", repo_dir.display(), agent_path.display()),
        _ => format!("cd {} && python3 -B {}", repo_dir.display(), agent_path.display()),
    };

    let env_vars = [
        ("INSTRUCTION_FILE", instruction_path.to_str().unwrap_or("")),
        ("REPO_DIR", repo_dir.to_str().unwrap_or("")),
        ("WORK_DIR", work_dir.to_str().unwrap_or("")),
    ];

    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c")
        .arg(&run_cmd_str)
        .current_dir(&repo_dir);

    for (k, v) in &env_vars {
        cmd.env(k, v);
    }

    let start = Instant::now();
    let child = cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn agent process")?;

    let output = tokio::select! {
        result = child.wait_with_output() => {
            result.context("Agent process failed")?
        }
        _ = cancel_rx.changed() => {
            anyhow::bail!("Evaluation cancelled");
        }
        _ = tokio::time::sleep(timeout) => {
            anyhow::bail!("Agent timed out after {}s", timeout.as_secs());
        }
    };

    let agent_output = format!(
        "=== STDOUT ===\n{}\n=== STDERR ===\n{}\n=== EXIT: {} ({}ms) ===",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        output.status.code().unwrap_or(-1),
        start.elapsed().as_millis()
    );

    Ok(agent_output)
}

pub async fn run_tests(
    task: &SweForgeTask,
    work_dir: &Path,
    timeout: Duration,
) -> Result<(bool, Vec<TaskTestResult>, String)> {
    let repo_dir = work_dir.join("repo");
    let tests_dir = repo_dir.join("__tests__");

    let mut all_passed = true;
    let mut results = Vec::new();
    let mut combined_output = String::new();

    if task.test_scripts.is_empty() {
        anyhow::bail!("No test scripts found in task");
    }

    for (script_name, script_content) in &task.test_scripts {
        info!("Running test: {}", script_name);

        // Write test script
        let script_path = tests_dir.join(script_name);
        tokio::fs::write(&script_path, script_content).await?;

        // Make executable
        run_cmd("chmod", &["+x", script_path.to_str().unwrap_or("")], &repo_dir, Duration::from_secs(5), None).await?;

        // Run test
        let result = run_cmd(
            "sh",
            &[script_path.to_str().unwrap_or("")],
            &repo_dir,
            timeout,
            Some(&[
                ("REPO_DIR", repo_dir.to_str().unwrap_or("")),
                ("TESTS_DIR", tests_dir.to_str().unwrap_or("")),
            ]),
        )
        .await;

        let (passed, output, exit_code) = match result {
            Ok(out) => {
                let passed = out.exit_code == 0;
                let output = format!("{}\n{}", out.stdout, out.stderr);
                (passed, output, out.exit_code)
            }
            Err(e) => {
                let output = format!("Test execution error: {}", e);
                (false, output, -1)
            }
        };

        if !passed {
            all_passed = false;
        }

        combined_output.push_str(&format!("=== {} (exit {}) ===\n{}\n", script_name, exit_code, output));

        results.push(TaskTestResult {
            name: script_name.clone(),
            passed,
            output,
            exit_code,
        });
    }

    Ok((all_passed, results, combined_output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_run_cmd_echo() {
        let tmp = tempfile::tempdir().unwrap();
        let result = run_cmd("echo", &["hello"], tmp.path(), Duration::from_secs(5), None)
            .await
            .unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.trim().contains("hello"));
    }

    #[tokio::test]
    async fn test_run_cmd_timeout() {
        let tmp = tempfile::tempdir().unwrap();
        let result = run_cmd(
            "sleep",
            &["10"],
            tmp.path(),
            Duration::from_millis(100),
            None,
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_cmd_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let result = run_cmd("false", &[], tmp.path(), Duration::from_secs(5), None)
            .await
            .unwrap();
        assert_ne!(result.exit_code, 0);
    }
}
