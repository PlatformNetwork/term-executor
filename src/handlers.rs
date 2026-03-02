use axum::{
    extract::{Multipart, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use base64::Engine;
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::RwLock;
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
    pub agent_archive: Arc<RwLock<Option<Vec<u8>>>>,
    pub agent_env: Arc<RwLock<HashMap<String, String>>>,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(upload_frontend))
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/metrics", get(metrics))
        .route("/upload-agent", post(upload_agent))
        .route("/upload-agent-json", post(upload_agent_json))
        .route("/agent-code", get(get_agent_code))
        .route("/submit", post(submit_batch))
        .route("/batch/:id", get(get_batch))
        .route("/batch/:id/tasks", get(get_batch_tasks))
        .route("/batch/:id/task/:task_id", get(get_task))
        .route("/batches", get(list_batches))
        .route("/verify/:batch_id", get(verify_batch))
        .route("/instance", get(instance_info))
        .route("/dataset", get(fetch_dataset))
        .route("/submit_tasks", post(submit_tasks))
        .route("/evaluate", post(evaluate_with_stored_agent))
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

async fn upload_frontend(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let enabled = state.config.sudo_password.is_some();
    let version = env!("CARGO_PKG_VERSION");

    if !enabled {
        return Html(format!(
            r##"<!DOCTYPE html><html><head><meta charset="utf-8"><title>term-executor</title>
<style>body{{background:#0a0a0a;color:#ff4444;font-family:monospace;display:flex;justify-content:center;padding:60px}}</style>
</head><body><div>term-executor v{version} — Upload disabled (SUDO_PASSWORD not set)</div></body></html>"##
        ));
    }

    Html(format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>term-executor</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{background:#0a0a0a;color:#e0e0e0;font-family:monospace;display:flex;justify-content:center;padding:40px 20px}}
.c{{max-width:720px;width:100%}}
h1{{color:#b2ff22;margin-bottom:8px;font-size:1.4em}}
.sub{{color:#666;margin-bottom:24px;font-size:0.85em}}
label{{display:block;color:#888;margin:12px 0 4px;font-size:0.85em}}
input[type=password],input[type=file]{{width:100%;padding:10px;background:#111;border:1px solid #333;color:#e0e0e0;border-radius:4px;font-family:monospace}}
textarea{{width:100%;height:120px;padding:10px;background:#111;border:1px solid #333;color:#ffbd2e;border-radius:4px;font-family:monospace;font-size:12px;resize:vertical}}
button{{margin-top:20px;padding:12px 32px;background:#b2ff22;color:#0a0a0a;border:none;border-radius:4px;font-weight:bold;cursor:pointer;font-family:monospace;font-size:1em;width:100%}}
button:hover{{background:#9de01a}}
button:disabled{{background:#333;color:#666;cursor:not-allowed}}
.r{{margin-top:20px;padding:16px;border-radius:4px;display:none;font-size:0.9em;word-break:break-all}}
.ok{{background:#0d1f00;border:1px solid #2a5a00;color:#b2ff22}}
.err{{background:#1f0000;border:1px solid #5a0000;color:#ff4444}}
.info{{color:#555;font-size:0.75em;margin-top:16px;line-height:1.6}}
</style>
</head>
<body>
<div class="c">
<h1>term-executor</h1>
<p class="sub">v{version} — Agent Upload</p>
<form id="f" onsubmit="return up(event)">
<label>Password</label>
<input type="password" id="pw" required autocomplete="off">
<label>Agent ZIP (project with requirements.txt + agent.py)</label>
<input type="file" id="zip" accept=".zip" required>
<label>Environment Variables (one per line: KEY=VALUE)</label>
<textarea id="env" placeholder="CHUTES_API_KEY=cpk_...&#10;MODEL_NAME=deepseek-ai/DeepSeek-V3-0324-TEE"></textarea>
<button type="submit" id="btn">Upload Agent</button>
</form>
<div id="res" class="r"></div>
<div class="info">
Upload a ZIP containing your agent project (agent.py, requirements.txt, etc).<br>
Env vars are injected when running agent.py. Stored in-memory only. TLS by Basilica.
</div>
</div>
<script>
async function up(e){{
  e.preventDefault();
  const btn=document.getElementById('btn'),res=document.getElementById('res');
  const file=document.getElementById('zip').files[0];
  if(!file){{res.style.display='block';res.className='r err';res.textContent='No ZIP selected';return false}}
  btn.disabled=true;btn.textContent='Uploading...';
  const fd=new FormData();
  fd.append('password',document.getElementById('pw').value);
  fd.append('archive',file);
  fd.append('env_vars',document.getElementById('env').value);
  try{{
    const r=await fetch('/upload-agent',{{method:'POST',body:fd}});
    const d=await r.json();
    res.style.display='block';
    if(r.ok){{
      res.className='r ok';
      res.textContent='Uploaded — hash: '+d.archive_hash+' ('+d.size_bytes+' bytes, '+d.files_count+' files, '+d.env_count+' env vars)';
    }}else{{
      res.className='r err';
      res.textContent='Error: '+(d.message||d.error||'unknown');
    }}
  }}catch(err){{
    res.style.display='block';res.className='r err';res.textContent='Network error: '+err.message;
  }}finally{{btn.disabled=false;btn.textContent='Upload Agent'}}
  return false;
}}
</script>
</body>
</html>"##
    ))
}

async fn upload_agent(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let expected = state.config.sudo_password.as_deref().ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "upload_disabled", "message": "SUDO_PASSWORD not configured"})))
    })?;

    let mut password: Option<String> = None;
    let mut archive_data: Option<Vec<u8>> = None;
    let mut env_vars_raw: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "password" => {
                password = field.text().await.ok();
            }
            "archive" | "file" => {
                let bytes = field.bytes().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": format!("Upload read failed: {}", e)})),
                    )
                })?;
                archive_data = Some(bytes.to_vec());
            }
            "env_vars" => {
                env_vars_raw = field.text().await.ok();
            }
            _ => {}
        }
    }

    let pw = password.unwrap_or_default();
    if !constant_time_eq(pw.as_bytes(), expected.as_bytes()) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_password", "message": "Invalid password"})),
        ));
    }

    let archive_bytes = archive_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error": "missing_archive", "message": "No ZIP file uploaded"}),
            ),
        )
    })?;

    if archive_bytes.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "empty_archive", "message": "ZIP file is empty"})),
        ));
    }

    const MAX_AGENT_SIZE: usize = 50 * 1024 * 1024; // 50MB
    if archive_bytes.len() > MAX_AGENT_SIZE {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error": "archive_too_large", "message": "ZIP exceeds 50MB limit"}),
            ),
        ));
    }

    // Validate it's a real ZIP
    let files_count = {
        let cursor = std::io::Cursor::new(&archive_bytes);
        let archive = zip::ZipArchive::new(cursor).map_err(|e| {
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid_zip", "message": format!("Not a valid ZIP: {}", e)})))
        })?;
        archive.len()
    };

    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(&archive_bytes);
        hex::encode(hasher.finalize())
    };
    let size = archive_bytes.len();

    // Parse env vars: KEY=VALUE per line, skip empty/comments
    let mut env_map = HashMap::new();
    if let Some(raw) = &env_vars_raw {
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim().to_string();
                let val = trimmed[eq_pos + 1..].trim().to_string();
                if !key.is_empty() {
                    env_map.insert(key, val);
                }
            }
        }
    }
    let env_count = env_map.len();

    *state.agent_archive.write().await = Some(archive_bytes);
    *state.agent_env.write().await = env_map;

    Ok(Json(serde_json::json!({
        "success": true,
        "archive_hash": hash,
        "size_bytes": size,
        "files_count": files_count,
        "env_count": env_count,
    })))
}

async fn upload_agent_json(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let expected = state.config.sudo_password.as_deref().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "upload_disabled"})),
        )
    })?;

    let pw = body.get("password").and_then(|v| v.as_str()).unwrap_or("");
    if !constant_time_eq(pw.as_bytes(), expected.as_bytes()) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_password"})),
        ));
    }

    let archive_b64 = body
        .get("archive_base64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "missing archive_base64 field"})),
            )
        })?;

    let archive_bytes = base64::engine::general_purpose::STANDARD
        .decode(archive_b64)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid base64: {}", e)})),
            )
        })?;

    if archive_bytes.is_empty() || archive_bytes.len() > 50 * 1024 * 1024 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "archive empty or too large"})),
        ));
    }

    let files_count = {
        let cursor = std::io::Cursor::new(&archive_bytes);
        let archive = zip::ZipArchive::new(cursor).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid zip: {}", e)})),
            )
        })?;
        archive.len()
    };

    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(&archive_bytes);
        hex::encode(hasher.finalize())
    };
    let size = archive_bytes.len();

    // Parse optional env vars
    let mut env_map = std::collections::HashMap::new();
    if let Some(env_obj) = body.get("env").and_then(|v| v.as_object()) {
        for (k, v) in env_obj {
            if let Some(s) = v.as_str() {
                env_map.insert(k.clone(), s.to_string());
            }
        }
    }
    let env_count = env_map.len();

    *state.agent_archive.write().await = Some(archive_bytes);
    *state.agent_env.write().await = env_map;

    Ok(Json(serde_json::json!({
        "success": true,
        "archive_hash": hash,
        "size_bytes": size,
        "files_count": files_count,
        "env_count": env_count,
    })))
}

async fn get_agent_code(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let expected = state.config.sudo_password.as_deref().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "disabled"})),
        )
    })?;

    let password = headers
        .get("x-password")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !constant_time_eq(password.as_bytes(), expected.as_bytes()) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_password"})),
        ));
    }

    let archive = state.agent_archive.read().await;
    match archive.as_deref() {
        Some(bytes) => Ok((
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/zip"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=\"agent.zip\"",
                ),
            ],
            bytes.to_vec(),
        )
            .into_response()),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(
                serde_json::json!({"error": "no_agent", "message": "No agent archive uploaded yet"}),
            ),
        )),
    }
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
                "agent_output": t.agent_output,
                "agent_patch": t.agent_patch,
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
        "agent_output": task.agent_output,
        "agent_patch": task.agent_patch,
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

/// Execution proof for a completed batch.
/// Returns a SHA256 hash of the batch results that validators can verify.
#[derive(Serialize)]
struct ExecutionProof {
    batch_id: String,
    status: crate::session::BatchStatus,
    total_tasks: usize,
    passed_tasks: usize,
    failed_tasks: usize,
    aggregate_reward: f64,
    /// SHA256 hash of: batch_id + task results (task_id, passed, reward) sorted
    results_hash: String,
    /// Per-task summary
    task_summaries: Vec<TaskSummary>,
    /// Executor version
    executor_version: String,
    /// Instance uptime in seconds
    uptime_secs: i64,
}

#[derive(Serialize)]
struct TaskSummary {
    task_id: String,
    passed: bool,
    reward: f64,
    duration_ms: Option<u64>,
}

async fn verify_batch(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(batch_id): axum::extract::Path<String>,
) -> Result<Json<ExecutionProof>, StatusCode> {
    let batch = state.sessions.get(&batch_id).ok_or(StatusCode::NOT_FOUND)?;
    let result = batch.result.lock().await;

    // Only return proof for completed batches
    if result.status != crate::session::BatchStatus::Completed
        && result.status != crate::session::BatchStatus::Failed
    {
        return Err(StatusCode::CONFLICT);
    }

    // Build deterministic hash of results
    let mut hasher = Sha256::new();
    hasher.update(result.batch_id.as_bytes());
    let mut sorted_tasks: Vec<_> = result.tasks.iter().collect();
    sorted_tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    for task in &sorted_tasks {
        hasher.update(task.task_id.as_bytes());
        hasher.update(if task.passed == Some(true) {
            b"1"
        } else {
            b"0"
        });
        hasher.update(task.reward.to_bits().to_le_bytes());
    }
    let results_hash = hex::encode(hasher.finalize());

    let task_summaries: Vec<TaskSummary> = sorted_tasks
        .iter()
        .map(|t| TaskSummary {
            task_id: t.task_id.clone(),
            passed: t.passed == Some(true),
            reward: t.reward,
            duration_ms: t.duration_ms,
        })
        .collect();

    let uptime = (Utc::now() - state.started_at).num_seconds();

    Ok(Json(ExecutionProof {
        batch_id: result.batch_id.clone(),
        status: result.status.clone(),
        total_tasks: result.total_tasks,
        passed_tasks: result.passed_tasks,
        failed_tasks: result.failed_tasks,
        aggregate_reward: result.aggregate_reward,
        results_hash,
        task_summaries,
        executor_version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: uptime,
    }))
}

/// Instance metadata — returns info about this executor instance.
/// Validators use this to verify the executor is running the expected image.
#[derive(Serialize)]
struct InstanceInfo {
    /// Executor version from Cargo.toml
    version: String,
    /// Image name (from IMAGE_NAME env var, set in Dockerfile)
    image: String,
    /// Image digest (from IMAGE_DIGEST env var, set at build/deploy time)
    image_digest: String,
    /// Uptime in seconds
    uptime_secs: i64,
    /// Node hostname
    hostname: String,
    /// Max concurrent tasks
    max_concurrent_tasks: usize,
    /// Bittensor netuid
    netuid: u16,
}

async fn instance_info(State(state): State<Arc<AppState>>) -> Json<InstanceInfo> {
    let uptime = (Utc::now() - state.started_at).num_seconds();
    Json(InstanceInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        image: std::env::var("IMAGE_NAME")
            .unwrap_or_else(|_| "platformnetwork/term-executor".to_string()),
        image_digest: std::env::var("IMAGE_DIGEST").unwrap_or_default(),
        uptime_secs: uptime,
        hostname: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_default(),
        max_concurrent_tasks: state.config.max_concurrent_tasks,
        netuid: state.config.bittensor_netuid,
    })
}

/// Fetch SWE-bench tasks from HuggingFace CortexLM/swe-forge dataset.
/// Query params: ?split=test&limit=10&offset=0&difficulty=hard
async fn fetch_dataset(
    axum::extract::Query(query): axum::extract::Query<DatasetQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let client = crate::swe_forge::client::HuggingFaceClient::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to create HF client: {}", e)})),
        )
    })?;

    let split = query.split.unwrap_or_else(|| "train".to_string());
    let limit = query.limit.unwrap_or(10).min(100);
    let offset = query.offset.unwrap_or(0);

    let config = crate::swe_forge::types::DatasetConfig {
        dataset_id: "CortexLM/swe-forge".to_string(),
        split: split.clone(),
        limit,
        offset,
    };

    let dataset = client.fetch_dataset(&config).await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": format!("Failed to fetch dataset: {}", e)})),
        )
    })?;

    // Filter by difficulty if specified
    let entries: Vec<&crate::swe_forge::types::DatasetEntry> =
        if let Some(ref diff) = query.difficulty {
            dataset
                .entries
                .iter()
                .filter(|e| {
                    e.difficulty
                        .as_deref()
                        .map(|d| d.eq_ignore_ascii_case(diff))
                        .unwrap_or(false)
                })
                .collect()
        } else {
            dataset.entries.iter().collect()
        };

    Ok(Json(serde_json::json!({
        "dataset_id": dataset.dataset_id,
        "split": dataset.split,
        "total_count": dataset.total_count,
        "returned": entries.len(),
        "entries": entries.iter().map(|e| serde_json::json!({
            "instance_id": e.instance_id,
            "repo": e.repo,
            "base_commit": e.base_commit,
            "problem_statement": &e.problem_statement[..e.problem_statement.len().min(500)],
            "fail_to_pass": e.fail_to_pass,
            "pass_to_pass": e.pass_to_pass,
            "version": e.version,
            "language": e.language,
            "difficulty": e.difficulty,
            "difficulty_score": e.difficulty_score,
            "quality_score": e.quality_score,
        })).collect::<Vec<_>>(),
    })))
}

#[derive(serde::Deserialize)]
struct DatasetQuery {
    split: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    difficulty: Option<String>,
}

/// Request body for /submit_tasks: validators provide task IDs to execute.
/// The executor fetches matching tasks from HuggingFace CortexLM/swe-forge,
/// pairs them with the uploaded agent archive, and runs them.
#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct SubmitTasksRequest {
    task_ids: Vec<String>,
    #[serde(default = "default_train_split")]
    split: String,
}

#[allow(dead_code)]
fn default_train_split() -> String {
    "train".to_string()
}

/// Accept a list of task_ids from validators, fetch them from HuggingFace,
/// and execute them with the agent code from the uploaded archive.
async fn submit_tasks(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Auth check
    let auth_headers = auth::extract_auth_headers(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "missing_auth",
                "message": "Missing required headers: X-Hotkey, X-Nonce, X-Signature"
            })),
        )
    })?;

    if state.validator_whitelist.validator_count() > 0 {
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
    }

    // Parse multipart: expect "task_ids" (JSON) and "archive" (file)
    let mut task_ids: Option<Vec<String>> = None;
    let mut split = "train".to_string();
    let mut archive_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "task_ids" => {
                let text = field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(
                            serde_json::json!({"error": format!("Failed to read task_ids: {}", e)}),
                        ),
                    )
                })?;
                task_ids = Some(serde_json::from_str::<Vec<String>>(&text).map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": format!("Invalid task_ids JSON: {}", e)})),
                    )
                })?);
            }
            "split" => {
                split = field.text().await.unwrap_or_else(|_| "train".to_string());
            }
            "archive" | "file" => {
                let mut buf = Vec::new();
                use futures::TryStreamExt;
                let mut stream = field;
                while let Some(chunk) = stream.try_next().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": format!("Upload failed: {}", e)})),
                    )
                })? {
                    buf.extend_from_slice(&chunk);
                }
                archive_data = Some(buf);
            }
            _ => {}
        }
    }

    let task_ids = task_ids.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Missing task_ids field"})),
        )
    })?;

    let archive_bytes = archive_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Missing archive file with agent code"})),
        )
    })?;

    if task_ids.is_empty() || task_ids.len() > 50 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "task_ids must have 1-50 entries"})),
        ));
    }

    // Fetch full dataset from HuggingFace to find matching tasks
    let hf_client = crate::swe_forge::client::HuggingFaceClient::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("HF client error: {}", e)})),
        )
    })?;

    let config = crate::swe_forge::types::DatasetConfig {
        dataset_id: "CortexLM/swe-forge".to_string(),
        split,
        limit: 100, // fetch all (dataset has 66 rows currently)
        offset: 0,
    };

    let dataset = hf_client.fetch_dataset(&config).await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": format!("Failed to fetch HF dataset: {}", e)})),
        )
    })?;

    // Match requested task_ids
    let matched: Vec<&crate::swe_forge::types::DatasetEntry> = dataset
        .entries
        .iter()
        .filter(|e| task_ids.contains(&e.instance_id))
        .collect();

    let not_found: Vec<&String> = task_ids
        .iter()
        .filter(|id| !matched.iter().any(|e| &e.instance_id == *id))
        .collect();

    if matched.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "No matching tasks found in dataset",
                "requested": task_ids,
                "available_count": dataset.entries.len(),
            })),
        ));
    }

    // Convert HF entries to SweForgeTask + build archive with tasks/ dirs
    let mut registry = crate::task::registry::TaskRegistry::new();
    let hf_dataset = crate::swe_forge::types::HuggingFaceDataset {
        dataset_id: dataset.dataset_id.clone(),
        split: dataset.split.clone(),
        entries: matched.into_iter().cloned().collect(),
        total_count: dataset.total_count,
    };
    registry.load_from_huggingface(&hf_dataset).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to load tasks: {}", e)})),
        )
    })?;

    // Extract agent code from uploaded archive
    let extract_dir = state.config.workspace_base.join("_extract_submit_tasks");
    let _ = tokio::fs::remove_dir_all(&extract_dir).await;
    let extracted = crate::task::extract_uploaded_archive(&archive_bytes, &extract_dir)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({"error": format!("Failed to extract agent archive: {}", e)}),
                ),
            )
        })?;
    let _ = tokio::fs::remove_dir_all(&extract_dir).await;

    // Replace the tasks from archive with the HF tasks, but keep the agent code
    let hf_tasks: Vec<crate::task::SweForgeTask> = registry.get_tasks().to_vec();
    let final_archive = crate::task::ExtractedArchive {
        tasks: hf_tasks,
        agent_code: extracted.agent_code,
        agent_language: extracted.agent_language,
    };

    if state.sessions.has_active_batch() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "A batch is already running"})),
        ));
    }

    let total_tasks = final_archive.tasks.len();
    let batch = state.sessions.create_batch(total_tasks);
    let batch_id = batch.id.clone();
    let concurrent = state.config.max_concurrent_tasks;

    state.executor.spawn_batch(batch, final_archive, concurrent);

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "batch_id": batch_id,
            "total_tasks": total_tasks,
            "matched_task_ids": task_ids.iter().filter(|id| !not_found.contains(id)).collect::<Vec<_>>(),
            "not_found": not_found,
            "ws_url": format!("/ws?batch_id={}", batch_id),
        })),
    ))
}

/// Evaluate using the stored agent archive (from /upload-agent).
/// Accepts JSON body: { "task_ids": [...], "split": "train" }
/// Auth: validator hotkey OR sudo password.
async fn evaluate_with_stored_agent(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Auth: try validator hotkey first, then sudo password
    let mut authed = false;

    if let Some(auth_headers) = auth::extract_auth_headers(&headers) {
        authed = state
            .config
            .trusted_validators
            .contains(&auth_headers.hotkey)
            || (state.validator_whitelist.validator_count() > 0
                && auth::verify_request(
                    &auth_headers,
                    &state.nonce_store,
                    &state.validator_whitelist,
                )
                .is_ok());
    }

    if !authed {
        if let Some(password) = headers
            .get("X-Password")
            .or_else(|| headers.get("x-password"))
            .and_then(|v| v.to_str().ok())
        {
            if let Some(ref sudo_pw) = state.config.sudo_password {
                if constant_time_eq(password.as_bytes(), sudo_pw.as_bytes()) {
                    authed = true;
                }
            }
        }
    }

    if !authed {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(
                serde_json::json!({"error": "unauthorized", "message": "Valid validator hotkey or sudo password required"}),
            ),
        ));
    }

    // Parse task_ids
    let task_ids: Vec<String> = body
        .get("task_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let _split = body
        .get("split")
        .and_then(|v| v.as_str())
        .unwrap_or("train")
        .to_string();

    if task_ids.is_empty() || task_ids.len() > 50 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "task_ids must have 1-50 entries"})),
        ));
    }

    // Get stored agent archive
    let archive_bytes = {
        let guard = state.agent_archive.read().await;
        guard.clone().ok_or_else(|| {
            (
                StatusCode::PRECONDITION_FAILED,
                Json(serde_json::json!({"error": "no_agent", "message": "No agent uploaded yet. Use /upload-agent first."})),
            )
        })?
    };

    // Download task files from HF repo (workspace.yaml, tests/*.sh, etc.)
    let hf_client = crate::swe_forge::client::HuggingFaceClient::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("HF client error: {}", e)})),
        )
    })?;

    let dataset_id = "CortexLM/swe-forge";
    let tasks_base = state.config.workspace_base.join("_hf_tasks");
    let _ = tokio::fs::remove_dir_all(&tasks_base).await;

    let mut hf_tasks: Vec<crate::task::SweForgeTask> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for task_id in &task_ids {
        let task_dir = tasks_base.join(task_id.replace('/', "__"));
        match hf_client
            .download_task_files(dataset_id, task_id, &task_dir)
            .await
        {
            Ok(()) => match crate::task::parse_task(&task_dir) {
                Ok(mut task) => {
                    task.id = task_id.clone();
                    hf_tasks.push(task);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse task {}: {}", task_id, e);
                    errors.push(format!("{}: parse error: {}", task_id, e));
                }
            },
            Err(e) => {
                tracing::warn!("Failed to download task {}: {}", task_id, e);
                errors.push(format!("{}: download error: {}", task_id, e));
            }
        }
    }

    if hf_tasks.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "No valid tasks found",
                "details": errors,
            })),
        ));
    }

    // Extract agent code only (no tasks/ required - we use HF tasks)
    let extract_dir = state.config.workspace_base.join("_extract_evaluate");
    let _ = tokio::fs::remove_dir_all(&extract_dir).await;
    let (agent_code, agent_language) =
        crate::task::extract_agent_only(&archive_bytes, &extract_dir)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": format!("Failed to extract agent: {}", e)})),
                )
            })?;
    let _ = tokio::fs::remove_dir_all(&extract_dir).await;

    let final_archive = crate::task::ExtractedArchive {
        tasks: hf_tasks,
        agent_code,
        agent_language,
    };

    if state.sessions.has_active_batch() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "A batch is already running. Try again later."})),
        ));
    }

    let total_tasks = final_archive.tasks.len();
    let batch = state.sessions.create_batch(total_tasks);
    let batch_id = batch.id.clone();
    let concurrent = state.config.max_concurrent_tasks;

    state.executor.spawn_batch(batch, final_archive, concurrent);

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "batch_id": batch_id,
            "total_tasks": total_tasks,
            "matched_task_ids": task_ids,
        })),
    ))
}
