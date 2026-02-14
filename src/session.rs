use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalRequest {
    pub agent_code: String,
    pub agent_language: String,
    pub task_url: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EvalStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EvalStep {
    Queued,
    DownloadingTask,
    CloningRepo,
    InstallingDeps,
    RunningAgent,
    RunningTests,
    Cleanup,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTestResult {
    pub name: String,
    pub passed: bool,
    pub output: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub status: EvalStatus,
    pub step: EvalStep,
    pub passed: Option<bool>,
    pub test_results: Vec<TaskTestResult>,
    pub agent_output: String,
    pub test_output: String,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
}

pub struct Session {
    pub id: String,
    pub request: EvalRequest,
    pub result: Arc<Mutex<EvalResult>>,
    pub created_at: DateTime<Utc>,
    pub cancel: tokio::sync::watch::Sender<bool>,
}

#[allow(dead_code)]
pub struct SessionStats {
    pub created: AtomicU64,
    pub active: AtomicU64,
    pub completed: AtomicU64,
    pub failed: AtomicU64,
    pub cancelled: AtomicU64,
}

impl SessionStats {
    pub fn new() -> Self {
        Self {
            created: AtomicU64::new(0),
            active: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            cancelled: AtomicU64::new(0),
        }
    }
}

pub struct SessionManager {
    sessions: DashMap<String, Arc<Session>>,
    ttl_secs: u64,
    pub stats: SessionStats,
}

impl SessionManager {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            sessions: DashMap::new(),
            ttl_secs,
            stats: SessionStats::new(),
        }
    }

    pub fn create(&self, request: EvalRequest) -> Arc<Session> {
        let id = uuid::Uuid::new_v4().to_string();
        let (cancel_tx, _) = tokio::sync::watch::channel(false);

        let session = Arc::new(Session {
            id: id.clone(),
            request,
            result: Arc::new(Mutex::new(EvalResult {
                status: EvalStatus::Pending,
                step: EvalStep::Queued,
                passed: None,
                test_results: Vec::new(),
                agent_output: String::new(),
                test_output: String::new(),
                error: None,
                duration_ms: None,
            })),
            created_at: Utc::now(),
            cancel: cancel_tx,
        });

        self.sessions.insert(id, session.clone());
        self.stats.created.fetch_add(1, Ordering::Relaxed);
        self.stats.active.fetch_add(1, Ordering::Relaxed);
        session
    }

    pub fn get(&self, id: &str) -> Option<Arc<Session>> {
        self.sessions.get(id).map(|s| s.value().clone())
    }

    #[allow(dead_code)]
    pub fn remove(&self, id: &str) -> Option<Arc<Session>> {
        self.sessions.remove(id).map(|(_, s)| s)
    }

    #[allow(dead_code)]
    pub fn active_count(&self) -> usize {
        self.stats.active.load(Ordering::Relaxed) as usize
    }

    pub fn list_sessions(&self) -> Vec<SessionSummary> {
        self.sessions
            .iter()
            .map(|entry| {
                let s = entry.value();
                SessionSummary {
                    id: s.id.clone(),
                    task_url: s.request.task_url.clone(),
                    language: s.request.agent_language.clone(),
                    created_at: s.created_at,
                }
            })
            .collect()
    }

    pub fn mark_completed(&self) {
        self.stats.active.fetch_sub(1, Ordering::Relaxed);
        self.stats.completed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn mark_failed(&self) {
        self.stats.active.fetch_sub(1, Ordering::Relaxed);
        self.stats.failed.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn mark_cancelled(&self) {
        self.stats.active.fetch_sub(1, Ordering::Relaxed);
        self.stats.cancelled.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn reaper_loop(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let now = Utc::now();
            let mut expired = Vec::new();

            for entry in self.sessions.iter() {
                let age = (now - entry.value().created_at).num_seconds() as u64;
                if age > self.ttl_secs {
                    expired.push(entry.key().clone());
                }
            }

            for id in expired {
                if let Some((_, session)) = self.sessions.remove(&id) {
                    let _ = session.cancel.send(true);
                    info!("Reaped expired session {}", id);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub task_url: String,
    pub language: String,
    pub created_at: DateTime<Utc>,
}
