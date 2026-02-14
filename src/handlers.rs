use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info};

use crate::session::{EvalRequest, EvalResult, EvalStatus};
use crate::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[derive(Serialize)]
pub struct StartEvalResponse {
    pub evaluation_id: String,
    pub status: &'static str,
}

pub async fn start_evaluation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EvalRequest>,
) -> Result<(StatusCode, Json<StartEvalResponse>), (StatusCode, Json<serde_json::Value>)> {
    if req.agent_code.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "agent_code is required"})),
        ));
    }
    if req.task_url.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "task_url is required"})),
        ));
    }

    let session = state.session_manager.create(req);
    let session_id = session.id.clone();

    // Spawn evaluation in background
    let sess = session.clone();
    tokio::spawn(async move {
        run_evaluation(sess).await;
    });

    info!("Started evaluation {}", session_id);

    Ok((
        StatusCode::ACCEPTED,
        Json(StartEvalResponse {
            evaluation_id: session_id,
            status: "pending",
        }),
    ))
}

async fn run_evaluation(session: Arc<crate::session::Session>) {
    let start = Instant::now();
    let eval_id = session.id.clone();

    // Mark as running
    {
        let mut result = session.result.lock().await;
        result.status = EvalStatus::Running;
    }

    let cancel_rx = session.cancel.subscribe();
    let timeout = Duration::from_secs(session.request.timeout_secs.unwrap_or(600));

    // Create work directory
    let work_dir = std::path::PathBuf::from(format!("/tmp/sessions/{}", eval_id));
    if let Err(e) = tokio::fs::create_dir_all(&work_dir).await {
        set_error(&session, &format!("Failed to create work dir: {}", e)).await;
        return;
    }

    // 1. Download and extract task
    info!("[{}] Downloading task from {}", eval_id, session.request.task_url);
    let extract_dir = work_dir.join("task");
    if let Err(e) = crate::task::download_and_extract(&session.request.task_url, &extract_dir).await {
        set_error(&session, &format!("Failed to download task: {}", e)).await;
        cleanup(&work_dir).await;
        return;
    }

    // 2. Parse task
    let task_root = match crate::task::find_task_root(&extract_dir) {
        Ok(r) => r,
        Err(e) => {
            set_error(&session, &format!("Invalid task format: {}", e)).await;
            cleanup(&work_dir).await;
            return;
        }
    };

    let task = match crate::task::parse_task(&task_root) {
        Ok(t) => t,
        Err(e) => {
            set_error(&session, &format!("Failed to parse task: {}", e)).await;
            cleanup(&work_dir).await;
            return;
        }
    };

    // 3. Setup workspace (clone repo, install deps)
    info!("[{}] Setting up workspace", eval_id);
    if let Err(e) = crate::executor::setup_workspace(&task, &work_dir).await {
        set_error(&session, &format!("Workspace setup failed: {}", e)).await;
        cleanup(&work_dir).await;
        return;
    }

    // 4. Run agent
    info!("[{}] Running agent", eval_id);
    let agent_output = match crate::executor::run_agent(
        &task,
        &work_dir,
        &session.request.agent_code,
        &session.request.agent_language,
        timeout,
        cancel_rx,
    )
    .await
    {
        Ok(output) => output,
        Err(e) => {
            let msg = format!("Agent execution failed: {}", e);
            info!("[{}] {}", eval_id, msg);
            // Agent failure is NOT an error — tests still run to check if anything passed
            msg
        }
    };

    // 5. Run tests
    info!("[{}] Running tests", eval_id);
    let test_timeout = Duration::from_secs(300);
    match crate::executor::run_tests(&task, &work_dir, test_timeout).await {
        Ok((passed, test_results, test_output)) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let mut result = session.result.lock().await;
            result.status = EvalStatus::Completed;
            result.passed = Some(passed);
            result.test_results = test_results;
            result.agent_output = agent_output;
            result.test_output = test_output;
            result.duration_ms = Some(duration_ms);
            info!(
                "[{}] Evaluation complete: passed={} ({}ms)",
                eval_id, passed, duration_ms
            );
        }
        Err(e) => {
            let mut result = session.result.lock().await;
            result.status = EvalStatus::Failed;
            result.agent_output = agent_output;
            result.error = Some(format!("Test execution failed: {}", e));
            result.duration_ms = Some(start.elapsed().as_millis() as u64);
            error!("[{}] Test execution failed: {}", eval_id, e);
        }
    }

    cleanup(&work_dir).await;
}

async fn set_error(session: &crate::session::Session, msg: &str) {
    let mut result = session.result.lock().await;
    result.status = EvalStatus::Failed;
    result.error = Some(msg.to_string());
    error!("[{}] {}", session.id, msg);
}

async fn cleanup(work_dir: &std::path::Path) {
    if let Err(e) = tokio::fs::remove_dir_all(work_dir).await {
        tracing::warn!("Failed to cleanup {}: {}", work_dir.display(), e);
    }
}

#[derive(Serialize)]
pub struct PollResponse {
    pub evaluation_id: String,
    #[serde(flatten)]
    pub result: EvalResult,
}

pub async fn poll_evaluation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PollResponse>, (StatusCode, Json<serde_json::Value>)> {
    let session = state.session_manager.get(&id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Evaluation not found"})),
        )
    })?;

    let result = session.result.lock().await.clone();

    Ok(Json(PollResponse {
        evaluation_id: id,
        result,
    }))
}

pub async fn cancel_evaluation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let session = state.session_manager.get(&id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Evaluation not found"})),
        )
    })?;

    let _ = session.cancel.send(true);

    {
        let mut result = session.result.lock().await;
        if matches!(result.status, EvalStatus::Pending | EvalStatus::Running) {
            result.status = EvalStatus::Cancelled;
        }
    }

    info!("Cancelled evaluation {}", id);
    Ok(StatusCode::NO_CONTENT)
}
