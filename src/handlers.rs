use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tracing::warn;

use crate::auth::{self, NonceStore};
use crate::config::Config;
use crate::executor::Executor;
use crate::metrics::Metrics;
use crate::session::SessionManager;
use crate::ws;

use crate::consensus::{ConsensusManager, ConsensusStatus};
use crate::validator_whitelist::ValidatorWhitelist;
use sha2::{Digest, Sha256};

pub struct AppState {
    pub config: Arc<Config>,
    pub sessions: Arc<SessionManager>,
    pub metrics: Arc<Metrics>,
    pub executor: Arc<Executor>,
    pub nonce_store: Arc<NonceStore>,
    pub started_at: chrono::DateTime<Utc>,
    pub validator_whitelist: Arc<ValidatorWhitelist>,
    pub consensus_manager: Arc<ConsensusManager>,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/metrics", get(metrics))
        .route("/submit", post(submit_batch))
        .route("/batch/{id}", get(get_batch))
        .route("/batch/{id}/tasks", get(get_batch_tasks))
        .route("/batch/{id}/task/{task_id}", get(get_task))
        .route("/batches", get(list_batches))
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

#[derive(Serialize)]
struct StatusResponse {
    version: String,
    uptime_secs: i64,
    active_batches: u64,
    total_batches: u64,
    completed_batches: u64,
    tasks_passed: u64,
    tasks_failed: u64,
    max_concurrent_tasks: usize,
    has_active_batch: bool,
}

async fn status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let uptime = (Utc::now() - state.started_at).num_seconds();
    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: uptime,
        active_batches: state.metrics.batches_active.load(Ordering::Relaxed),
        total_batches: state.metrics.batches_total.load(Ordering::Relaxed),
        completed_batches: state.metrics.batches_completed.load(Ordering::Relaxed),
        tasks_passed: state.metrics.tasks_passed.load(Ordering::Relaxed),
        tasks_failed: state.metrics.tasks_failed.load(Ordering::Relaxed),
        max_concurrent_tasks: state.config.max_concurrent_tasks,
        has_active_batch: state.sessions.has_active_batch(),
    })
}

async fn metrics(State(state): State<Arc<AppState>>) -> Response {
    let body = state.metrics.render_prometheus();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
        .into_response()
}

#[derive(serde::Deserialize)]
struct SubmitQuery {
    #[serde(default)]
    concurrent_tasks: Option<usize>,
}

async fn submit_batch(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    query: axum::extract::Query<SubmitQuery>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let auth_headers = auth::extract_auth_headers(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "missing_auth",
                "message": "Missing required headers: X-Hotkey, X-Nonce, X-Signature"
            })),
        )
    })?;

    if state.validator_whitelist.validator_count() == 0 {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "whitelist_not_ready",
                "message": "Validator whitelist not yet initialized. Please retry shortly."
            })),
        ));
    }

    if let Err(e) = auth::verify_request(
        &auth_headers,
        &state.nonce_store,
        &state.validator_whitelist,
    ) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": e.code(),
                "message": e.message(),
            })),
        ));
    }

    let max_bytes = state.config.max_archive_bytes;
    let mut archive_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "archive" || name == "file" {
            let mut buf = Vec::new();
            let mut stream = field;
            use futures::TryStreamExt;
            while let Some(chunk) = stream.try_next().await.map_err(|e| {
                warn!(error = %e, "Failed to read multipart chunk");
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "upload_failed",
                        "message": "Failed to read uploaded archive"
                    })),
                )
            })? {
                if buf.len() + chunk.len() > max_bytes {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "archive_too_large",
                            "message": format!(
                                "Archive exceeds maximum size of {} bytes",
                                max_bytes
                            )
                        })),
                    ));
                }
                buf.extend_from_slice(&chunk);
            }
            archive_data = Some(buf);
        }
    }

    let archive_bytes = archive_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "missing_archive",
                "message": "No archive file uploaded. Send a multipart form with field 'archive'."
            })),
        )
    })?;

    if state.consensus_manager.is_at_capacity() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "too_many_pending",
                "message": "Too many pending consensus entries. Please retry later."
            })),
        ));
    }

    let archive_hash = {
        let mut hasher = Sha256::new();
        hasher.update(&archive_bytes);
        hex::encode(hasher.finalize())
    };

    let total_validators = state.validator_whitelist.validator_count();
    let required_f = (total_validators as f64 * state.config.consensus_threshold).ceil();
    let required = (required_f.min(usize::MAX as f64) as usize).max(1);

    let concurrent = query
        .concurrent_tasks
        .unwrap_or(state.config.max_concurrent_tasks)
        .min(state.config.max_concurrent_tasks);

    let status = state.consensus_manager.record_vote(
        &archive_hash,
        &auth_headers.hotkey,
        Some(concurrent),
        required,
        total_validators,
    );

    match status {
        ConsensusStatus::Pending {
            votes,
            required,
            total_validators,
        } => Ok((
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "status": "pending_consensus",
                "archive_hash": archive_hash,
                "votes": votes,
                "required": required,
                "total_validators": total_validators,
            })),
        )),
        ConsensusStatus::AlreadyVoted {
            votes,
            required,
            total_validators,
        } => Ok((
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "status": "pending_consensus",
                "archive_hash": archive_hash,
                "votes": votes,
                "required": required,
                "total_validators": total_validators,
                "note": "Your vote was already recorded",
            })),
        )),
        ConsensusStatus::Reached {
            concurrent_tasks,
            votes,
            required,
        } => {
            let effective_concurrent = concurrent_tasks
                .unwrap_or(state.config.max_concurrent_tasks)
                .min(state.config.max_concurrent_tasks);

            if state.sessions.has_active_batch() {
                return Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({
                        "error": "busy",
                        "message": "A batch is already running. Wait for it to complete."
                    })),
                ));
            }

            let extract_dir = state.config.workspace_base.join("_extract_tmp");
            let _ = tokio::fs::remove_dir_all(&extract_dir).await;

            let extracted = crate::task::extract_uploaded_archive(&archive_bytes, &extract_dir)
                .await
                .map_err(|e| {
                    warn!(error = %e, "Failed to extract uploaded archive");
                    (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "extraction_failed",
                            "message": "Failed to extract archive. Ensure it is a valid zip or tar.gz."
                        })),
                    )
                })?;

            let _ = tokio::fs::remove_dir_all(&extract_dir).await;

            let total_tasks = extracted.tasks.len();
            let batch = state.sessions.create_batch(total_tasks);
            let batch_id = batch.id.clone();

            state
                .executor
                .spawn_batch(batch, extracted, effective_concurrent);

            Ok((
                StatusCode::ACCEPTED,
                Json(serde_json::json!({
                    "batch_id": batch_id,
                    "total_tasks": total_tasks,
                    "concurrent_tasks": effective_concurrent,
                    "ws_url": format!("/ws?batch_id={}", batch_id),
                    "consensus_reached": true,
                    "votes": votes,
                    "required": required,
                })),
            ))
        }
    }
}

async fn get_batch(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let batch = state.sessions.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let result = batch.result.lock().await;

    Ok(Json(serde_json::json!({
        "batch_id": result.batch_id,
        "status": result.status,
        "total_tasks": result.total_tasks,
        "completed_tasks": result.completed_tasks,
        "passed_tasks": result.passed_tasks,
        "failed_tasks": result.failed_tasks,
        "aggregate_reward": result.aggregate_reward,
        "error": result.error,
        "duration_ms": result.duration_ms,
    })))
}

async fn get_batch_tasks(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let batch = state.sessions.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let result = batch.result.lock().await;

    let tasks: Vec<serde_json::Value> = result
        .tasks
        .iter()
        .map(|t| {
            serde_json::json!({
                "task_id": t.task_id,
                "status": t.status,
                "passed": t.passed,
                "reward": t.reward,
                "test_output": t.test_output,
                "error": t.error,
                "duration_ms": t.duration_ms,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "batch_id": result.batch_id,
        "tasks": tasks,
    })))
}

async fn get_task(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((batch_id, task_id)): axum::extract::Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let batch = state.sessions.get(&batch_id).ok_or(StatusCode::NOT_FOUND)?;
    let result = batch.result.lock().await;

    let task = result
        .tasks
        .iter()
        .find(|t| t.task_id == task_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "task_id": task.task_id,
        "status": task.status,
        "passed": task.passed,
        "reward": task.reward,
        "test_results": task.test_results,
        "test_output": task.test_output,
        "error": task.error,
        "duration_ms": task.duration_ms,
    })))
}

#[derive(Serialize)]
struct BatchListEntry {
    batch_id: String,
    created_at: String,
    status: crate::session::BatchStatus,
}

async fn list_batches(State(state): State<Arc<AppState>>) -> Json<Vec<BatchListEntry>> {
    let batches = state.sessions.list_batches();
    Json(
        batches
            .into_iter()
            .map(|b| BatchListEntry {
                batch_id: b.batch_id,
                created_at: b.created_at.to_rfc3339(),
                status: b.status,
            })
            .collect(),
    )
}
