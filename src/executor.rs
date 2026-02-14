use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch::Receiver;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::metrics::Metrics;
use crate::sandbox;
use crate::session::{
    EvalRequest, EvalResult, EvalStatus, EvalStep, Session, SessionManager, TaskTestResult,
};
use crate::task;

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
        clone_repo(&swe_task.workspace.repo, &repo_dir, config.clone_timeout_secs).await?;

        if let Some(ref commit) = swe_task.workspace.base_commit {
            checkout_commit(&repo_dir, commit, config.clone_timeout_secs).await?;
        }

        // Check disk quota
        if !sandbox::check_disk_quota(&work_dir, config.disk_quota_mb).await? {
            anyhow::bail!(
                "Disk quota exceeded (max {}MB)",
                config.disk_quota_mb
            );
        }

        // 3. Install dependencies
        set_step(session, EvalStep::InstallingDeps).await;
        if *cancel_rx.borrow() {
            anyhow::bail!("Cancelled");
        }

        if let Some(ref install_cmds) = swe_task.workspace.install {
            for cmd in install_cmds {
                info!("Running install command: {}", cmd);
                let out = sandbox::shell(
                    cmd,
                    &repo_dir,
                    Duration::from_secs(config.clone_timeout_secs),
                    None,
                )
                .await?;
                if out.exit_code != 0 {
                    warn!("Install command failed (exit {}): {}", out.exit_code, &out.stderr[..out.stderr.len().min(500)]);
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

        let test_results = run_tests(
            &swe_task.test_scripts,
            &repo_dir,
            config.test_timeout_secs,
        )
        .await?;

        let all_passed = test_results.iter().all(|t| t.passed);
        let test_output_combined = test_results
            .iter()
            .map(|t| format!("=== {} (exit {}) ===\n{}\n{}", t.name, t.exit_code, t.output, if t.passed { "PASS" } else { "FAIL" }))
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
    info!("Cloning {} → {}", repo_url, dest.display());
    let tmp = dest.parent().unwrap_or(Path::new("/tmp"));

    let cmd = format!(
        "git clone --depth 50 --single-branch {} {}",
        shell_escape(repo_url),
        shell_escape(&dest.to_string_lossy())
    );

    let out = sandbox::shell(&cmd, tmp, Duration::from_secs(timeout_secs), None).await?;

    if out.exit_code != 0 {
        anyhow::bail!("git clone failed (exit {}): {}", out.exit_code, out.stderr);
    }

    Ok(())
}

async fn checkout_commit(repo_dir: &Path, commit: &str, timeout_secs: u64) -> Result<()> {
    info!("Checking out commit {}", commit);
    let cmd = format!("git checkout {}", shell_escape(commit));
    let out = sandbox::shell(&cmd, repo_dir, Duration::from_secs(timeout_secs), None).await?;
    if out.exit_code != 0 {
        warn!("git checkout {} failed: {}", commit, &out.stderr[..out.stderr.len().min(300)]);
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

fn agent_runner(language: &str, script_path: &str) -> String {
    match language.to_lowercase().as_str() {
        "python" | "py" => format!("python3 {}", script_path),
        "javascript" | "js" | "node" => format!("node {}", script_path),
        "typescript" | "ts" => format!("npx tsx {}", script_path),
        "rust" | "rs" => format!("rustc {} -o /tmp/agent && /tmp/agent", script_path),
        "go" | "golang" => format!("go run {}", script_path),
        "ruby" | "rb" => format!("ruby {}", script_path),
        _ => format!("bash {}", script_path),
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

    let run_cmd = agent_runner(&request.agent_language, &script_name);
    info!("Running agent: {}", run_cmd);

    let env_vars = [
        ("TASK_PROMPT", prompt_path.to_string_lossy().to_string()),
        ("REPO_DIR", repo_dir.to_string_lossy().to_string()),
    ];
    let env_refs: Vec<(&str, &str)> = env_vars
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();

    let out = sandbox::shell(
        &run_cmd,
        repo_dir,
        Duration::from_secs(timeout_secs),
        Some(&env_refs),
    )
    .await?;

    if out.exit_code != 0 {
        warn!("Agent exited with code {}", out.exit_code);
    }

    let combined = format!("{}\n{}", out.stdout, out.stderr);
    Ok(combined)
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

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            let _ = std::fs::set_permissions(&script_path, perms);
        }

        debug!("Running test script: {}", name);
        let out = sandbox::shell(
            &format!("bash {}", shell_escape(&script_path.to_string_lossy())),
            repo_dir,
            Duration::from_secs(timeout_secs),
            None,
        )
        .await;

        match out {
            Ok(o) => {
                results.push(TaskTestResult {
                    name: name.clone(),
                    passed: o.exit_code == 0,
                    output: format!("{}\n{}", o.stdout, o.stderr),
                    exit_code: o.exit_code,
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

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
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
        assert!(agent_runner("python", "agent.py").contains("python3"));
        assert!(agent_runner("js", "agent.js").contains("node"));
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello world"), "'hello world'");
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }
}
