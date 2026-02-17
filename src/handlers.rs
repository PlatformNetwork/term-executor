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

use crate::auth::{self, NonceStore};
use crate::config::Config;
use crate::executor::Executor;
use crate::metrics::Metrics;
use crate::session::SessionManager;
use crate::ws;

pub struct AppState {
    pub config: Arc<Config>,
    pub sessions: Arc<SessionManager>,
    pub metrics: Arc<Metrics>,
    pub executor: Arc<Executor>,
    pub nonce_store: Arc<NonceStore>,
    pub started_at: chrono::DateTime<Utc>,
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
    #[serde(default = "default_concurrent")]
    concurrent_tasks: Option<usize>,
}

fn default_concurrent() -> Option<usize> {
    None
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

    if let Err(e) = auth::verify_request(&auth_headers, &state.nonce_store) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": e.code(),
                "message": e.message(),
            })),
        ));
    }

    if state.sessions.has_active_batch() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "busy",
                "message": "A batch is already running. Wait for it to complete."
            })),
        ));
    }

    let mut archive_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "archive" || name == "file" {
            let data = field.bytes().await.map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "upload_failed",
                        "message": format!("Failed to read upload: {}", e)
                    })),
                )
            })?;
            archive_data = Some(data.to_vec());
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

    if archive_bytes.len() > state.config.max_archive_bytes {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "archive_too_large",
                "message": format!("Archive is {} bytes, max is {}", archive_bytes.len(), state.config.max_archive_bytes)
            })),
        ));
    }

    let extract_dir = state.config.workspace_base.join("_extract_tmp");
    let _ = tokio::fs::remove_dir_all(&extract_dir).await;

    let extracted = crate::task::extract_uploaded_archive(&archive_bytes, &extract_dir)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "extraction_failed",
                    "message": format!("Failed to extract archive: {}", e)
                })),
            )
        })?;

    let _ = tokio::fs::remove_dir_all(&extract_dir).await;

    let total_tasks = extracted.tasks.len();
    let concurrent = query
        .concurrent_tasks
        .unwrap_or(state.config.max_concurrent_tasks)
        .min(state.config.max_concurrent_tasks);

    let batch = state.sessions.create_batch(total_tasks);
    let batch_id = batch.id.clone();

    state.executor.spawn_batch(batch, extracted, concurrent);

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "batch_id": batch_id,
            "total_tasks": total_tasks,
            "concurrent_tasks": concurrent,
            "ws_url": format!("/ws?batch_id={}", batch_id),
        })),
    ))
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
