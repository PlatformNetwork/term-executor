use anyhow::{Context, Result};
use std::collections::HashMap;
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

/// Filter out system-level package commands that require root (apt-get, dpkg, etc.).
/// In Basilica containers, the executor runs as non-root with no_new_privs,
/// so apt/sudo commands cannot succeed at runtime.
/// All system deps must be pre-installed in the Docker image at build time.
fn filter_install_command(cmd: &str) -> String {
    let system_prefixes = [
        "apt-get",
        "apt ",
        "dpkg",
        "yum ",
        "dnf ",
        "pacman ",
        "apk ",
        "snap ",
        "flatpak ",
        "sudo apt",
        "sudo dpkg",
    ];

    let parts: Vec<&str> = cmd.split("&&").collect();
    let filtered: Vec<&str> = parts
        .iter()
        .map(|p| p.trim())
        .filter(|p| !system_prefixes.iter().any(|prefix| p.starts_with(prefix)))
        .collect();

    filtered.join(" && ")
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
        agent_env: HashMap<String, String>,
    ) {
        let config = self.config.clone();
        let sessions = self.sessions.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let start = std::time::Instant::now();
            metrics.start_batch();

            let result = run_batch(&config, &batch, archive, concurrent_limit, agent_env).await;
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
    agent_env: HashMap<String, String>,
) -> Result<BatchResult> {
    let total_tasks = archive.tasks.len();
    let agent_code = Arc::new(archive.agent_code);
    let agent_language = Arc::new(archive.agent_language);
    let agent_archive = Arc::new(archive.agent_archive);
    let agent_env = Arc::new(agent_env);

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
    let batch_result = batch.result.clone();

    let mut handles = Vec::new();

    for task in archive.tasks {
        let config = config.clone();
        let batch_id = batch.id.clone();
        let events_tx = batch.events_tx.clone();
        let agent_code = agent_code.clone();
        let agent_language = agent_language.clone();
        let agent_archive = agent_archive.clone();
        let agent_env = agent_env.clone();
        let semaphore = semaphore.clone();
        let batch_result = batch_result.clone();
        let cancel_rx = batch.cancel.subscribe();

        let handle = tokio::spawn(async move {
            // Mark task as queued in batch result immediately
            {
                let mut res = batch_result.lock().await;
                let mut placeholder = TaskResult::new(task.id.clone());
                placeholder.status = TaskStatus::Queued;
                res.tasks.push(placeholder);
            }

            let _permit = match semaphore.acquire().await {
                Ok(p) => p,
                Err(_) => {
                    warn!(task_id = %task.id, "Semaphore closed, skipping task");
                    let mut res = batch_result.lock().await;
                    if let Some(t) = res.tasks.iter_mut().find(|t| t.task_id == task.id) {
                        t.status = TaskStatus::Failed;
                        t.error = Some("Semaphore closed".to_string());
                    }
                    res.completed_tasks += 1;
                    res.failed_tasks += 1;
                    return;
                }
            };

            let task_id = task.id.clone();

            // Mark task as running
            {
                let mut res = batch_result.lock().await;
                if let Some(t) = res.tasks.iter_mut().find(|t| t.task_id == task_id) {
                    t.status = TaskStatus::RunningAgent;
                }
            }

            let _ = events_tx.send(crate::session::WsEvent {
                event: "task_started".to_string(),
                batch_id: batch_id.clone(),
                task_id: Some(task_id.clone()),
                data: serde_json::json!({ "task_id": task_id }),
            });

            let result = run_single_task(
                &config,
                &task,
                &agent_code,
                &agent_language,
                agent_archive.as_deref(),
                &agent_env,
                cancel_rx,
            )
            .await;

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

            // Replace placeholder with real result
            {
                let mut res = batch_result.lock().await;
                if let Some(t) = res.tasks.iter_mut().find(|t| t.task_id == task_id) {
                    *t = result;
                }
                res.completed_tasks += 1;
                if res
                    .tasks
                    .iter()
                    .any(|t| t.task_id == task_id && t.reward == 1.0)
                {
                    res.passed_tasks += 1;
                } else {
                    res.failed_tasks += 1;
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        if let Err(e) = handle.await {
            warn!("Task handle panicked: {}", e);
        }
    }

    let res = batch.result.lock().await;
    let aggregate_reward = if total_tasks > 0 {
        res.tasks.iter().map(|r| r.reward).sum::<f64>() / total_tasks as f64
    } else {
        0.0
    };

    Ok(BatchResult {
        batch_id: batch.id.clone(),
        status: BatchStatus::Completed,
        total_tasks,
        completed_tasks: res.completed_tasks,
        passed_tasks: res.passed_tasks,
        failed_tasks: res.failed_tasks,
        tasks: res.tasks.clone(),
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
    agent_archive: Option<&[u8]>,
    agent_env: &HashMap<String, String>,
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
        agent_archive,
        agent_env,
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

#[allow(clippy::too_many_arguments)]
async fn run_task_pipeline(
    config: &Config,
    task: &SweForgeTask,
    agent_code: &str,
    agent_language: &str,
    agent_archive: Option<&[u8]>,
    agent_env: &HashMap<String, String>,
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
            let effective_cmd = filter_install_command(cmd);
            if effective_cmd.is_empty() {
                info!(
                    "[{}] Skipping system install: {}",
                    task.id,
                    &cmd[..cmd.len().min(100)]
                );
                continue;
            }
            info!("[{}] Installing: {}", task.id, effective_cmd);
            let (_, stderr, exit) = run_shell(
                &effective_cmd,
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
        agent_archive,
        &task.prompt,
        &repo_dir,
        config.agent_timeout_secs,
        agent_env,
    )
    .await?;

    // Capture git diff after agent runs (the patch the agent produced)
    let agent_patch =
        match run_cmd(&["git", "diff"], &repo_dir, Duration::from_secs(30), None).await {
            Ok((stdout, _, _)) => stdout,
            Err(_) => String::new(),
        };
    debug!("[{}] Agent patch: {} bytes", task.id, agent_patch.len());

    // Store agent output and patch for later retrieval
    let _ = tokio::fs::write(work_dir.join("agent_output.txt"), &agent_output).await;
    let _ = tokio::fs::write(work_dir.join("agent_patch.diff"), &agent_patch).await;

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
    result.agent_output = agent_output;
    result.agent_patch = agent_patch;

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
    agent_archive: Option<&[u8]>,
    prompt: &str,
    repo_dir: &Path,
    timeout_secs: u64,
    agent_env: &HashMap<String, String>,
) -> Result<String> {
    let prompt_path = repo_dir.join("_task_prompt.md");
    tokio::fs::write(&prompt_path, prompt).await?;

    // If we have the full archive, extract it into the repo so the agent project
    // structure (agent_code/agent.py, requirements.txt, src/, etc.) is preserved.
    let (argv_owned, run_dir) = if let Some(archive_bytes) = agent_archive {
        let agent_base = repo_dir.join("_agent");
        let _ = tokio::fs::create_dir_all(&agent_base).await;
        let base = agent_base.clone();
        let data = archive_bytes.to_vec();
        tokio::task::spawn_blocking(move || crate::task::extract_archive_bytes(&data, &base))
            .await
            .context("extract agent archive")??;

        // Find agent_code/ dir inside extracted archive
        let agent_dir = if agent_base.join("agent_code").exists() {
            agent_base.join("agent_code")
        } else {
            // Look one level deeper
            let mut found = agent_base.clone();
            if let Ok(mut entries) = tokio::fs::read_dir(&agent_base).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if entry.path().join("agent_code").exists() {
                        found = entry.path().join("agent_code");
                        break;
                    }
                }
            }
            found
        };

        // Install Python dependencies if requirements.txt exists
        if agent_dir.join("requirements.txt").exists() {
            info!("Installing agent requirements.txt");
            let (_, stderr, exit) = run_shell(
                "pip install --break-system-packages -q -r requirements.txt 2>&1 || pip3 install --break-system-packages -q -r requirements.txt 2>&1 || true",
                &agent_dir,
                Duration::from_secs(120),
                None,
            )
            .await?;
            if exit != 0 {
                warn!(
                    "Agent pip install failed (exit {}): {}",
                    exit,
                    &stderr[..stderr.len().min(500)]
                );
            }
        }

        // Determine entry point (use absolute path so we can run from repo_dir)
        let entry_file = if agent_dir.join("agent.py").exists() {
            agent_dir.join("agent.py")
        } else if agent_dir.join("main.py").exists() {
            agent_dir.join("main.py")
        } else {
            agent_dir.join("agent.py")
        };

        let mut argv = vec![
            "python3".to_string(),
            entry_file.to_string_lossy().to_string(),
        ];
        argv.push("--instruction".into());
        argv.push(prompt.into());
        // Run from repo_dir so agent's CWD is the target repo
        (argv, repo_dir.to_path_buf())
    } else {
        // Legacy path: single-file agent code written to _agent_code.py
        let ext = agent_extension(agent_language);
        let script_name = format!("_agent_code{}", ext);
        let script_path = repo_dir.join(&script_name);
        tokio::fs::write(&script_path, agent_code).await?;

        let mut argv = agent_runner(agent_language, &script_name);
        if matches!(agent_language.to_lowercase().as_str(), "python" | "py") {
            argv.push("--instruction".into());
            argv.push(prompt.into());
        }
        (argv, repo_dir.to_path_buf())
    };

    let argv: Vec<&str> = argv_owned.iter().map(|s| s.as_str()).collect();
    info!(
        "Running agent: {:?} in {} with {} env vars",
        argv,
        run_dir.display(),
        agent_env.len()
    );

    let mut all_env: Vec<(String, String)> = vec![
        (
            "TASK_PROMPT".into(),
            prompt_path.to_string_lossy().to_string(),
        ),
        ("REPO_DIR".into(), repo_dir.to_string_lossy().to_string()),
    ];
    for (k, v) in agent_env {
        all_env.push((k.clone(), v.clone()));
    }
    let env_refs: Vec<(&str, &str)> = all_env
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let (stdout, stderr, exit) = run_cmd(
        &argv,
        &run_dir,
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
