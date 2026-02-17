use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub struct Metrics {
    pub batches_total: AtomicU64,
    pub batches_active: AtomicU64,
    pub batches_completed: AtomicU64,
    pub tasks_total: AtomicU64,
    pub tasks_passed: AtomicU64,
    pub tasks_failed: AtomicU64,
    pub duration_sum_ms: AtomicU64,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            batches_total: AtomicU64::new(0),
            batches_active: AtomicU64::new(0),
            batches_completed: AtomicU64::new(0),
            tasks_total: AtomicU64::new(0),
            tasks_passed: AtomicU64::new(0),
            tasks_failed: AtomicU64::new(0),
            duration_sum_ms: AtomicU64::new(0),
        })
    }

    pub fn start_batch(&self) {
        self.batches_total.fetch_add(1, Ordering::Relaxed);
        self.batches_active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn finish_batch(&self, all_passed: bool, duration_ms: u64) {
        self.batches_active.fetch_sub(1, Ordering::Relaxed);
        self.batches_completed.fetch_add(1, Ordering::Relaxed);
        self.duration_sum_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        if all_passed {
            self.tasks_passed.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[allow(dead_code)]
    pub fn record_task_result(&self, passed: bool) {
        self.tasks_total.fetch_add(1, Ordering::Relaxed);
        if passed {
            self.tasks_passed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.tasks_failed.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn render_prometheus(&self) -> String {
        let batches_total = self.batches_total.load(Ordering::Relaxed);
        let batches_active = self.batches_active.load(Ordering::Relaxed);
        let batches_completed = self.batches_completed.load(Ordering::Relaxed);
        let tasks_total = self.tasks_total.load(Ordering::Relaxed);
        let tasks_passed = self.tasks_passed.load(Ordering::Relaxed);
        let tasks_failed = self.tasks_failed.load(Ordering::Relaxed);
        let dur_sum = self.duration_sum_ms.load(Ordering::Relaxed);

        format!(
            "# HELP term_executor_batches_total Total batches submitted.\n\
             # TYPE term_executor_batches_total counter\n\
             term_executor_batches_total {}\n\
             # HELP term_executor_batches_active Currently running batches.\n\
             # TYPE term_executor_batches_active gauge\n\
             term_executor_batches_active {}\n\
             # HELP term_executor_batches_completed Completed batches.\n\
             # TYPE term_executor_batches_completed counter\n\
             term_executor_batches_completed {}\n\
             # HELP term_executor_tasks_total Total tasks evaluated.\n\
             # TYPE term_executor_tasks_total counter\n\
             term_executor_tasks_total {}\n\
             # HELP term_executor_tasks_passed Tasks that passed (reward=1).\n\
             # TYPE term_executor_tasks_passed counter\n\
             term_executor_tasks_passed {}\n\
             # HELP term_executor_tasks_failed Tasks that failed (reward=0).\n\
             # TYPE term_executor_tasks_failed counter\n\
             term_executor_tasks_failed {}\n\
             # HELP term_executor_duration_ms_sum Sum of batch durations in ms.\n\
             # TYPE term_executor_duration_ms_sum counter\n\
             term_executor_duration_ms_sum {}\n",
            batches_total,
            batches_active,
            batches_completed,
            tasks_total,
            tasks_passed,
            tasks_failed,
            dur_sum
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_lifecycle() {
        let m = Metrics::new();
        m.start_batch();
        assert_eq!(m.batches_active.load(Ordering::Relaxed), 1);
        assert_eq!(m.batches_total.load(Ordering::Relaxed), 1);

        m.finish_batch(true, 5000);
        assert_eq!(m.batches_active.load(Ordering::Relaxed), 0);
        assert_eq!(m.batches_completed.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_prometheus_output() {
        let m = Metrics::new();
        m.start_batch();
        m.finish_batch(false, 1234);
        let out = m.render_prometheus();
        assert!(out.contains("term_executor_batches_total 1"));
        assert!(out.contains("term_executor_duration_ms_sum 1234"));
    }
}
