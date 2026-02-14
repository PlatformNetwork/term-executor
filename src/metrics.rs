use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub struct Metrics {
    pub evals_total: AtomicU64,
    pub evals_passed: AtomicU64,
    pub evals_failed: AtomicU64,
    pub evals_cancelled: AtomicU64,
    pub evals_active: AtomicU64,
    pub evals_duration_sum_ms: AtomicU64,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            evals_total: AtomicU64::new(0),
            evals_passed: AtomicU64::new(0),
            evals_failed: AtomicU64::new(0),
            evals_cancelled: AtomicU64::new(0),
            evals_active: AtomicU64::new(0),
            evals_duration_sum_ms: AtomicU64::new(0),
        })
    }

    pub fn start_eval(&self) {
        self.evals_total.fetch_add(1, Ordering::Relaxed);
        self.evals_active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn finish_eval(&self, passed: Option<bool>, duration_ms: u64) {
        self.evals_active.fetch_sub(1, Ordering::Relaxed);
        self.evals_duration_sum_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        match passed {
            Some(true) => {
                self.evals_passed.fetch_add(1, Ordering::Relaxed);
            }
            Some(false) => {
                self.evals_failed.fetch_add(1, Ordering::Relaxed);
            }
            None => {
                self.evals_failed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    #[allow(dead_code)]
    pub fn cancel_eval(&self) {
        self.evals_active.fetch_sub(1, Ordering::Relaxed);
        self.evals_cancelled.fetch_add(1, Ordering::Relaxed);
    }

    pub fn render_prometheus(&self) -> String {
        let total = self.evals_total.load(Ordering::Relaxed);
        let passed = self.evals_passed.load(Ordering::Relaxed);
        let failed = self.evals_failed.load(Ordering::Relaxed);
        let cancelled = self.evals_cancelled.load(Ordering::Relaxed);
        let active = self.evals_active.load(Ordering::Relaxed);
        let dur_sum = self.evals_duration_sum_ms.load(Ordering::Relaxed);

        format!(
            "# HELP term_executor_evaluations_total Total evaluations started.\n\
             # TYPE term_executor_evaluations_total counter\n\
             term_executor_evaluations_total {}\n\
             # HELP term_executor_evaluations_passed Total evaluations that passed.\n\
             # TYPE term_executor_evaluations_passed counter\n\
             term_executor_evaluations_passed {}\n\
             # HELP term_executor_evaluations_failed Total evaluations that failed.\n\
             # TYPE term_executor_evaluations_failed counter\n\
             term_executor_evaluations_failed {}\n\
             # HELP term_executor_evaluations_cancelled Total evaluations cancelled.\n\
             # TYPE term_executor_evaluations_cancelled counter\n\
             term_executor_evaluations_cancelled {}\n\
             # HELP term_executor_evaluations_active Currently running evaluations.\n\
             # TYPE term_executor_evaluations_active gauge\n\
             term_executor_evaluations_active {}\n\
             # HELP term_executor_evaluations_duration_ms_sum Sum of evaluation durations in ms.\n\
             # TYPE term_executor_evaluations_duration_ms_sum counter\n\
             term_executor_evaluations_duration_ms_sum {}\n",
            total, passed, failed, cancelled, active, dur_sum
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_lifecycle() {
        let m = Metrics::new();
        m.start_eval();
        assert_eq!(m.evals_active.load(Ordering::Relaxed), 1);
        assert_eq!(m.evals_total.load(Ordering::Relaxed), 1);

        m.finish_eval(Some(true), 5000);
        assert_eq!(m.evals_active.load(Ordering::Relaxed), 0);
        assert_eq!(m.evals_passed.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_metrics_cancel() {
        let m = Metrics::new();
        m.start_eval();
        m.cancel_eval();
        assert_eq!(m.evals_cancelled.load(Ordering::Relaxed), 1);
        assert_eq!(m.evals_active.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_prometheus_output() {
        let m = Metrics::new();
        m.start_eval();
        m.finish_eval(Some(false), 1234);
        let out = m.render_prometheus();
        assert!(out.contains("term_executor_evaluations_total 1"));
        assert!(out.contains("term_executor_evaluations_failed 1"));
        assert!(out.contains("term_executor_evaluations_duration_ms_sum 1234"));
    }
}
