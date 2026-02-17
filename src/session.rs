use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    Pending,
    Extracting,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    CloningRepo,
    InstallingDeps,
    RunningAgent,
    RunningTests,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTestResult {
    pub name: String,
    pub passed: bool,
    pub output: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub passed: Option<bool>,
    pub reward: f64,
    pub test_results: Vec<TaskTestResult>,
    pub test_output: String,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
}

impl TaskResult {
    pub fn new(task_id: String) -> Self {
        Self {
            task_id,
            status: TaskStatus::Queued,
            passed: None,
            reward: 0.0,
            test_results: Vec::new(),
            test_output: String::new(),
            error: None,
            duration_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub batch_id: String,
    pub status: BatchStatus,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub passed_tasks: usize,
    pub failed_tasks: usize,
    pub tasks: Vec<TaskResult>,
    pub aggregate_reward: f64,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WsEvent {
    pub event: String,
    pub batch_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub data: serde_json::Value,
}

pub struct Batch {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub result: Arc<Mutex<BatchResult>>,
    pub events_tx: broadcast::Sender<WsEvent>,
    pub cancel: tokio::sync::watch::Sender<bool>,
}

impl Batch {
    pub async fn emit_event(&self, event: &str, task_id: Option<&str>, data: serde_json::Value) {
        let ws_event = WsEvent {
            event: event.to_string(),
            batch_id: self.id.clone(),
            task_id: task_id.map(|s| s.to_string()),
            data,
        };
        let _ = self.events_tx.send(ws_event);
    }
}

pub struct SessionStats {
    pub created: AtomicU64,
    pub active: AtomicU64,
    pub completed: AtomicU64,
    pub failed: AtomicU64,
}

impl SessionStats {
    pub fn new() -> Self {
        Self {
            created: AtomicU64::new(0),
            active: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            failed: AtomicU64::new(0),
        }
    }
}

pub struct SessionManager {
    batches: DashMap<String, Arc<Batch>>,
    ttl_secs: u64,
    pub stats: SessionStats,
}

impl SessionManager {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            batches: DashMap::new(),
            ttl_secs,
            stats: SessionStats::new(),
        }
    }

    pub fn create_batch(&self, total_tasks: usize) -> Arc<Batch> {
        let id = uuid::Uuid::new_v4().to_string();
        let (events_tx, _) = broadcast::channel(256);
        let (cancel_tx, _) = tokio::sync::watch::channel(false);

        let batch = Arc::new(Batch {
            id: id.clone(),
            created_at: Utc::now(),
            result: Arc::new(Mutex::new(BatchResult {
                batch_id: id.clone(),
                status: BatchStatus::Pending,
                total_tasks,
                completed_tasks: 0,
                passed_tasks: 0,
                failed_tasks: 0,
                tasks: Vec::new(),
                aggregate_reward: 0.0,
                error: None,
                duration_ms: None,
            })),
            events_tx,
            cancel: cancel_tx,
        });

        self.batches.insert(id, batch.clone());
        self.stats.created.fetch_add(1, Ordering::Relaxed);
        self.stats.active.fetch_add(1, Ordering::Relaxed);
        batch
    }

    pub fn get(&self, id: &str) -> Option<Arc<Batch>> {
        self.batches.get(id).map(|b| b.value().clone())
    }

    pub fn has_active_batch(&self) -> bool {
        for entry in self.batches.iter() {
            let result = entry.value().result.try_lock();
            if let Ok(r) = result {
                if r.status == BatchStatus::Running || r.status == BatchStatus::Extracting {
                    return true;
                }
            }
        }
        false
    }

    pub fn list_batches(&self) -> Vec<BatchSummary> {
        self.batches
            .iter()
            .map(|entry| {
                let b = entry.value();
                let status = b
                    .result
                    .try_lock()
                    .map(|r| r.status.clone())
                    .unwrap_or(BatchStatus::Running);
                BatchSummary {
                    batch_id: b.id.clone(),
                    created_at: b.created_at,
                    status,
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

    pub async fn reaper_loop(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let now = Utc::now();
            let mut expired = Vec::new();

            for entry in self.batches.iter() {
                let age = (now - entry.value().created_at).num_seconds() as u64;
                if age > self.ttl_secs {
                    expired.push(entry.key().clone());
                }
            }

            for id in expired {
                if let Some((_, batch)) = self.batches.remove(&id) {
                    let _ = batch.cancel.send(true);
                    info!("Reaped expired batch {}", id);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchSummary {
    pub batch_id: String,
    pub created_at: DateTime<Utc>,
    pub status: BatchStatus,
}
