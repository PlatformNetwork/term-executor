use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::watch::Receiver;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::metrics::Metrics;
use crate::session::{
    EvalRequest, EvalResult, EvalStatus, EvalStep, Session, SessionManager, TaskTestResult,
};
use crate::task;

const MAX_OUTPUT: usize = 1024 * 1024;

fn truncate_output(raw: &[u8]) -> String {
    if raw.len() <= MAX_OUTPUT {
        String::from_utf8_lossy(raw).to_string()
    } else {
        let t = String::from_utf8_lossy(&raw[..MAX_OUTPUT]).to_string();
        format!(
            "{}\n\n... [truncated at {} bytes, total {}]",
            t,
            MAX_OUTPUT,
            raw.len()
        )
    }
}

async fn run_cmd(
    argv: &[&str],
    cwd: &Path,
    timeout: Duration,
    env: Option<&[(&str, &str)]>,
) -> Result<(String, String, i32)> {
    let (program, args) = argv.split_first().context("empty argv")?;

    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if let Some(vars) = env {
        for (k, v) in vars {
            cmd.env(k, v);
        }
    }

    let child = cmd.spawn().context("Failed to spawn process")?;

    let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => anyhow::bail!("Process error: {}", e),
        Err(_) => anyhow::bail!("Command timed out after {}s", timeout.as_secs()),
    };

    Ok((
        truncate_output(&output.stdout),
        truncate_output(&output.stderr),
        output.status.code().unwrap_or(-1),
    ))
}

async fn run_shell(
    shell_cmd: &str,
    cwd: &Path,
    timeout: Duration,
    env: Option<&[(&str, &str)]>,
) -> Result<(String, String, i32)> {
    run_cmd(&["sh", "-c", shell_cmd], cwd, timeout, env).await
}

pub struct Executor {
    config: Arc<Config>,
    sessions: Arc<SessionManager>,
    metrics: Arc<Metrics>,
}

impl Executor {
    pub fn new(config: Arc<Config>, sessions: Arc<SessionManager>, metrics: Arc<Metrics>) -> Self {
        Self {
            config,
            sessions,
            metrics,
        }
    }

    pub fn spawn_eval(&self, session: Arc<Session>) {
        let config = self.config.clone();
        let sessions = self.sessions.clone();
        let metrics = self.metrics.clone();
        let cancel_rx = session.cancel.subscribe();

        tokio::spawn(async move {
            let start = std::time::Instant::now();
            metrics.start_eval();

            let result = run_eval(&config, &session, cancel_rx).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            let mut res = session.result.lock().await;
            match result {
                Ok(eval) => {
                    let passed = eval.passed;
                    *res = eval;
                    res.duration_ms = Some(duration_ms);
                    metrics.finish_eval(passed, duration_ms);
                    if passed.unwrap_or(false) {
                        sessions.mark_completed();
                    } else {
                        sessions.mark_failed();
                    }
                }
                Err(e) => {
                    error!("Evaluation {} failed: {:#}", session.id, e);
                    res.status = EvalStatus::Failed;
                    res.step = EvalStep::Done;
                    res.error = Some(format!("{:#}", e));
                    res.duration_ms = Some(duration_ms);
                    metrics.finish_eval(None, duration_ms);
                    sessions.mark_failed();
                }
            }
        });
    }
}

async fn run_eval(
    config: &Config,
    session: &Session,
    cancel_rx: Receiver<bool>,
) -> Result<EvalResult> {
    let work_dir = config.workspace_base.join(&session.id);
    tokio::fs::create_dir_all(&work_dir).await?;

    let result = async {
        // 1. Download task
        set_step(session, EvalStep::DownloadingTask).await;
        if *cancel_rx.borrow() {
            anyhow::bail!("Cancelled");
        }

        let task_dir = work_dir.join("task");
        task::download_and_extract(&session.request.task_url, &task_dir).await?;
        let task_root = task::find_task_root(&task_dir)?;
        let swe_task = task::parse_task(&task_root)?;

        // 2. Clone repository
        set_step(session, EvalStep::CloningRepo).await;
        if *cancel_rx.borrow() {
            anyhow::bail!("Cancelled");
        }

        let repo_dir = work_dir.join("repo");
        clone_repo(
            &swe_task.workspace.repo,
            &repo_dir,
            config.clone_timeout_secs,
        )
        .await?;

        if let Some(ref commit) = swe_task.workspace.base_commit {
            checkout_commit(&repo_dir, commit, config.clone_timeout_secs).await?;
        }

        // 3. Install dependencies
        set_step(session, EvalStep::InstallingDeps).await;
        if *cancel_rx.borrow() {
            anyhow::bail!("Cancelled");
        }

        if let Some(ref install_cmds) = swe_task.workspace.install {
            for cmd in install_cmds {
                info!("Running install command: {}", cmd);
                let (_, stderr, exit) = run_shell(
                    cmd,
                    &repo_dir,
                    Duration::from_secs(config.clone_timeout_secs),
                    None,
                )
                .await?;
                if exit != 0 {
                    warn!(
                        "Install command failed (exit {}): {}",
                        exit,
                        &stderr[..stderr.len().min(500)]
                    );
                }
            }
        }

        // 4. Write + run agent code
        set_step(session, EvalStep::RunningAgent).await;
        if *cancel_rx.borrow() {
            anyhow::bail!("Cancelled");
        }

        let agent_output = run_agent(
            &session.request,
            &swe_task.prompt,
            &repo_dir,
            config.agent_timeout_secs,
        )
        .await?;

        // 5. Write test source files
        for (name, content) in &swe_task.test_source_files {
            let dest = repo_dir.join(name);
            if let Some(parent) = dest.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&dest, content).await?;
        }

        // 6. Run tests
        set_step(session, EvalStep::RunningTests).await;
        if *cancel_rx.borrow() {
            anyhow::bail!("Cancelled");
        }

        let test_results =
            run_tests(&swe_task.test_scripts, &repo_dir, config.test_timeout_secs).await?;

        let all_passed = test_results.iter().all(|t| t.passed);
        let test_output_combined = test_results
            .iter()
            .map(|t| {
                format!(
                    "=== {} (exit {}) ===\n{}\n{}",
                    t.name,
                    t.exit_code,
                    t.output,
                    if t.passed { "PASS" } else { "FAIL" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(EvalResult {
            status: EvalStatus::Completed,
            step: EvalStep::Done,
            passed: Some(all_passed),
            test_results,
            agent_output,
            test_output: test_output_combined,
            error: None,
            duration_ms: None,
        })
    }
    .await;

    // Cleanup
    set_step(session, EvalStep::Cleanup).await;
    crate::cleanup::remove_work_dir(&work_dir).await;

    result
}

async fn set_step(session: &Session, step: EvalStep) {
    let mut res = session.result.lock().await;
    res.step = step;
    if res.status == EvalStatus::Pending {
        res.status = EvalStatus::Running;
    }
}

async fn clone_repo(repo_url: &str, dest: &Path, timeout_secs: u64) -> Result<()> {
    info!("Cloning {} -> {}", repo_url, dest.display());

    let (_, stderr, exit) = run_cmd(
        &[
            "git",
            "clone",
            "--depth",
            "50",
            "--single-branch",
            repo_url,
            &dest.to_string_lossy(),
        ],
        dest.parent().unwrap_or(Path::new("/tmp")),
        Duration::from_secs(timeout_secs),
        None,
    )
    .await?;

    if exit != 0 {
        anyhow::bail!("git clone failed (exit {}): {}", exit, stderr);
    }
    Ok(())
}

async fn checkout_commit(repo_dir: &Path, commit: &str, timeout_secs: u64) -> Result<()> {
    info!("Checking out commit {}", commit);

    let (_, stderr, exit) = run_cmd(
        &["git", "checkout", commit],
        repo_dir,
        Duration::from_secs(timeout_secs),
        None,
    )
    .await?;

    if exit != 0 {
        warn!(
            "git checkout {} failed: {}",
            commit,
            &stderr[..stderr.len().min(300)]
        );
    }
    Ok(())
}

fn agent_extension(language: &str) -> &str {
    match language.to_lowercase().as_str() {
        "python" | "py" => ".py",
        "javascript" | "js" | "node" => ".js",
        "typescript" | "ts" => ".ts",
        "rust" | "rs" => ".rs",
        "go" | "golang" => ".go",
        "ruby" | "rb" => ".rb",
        "shell" | "bash" | "sh" => ".sh",
        _ => ".sh",
    }
}

fn agent_runner(language: &str, script_path: &str) -> Vec<String> {
    match language.to_lowercase().as_str() {
        "python" | "py" => vec!["python3".into(), script_path.into()],
        "javascript" | "js" | "node" => vec!["node".into(), script_path.into()],
        "typescript" | "ts" => vec!["npx".into(), "tsx".into(), script_path.into()],
        "go" | "golang" => vec!["go".into(), "run".into(), script_path.into()],
        "ruby" | "rb" => vec!["ruby".into(), script_path.into()],
        _ => vec!["bash".into(), script_path.into()],
    }
}

async fn run_agent(
    request: &EvalRequest,
    prompt: &str,
    repo_dir: &Path,
    timeout_secs: u64,
) -> Result<String> {
    let ext = agent_extension(&request.agent_language);
    let script_name = format!("_agent_code{}", ext);
    let script_path = repo_dir.join(&script_name);
    tokio::fs::write(&script_path, &request.agent_code).await?;

    let prompt_path = repo_dir.join("_task_prompt.md");
    tokio::fs::write(&prompt_path, prompt).await?;

    let argv_owned = agent_runner(&request.agent_language, &script_name);
    let argv: Vec<&str> = argv_owned.iter().map(|s| s.as_str()).collect();
    info!("Running agent: {:?}", argv);

    let env_vars = [
        ("TASK_PROMPT", prompt_path.to_string_lossy().to_string()),
        ("REPO_DIR", repo_dir.to_string_lossy().to_string()),
    ];
    let env_refs: Vec<(&str, &str)> = env_vars.iter().map(|(k, v)| (*k, v.as_str())).collect();

    let (stdout, stderr, exit) = run_cmd(
        &argv,
        repo_dir,
        Duration::from_secs(timeout_secs),
        Some(&env_refs),
    )
    .await?;

    if exit != 0 {
        warn!("Agent exited with code {}", exit);
    }

    Ok(format!("{}\n{}", stdout, stderr))
}

async fn run_tests(
    scripts: &[(String, String)],
    repo_dir: &Path,
    timeout_secs: u64,
) -> Result<Vec<TaskTestResult>> {
    let mut results = Vec::new();

    for (name, content) in scripts {
        let script_path = repo_dir.join(name);
        if let Some(parent) = script_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&script_path, content).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            let _ = std::fs::set_permissions(&script_path, perms);
        }

        debug!("Running test script: {}", name);
        let result = run_cmd(
            &["bash", &script_path.to_string_lossy()],
            repo_dir,
            Duration::from_secs(timeout_secs),
            None,
        )
        .await;

        match result {
            Ok((stdout, stderr, exit)) => {
                results.push(TaskTestResult {
                    name: name.clone(),
                    passed: exit == 0,
                    output: format!("{}\n{}", stdout, stderr),
                    exit_code: exit,
                });
            }
            Err(e) => {
                results.push(TaskTestResult {
                    name: name.clone(),
                    passed: false,
                    output: format!("Error: {:#}", e),
                    exit_code: -1,
                });
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_extension() {
        assert_eq!(agent_extension("python"), ".py");
        assert_eq!(agent_extension("js"), ".js");
        assert_eq!(agent_extension("rust"), ".rs");
        assert_eq!(agent_extension("go"), ".go");
        assert_eq!(agent_extension("unknown"), ".sh");
    }

    #[test]
    fn test_agent_runner() {
        let r = agent_runner("python", "agent.py");
        assert_eq!(r[0], "python3");
        let r = agent_runner("js", "agent.js");
        assert_eq!(r[0], "node");
    }

    #[test]
    fn test_truncate_output() {
        let small = vec![b'A'; 100];
        assert_eq!(truncate_output(&small).len(), 100);

        let big = vec![b'B'; MAX_OUTPUT + 500];
        let t = truncate_output(&big);
        assert!(t.contains("truncated"));
    }
}
