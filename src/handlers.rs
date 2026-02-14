use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::auth;
use crate::config::Config;
use crate::executor::Executor;
use crate::metrics::Metrics;
use crate::session::{EvalRequest, SessionManager};

pub struct AppState {
    pub config: Arc<Config>,
    pub sessions: Arc<SessionManager>,
    pub metrics: Arc<Metrics>,
    pub executor: Arc<Executor>,
    pub semaphore: Arc<Semaphore>,
    pub started_at: chrono::DateTime<Utc>,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/metrics", get(metrics))
        .route("/evaluate", post(evaluate))
        .route("/evaluate/{id}", get(get_eval))
        .route("/evaluations", get(list_evals))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

#[derive(Serialize)]
struct StatusResponse {
    version: String,
    uptime_secs: i64,
    active_evals: u64,
    total_evals: u64,
    passed: u64,
    failed: u64,
    cancelled: u64,
    capacity: usize,
    available_slots: usize,
}

async fn status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let uptime = (Utc::now() - state.started_at).num_seconds();
    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: uptime,
        active_evals: state.metrics.evals_active.load(Ordering::Relaxed),
        total_evals: state.metrics.evals_total.load(Ordering::Relaxed),
        passed: state.metrics.evals_passed.load(Ordering::Relaxed),
        failed: state.metrics.evals_failed.load(Ordering::Relaxed),
        cancelled: state.metrics.evals_cancelled.load(Ordering::Relaxed),
        capacity: state.config.max_concurrent_evals,
        available_slots: state.semaphore.available_permits(),
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

#[derive(Deserialize)]
struct EvalPayload {
    agent_code: String,
    #[serde(default = "default_language")]
    agent_language: String,
    task_url: String,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

fn default_language() -> String {
    "python".to_string()
}

async fn evaluate(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<EvalPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Auth check
    if let Some(ref expected) = state.config.auth_token {
        let auth_header = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());
        if !auth::check_token(auth_header, expected) {
            return Err((StatusCode::UNAUTHORIZED, "Invalid token".to_string()));
        }
    }

    // Validate payload
    if payload.agent_code.len() > state.config.max_agent_code_bytes {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "agent_code too large ({} bytes, max {})",
                payload.agent_code.len(),
                state.config.max_agent_code_bytes
            ),
        ));
    }

    if payload.task_url.len() > 2048 {
        return Err((
            StatusCode::BAD_REQUEST,
            "task_url too long (max 2048 chars)".to_string(),
        ));
    }

    if payload.task_url.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "task_url is required".to_string()));
    }

    if payload.agent_code.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "agent_code is required".to_string(),
        ));
    }

    // Capacity check
    let permit = state.semaphore.clone().try_acquire_owned();
    if permit.is_err() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "At capacity ({}/{}). Try again later.",
                state.config.max_concurrent_evals, state.config.max_concurrent_evals
            ),
        ));
    }

    let request = EvalRequest {
        agent_code: payload.agent_code,
        agent_language: payload.agent_language,
        task_url: payload.task_url,
        timeout_secs: payload.timeout_secs,
    };

    let session = state.sessions.create(request);
    let id = session.id.clone();

    // Spawn with permit held; permit is dropped when task completes
    let executor = state.executor.clone();
    let permit = permit.unwrap();
    tokio::spawn(async move {
        executor.spawn_eval(session);
        // Hold the permit until the session manager marks it done
        // We don't actually need to hold it since the semaphore tracks capacity
        drop(permit);
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "eval_id": id })),
    ))
}

async fn get_eval(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let session = state.sessions.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let result = session.result.lock().await;

    Ok(Json(serde_json::json!({
        "eval_id": session.id,
        "status": result.status,
        "step": result.step,
        "passed": result.passed,
        "test_results": result.test_results,
        "agent_output": result.agent_output,
        "test_output": result.test_output,
        "error": result.error,
        "duration_ms": result.duration_ms,
    })))
}

#[derive(Serialize)]
struct EvalListEntry {
    eval_id: String,
    task_url: String,
    language: String,
    created_at: String,
}

async fn list_evals(State(state): State<Arc<AppState>>) -> Json<Vec<EvalListEntry>> {
    let sessions = state.sessions.list_sessions();
    Json(
        sessions
            .into_iter()
            .map(|s| EvalListEntry {
                eval_id: s.id,
                task_url: s.task_url,
                language: s.language,
                created_at: s.created_at.to_rfc3339(),
            })
            .collect(),
    )
}
