use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::metrics::Metrics;
use crate::session::{
    Batch, BatchResult, BatchStatus, SessionManager, TaskResult, TaskStatus, TaskTestResult,
};
use crate::task::{ExtractedArchive, SweForgeTask};

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

    pub fn spawn_batch(
        &self,
        batch: Arc<Batch>,
        archive: ExtractedArchive,
        concurrent_limit: usize,
    ) {
        let config = self.config.clone();
        let sessions = self.sessions.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let start = std::time::Instant::now();
            metrics.start_batch();

            let result = run_batch(&config, &batch, archive, concurrent_limit).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            let mut res = batch.result.lock().await;
            match result {
                Ok(batch_result) => {
                    let all_passed = batch_result.passed_tasks == batch_result.total_tasks;
                    *res = batch_result;
                    res.duration_ms = Some(duration_ms);
                    metrics.finish_batch(all_passed, duration_ms);
                    sessions.mark_completed();
                }
                Err(e) => {
                    error!("Batch {} failed: {:#}", batch.id, e);
                    res.status = BatchStatus::Failed;
                    res.error = Some(format!("{:#}", e));
                    res.duration_ms = Some(duration_ms);
                    metrics.finish_batch(false, duration_ms);
                    sessions.mark_failed();
                }
            }

            batch
                .emit_event(
                    "batch_complete",
                    None,
                    serde_json::json!({
                        "status": res.status,
                        "total": res.total_tasks,
                        "passed": res.passed_tasks,
                        "failed": res.failed_tasks,
                        "reward": res.aggregate_reward,
                        "duration_ms": res.duration_ms,
                    }),
                )
                .await;
        });
    }
}

async fn run_batch(
    config: &Config,
    batch: &Batch,
    archive: ExtractedArchive,
    concurrent_limit: usize,
) -> Result<BatchResult> {
    let total_tasks = archive.tasks.len();
    let agent_code = Arc::new(archive.agent_code);
    let agent_language = Arc::new(archive.agent_language);

    {
        let mut res = batch.result.lock().await;
        res.status = BatchStatus::Running;
        res.total_tasks = total_tasks;
    }

    batch
        .emit_event(
            "batch_started",
            None,
            serde_json::json!({
                "total_tasks": total_tasks,
                "concurrent_limit": concurrent_limit,
            }),
        )
        .await;

    let semaphore = Arc::new(Semaphore::new(concurrent_limit));
    let task_results: Arc<tokio::sync::Mutex<Vec<TaskResult>>> =
        Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let mut handles = Vec::new();

    for task in archive.tasks {
        let config = config.clone();
        let batch_id = batch.id.clone();
        let events_tx = batch.events_tx.clone();
        let agent_code = agent_code.clone();
        let agent_language = agent_language.clone();
        let semaphore = semaphore.clone();
        let task_results = task_results.clone();
        let cancel_rx = batch.cancel.subscribe();

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();

            let task_id = task.id.clone();
            let _ = events_tx.send(crate::session::WsEvent {
                event: "task_started".to_string(),
                batch_id: batch_id.clone(),
                task_id: Some(task_id.clone()),
                data: serde_json::json!({ "task_id": task_id }),
            });

            let result =
                run_single_task(&config, &task, &agent_code, &agent_language, cancel_rx).await;

            let _ = events_tx.send(crate::session::WsEvent {
                event: "task_complete".to_string(),
                batch_id: batch_id.clone(),
                task_id: Some(task_id.clone()),
                data: serde_json::json!({
                    "task_id": task_id,
                    "status": result.status,
                    "passed": result.passed,
                    "reward": result.reward,
                }),
            });

            task_results.lock().await.push(result);
        });

        handles.push(handle);
    }

    for handle in handles {
        if let Err(e) = handle.await {
            warn!("Task handle panicked: {}", e);
        }
    }

    let results = task_results.lock().await;
    let completed = results.len();
    let passed = results.iter().filter(|r| r.reward == 1.0).count();
    let failed = completed - passed;
    let aggregate_reward = if total_tasks > 0 {
        results.iter().map(|r| r.reward).sum::<f64>() / total_tasks as f64
    } else {
        0.0
    };

    Ok(BatchResult {
        batch_id: batch.id.clone(),
        status: BatchStatus::Completed,
        total_tasks,
        completed_tasks: completed,
        passed_tasks: passed,
        failed_tasks: failed,
        tasks: results.clone(),
        aggregate_reward,
        error: None,
        duration_ms: None,
    })
}

async fn run_single_task(
    config: &Config,
    task: &SweForgeTask,
    agent_code: &str,
    agent_language: &str,
    cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> TaskResult {
    let start = std::time::Instant::now();
    let mut result = TaskResult::new(task.id.clone());

    let work_dir = config.workspace_base.join(&task.id);
    if let Err(e) = tokio::fs::create_dir_all(&work_dir).await {
        result.status = TaskStatus::Failed;
        result.error = Some(format!("Failed to create work dir: {}", e));
        return result;
    }

    let eval_result = run_task_pipeline(
        config,
        task,
        agent_code,
        agent_language,
        &work_dir,
        &cancel_rx,
    )
    .await;

    crate::cleanup::remove_work_dir(&work_dir).await;

    let duration_ms = start.elapsed().as_millis() as u64;

    match eval_result {
        Ok(mut r) => {
            r.duration_ms = Some(duration_ms);
            r
        }
        Err(e) => {
            result.status = TaskStatus::Failed;
            result.error = Some(format!("{:#}", e));
            result.duration_ms = Some(duration_ms);
            result
        }
    }
}

async fn run_task_pipeline(
    config: &Config,
    task: &SweForgeTask,
    agent_code: &str,
    agent_language: &str,
    work_dir: &Path,
    cancel_rx: &tokio::sync::watch::Receiver<bool>,
) -> Result<TaskResult> {
    let mut result = TaskResult::new(task.id.clone());

    if *cancel_rx.borrow() {
        anyhow::bail!("Cancelled");
    }

    result.status = TaskStatus::CloningRepo;
    let repo_dir = work_dir.join("repo");
    clone_repo(&task.workspace.repo, &repo_dir, config.clone_timeout_secs).await?;

    if let Some(ref commit) = task.workspace.base_commit {
        checkout_commit(&repo_dir, commit, config.clone_timeout_secs).await?;
    }

    if *cancel_rx.borrow() {
        anyhow::bail!("Cancelled");
    }

    result.status = TaskStatus::InstallingDeps;
    if let Some(ref install_cmds) = task.workspace.install {
        for cmd in install_cmds {
            info!("[{}] Installing: {}", task.id, cmd);
            let (_, stderr, exit) = run_shell(
                cmd,
                &repo_dir,
                Duration::from_secs(config.clone_timeout_secs),
                None,
            )
            .await?;
            if exit != 0 {
                warn!(
                    "[{}] Install failed (exit {}): {}",
                    task.id,
                    exit,
                    &stderr[..stderr.len().min(500)]
                );
            }
        }
    }

    if *cancel_rx.borrow() {
        anyhow::bail!("Cancelled");
    }

    result.status = TaskStatus::RunningAgent;
    let agent_output = run_agent(
        agent_code,
        agent_language,
        &task.prompt,
        &repo_dir,
        config.agent_timeout_secs,
    )
    .await?;
    let _ = agent_output;

    for (name, content) in &task.test_source_files {
        let dest = repo_dir.join(name);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&dest, content).await?;
    }

    if *cancel_rx.borrow() {
        anyhow::bail!("Cancelled");
    }

    result.status = TaskStatus::RunningTests;
    let test_results = run_tests(&task.test_scripts, &repo_dir, config.test_timeout_secs).await?;

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

    result.status = if all_passed {
        TaskStatus::Completed
    } else {
        TaskStatus::Failed
    };
    result.passed = Some(all_passed);
    result.reward = if all_passed { 1.0 } else { 0.0 };
    result.test_results = test_results;
    result.test_output = test_output_combined;

    Ok(result)
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
    agent_code: &str,
    agent_language: &str,
    prompt: &str,
    repo_dir: &Path,
    timeout_secs: u64,
) -> Result<String> {
    let ext = agent_extension(agent_language);
    let script_name = format!("_agent_code{}", ext);
    let script_path = repo_dir.join(&script_name);
    tokio::fs::write(&script_path, agent_code).await?;

    let prompt_path = repo_dir.join("_task_prompt.md");
    tokio::fs::write(&prompt_path, prompt).await?;

    let argv_owned = agent_runner(agent_language, &script_name);
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
