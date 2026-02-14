use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
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

pub struct SessionManager {
    sessions: DashMap<String, Arc<Session>>,
    ttl_secs: u64,
}

impl SessionManager {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            sessions: DashMap::new(),
            ttl_secs,
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
        session
    }

    pub fn get(&self, id: &str) -> Option<Arc<Session>> {
        self.sessions.get(id).map(|s| s.value().clone())
    }

    pub fn remove(&self, id: &str) -> Option<Arc<Session>> {
        self.sessions.remove(id).map(|(_, s)| s)
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
