use platform_challenge_sdk::types::{ChallengeId, JobStatus};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{debug, info, warn};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusTransition {
    pub from: JobStatus,
    pub to: JobStatus,
    pub elapsed_ms: u64,
}

pub struct EvaluationProgress {
    challenge_id: ChallengeId,
    job_id: uuid::Uuid,
    status: JobStatus,
    started_at: Instant,
    last_transition: Instant,
    transitions: Vec<StatusTransition>,
    total_stages: usize,
    completed_stages: usize,
    current_stage_name: Option<String>,
}

impl EvaluationProgress {
    pub fn new(challenge_id: ChallengeId, job_id: uuid::Uuid) -> Self {
        let now = Instant::now();
        info!(
            challenge_id = %challenge_id,
            job_id = %job_id,
            "Starting evaluation progress tracking"
        );
        Self {
            challenge_id,
            job_id,
            status: JobStatus::Pending,
            started_at: now,
            last_transition: now,
            transitions: Vec::new(),
            total_stages: 0,
            completed_stages: 0,
            current_stage_name: None,
        }
    }

    pub fn with_total_stages(mut self, total: usize) -> Self {
        self.total_stages = total;
        self
    }

    pub fn challenge_id(&self) -> &ChallengeId {
        &self.challenge_id
    }

    pub fn job_id(&self) -> uuid::Uuid {
        self.job_id
    }

    pub fn status(&self) -> JobStatus {
        self.status
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    pub fn progress_percent(&self) -> f64 {
        if self.total_stages == 0 {
            return match self.status {
                JobStatus::Completed => 100.0,
                JobStatus::Failed | JobStatus::Timeout | JobStatus::Cancelled => 0.0,
                _ => 0.0,
            };
        }
        (self.completed_stages as f64 / self.total_stages as f64 * 100.0).clamp(0.0, 100.0)
    }

    pub fn current_stage_name(&self) -> Option<&str> {
        self.current_stage_name.as_deref()
    }

    pub fn transitions(&self) -> &[StatusTransition] {
        &self.transitions
    }

    pub fn transition_to(&mut self, new_status: JobStatus) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_transition).as_millis() as u64;

        debug!(
            challenge_id = %self.challenge_id,
            job_id = %self.job_id,
            from = ?self.status,
            to = ?new_status,
            elapsed_ms = %elapsed,
            "Status transition"
        );

        self.transitions.push(StatusTransition {
            from: self.status,
            to: new_status,
            elapsed_ms: elapsed,
        });

        self.status = new_status;
        self.last_transition = now;
    }

    pub fn start(&mut self) {
        self.transition_to(JobStatus::Running);
    }

    pub fn begin_stage(&mut self, stage_name: impl Into<String>) {
        let name = stage_name.into();
        debug!(
            challenge_id = %self.challenge_id,
            job_id = %self.job_id,
            stage = %name,
            "Beginning pipeline stage"
        );
        self.current_stage_name = Some(name);
    }

    pub fn complete_stage(&mut self) {
        if let Some(ref name) = self.current_stage_name {
            debug!(
                challenge_id = %self.challenge_id,
                job_id = %self.job_id,
                stage = %name,
                "Completed pipeline stage"
            );
        }
        self.completed_stages += 1;
        self.current_stage_name = None;
    }

    pub fn complete(&mut self) {
        self.transition_to(JobStatus::Completed);
        info!(
            challenge_id = %self.challenge_id,
            job_id = %self.job_id,
            elapsed_ms = %self.elapsed_ms(),
            stages_completed = %self.completed_stages,
            "Evaluation completed"
        );
    }

    pub fn fail(&mut self) {
        self.transition_to(JobStatus::Failed);
        warn!(
            challenge_id = %self.challenge_id,
            job_id = %self.job_id,
            elapsed_ms = %self.elapsed_ms(),
            stages_completed = %self.completed_stages,
            "Evaluation failed"
        );
    }

    pub fn timeout(&mut self) {
        self.transition_to(JobStatus::Timeout);
        warn!(
            challenge_id = %self.challenge_id,
            job_id = %self.job_id,
            elapsed_ms = %self.elapsed_ms(),
            "Evaluation timed out"
        );
    }

    pub fn cancel(&mut self) {
        self.transition_to(JobStatus::Cancelled);
        info!(
            challenge_id = %self.challenge_id,
            job_id = %self.job_id,
            elapsed_ms = %self.elapsed_ms(),
            "Evaluation cancelled"
        );
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Timeout | JobStatus::Cancelled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_challenge_id() -> ChallengeId {
        ChallengeId::from_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

    fn test_progress() -> EvaluationProgress {
        EvaluationProgress::new(test_challenge_id(), uuid::Uuid::new_v4())
    }

    #[test]
    fn test_new_progress() {
        let progress = test_progress();
        assert_eq!(progress.status(), JobStatus::Pending);
        assert!(!progress.is_terminal());
        assert!(progress.transitions().is_empty());
        assert!(progress.current_stage_name().is_none());
    }

    #[test]
    fn test_with_total_stages() {
        let progress = test_progress().with_total_stages(5);
        assert_eq!(progress.total_stages, 5);
        assert_eq!(progress.progress_percent(), 0.0);
    }

    #[test]
    fn test_start() {
        let mut progress = test_progress();
        progress.start();
        assert_eq!(progress.status(), JobStatus::Running);
        assert_eq!(progress.transitions().len(), 1);
        assert_eq!(progress.transitions()[0].from, JobStatus::Pending);
        assert_eq!(progress.transitions()[0].to, JobStatus::Running);
    }

    #[test]
    fn test_complete() {
        let mut progress = test_progress();
        progress.start();
        progress.complete();
        assert_eq!(progress.status(), JobStatus::Completed);
        assert!(progress.is_terminal());
        assert_eq!(progress.transitions().len(), 2);
    }

    #[test]
    fn test_fail() {
        let mut progress = test_progress();
        progress.start();
        progress.fail();
        assert_eq!(progress.status(), JobStatus::Failed);
        assert!(progress.is_terminal());
    }

    #[test]
    fn test_timeout() {
        let mut progress = test_progress();
        progress.start();
        progress.timeout();
        assert_eq!(progress.status(), JobStatus::Timeout);
        assert!(progress.is_terminal());
    }

    #[test]
    fn test_cancel() {
        let mut progress = test_progress();
        progress.start();
        progress.cancel();
        assert_eq!(progress.status(), JobStatus::Cancelled);
        assert!(progress.is_terminal());
    }

    #[test]
    fn test_stage_tracking() {
        let mut progress = test_progress().with_total_stages(3);
        progress.start();

        progress.begin_stage("compilation");
        assert_eq!(progress.current_stage_name(), Some("compilation"));
        assert_eq!(progress.progress_percent(), 0.0);

        progress.complete_stage();
        assert!(progress.current_stage_name().is_none());
        assert!((progress.progress_percent() - 33.333).abs() < 0.01);

        progress.begin_stage("testing");
        progress.complete_stage();
        assert!((progress.progress_percent() - 66.666).abs() < 0.01);

        progress.begin_stage("scoring");
        progress.complete_stage();
        assert!((progress.progress_percent() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_progress_percent_no_stages() {
        let progress = test_progress();
        assert_eq!(progress.progress_percent(), 0.0);
    }

    #[test]
    fn test_progress_percent_completed_no_stages() {
        let mut progress = test_progress();
        progress.start();
        progress.complete();
        assert_eq!(progress.progress_percent(), 100.0);
    }

    #[test]
    fn test_progress_percent_failed_no_stages() {
        let mut progress = test_progress();
        progress.start();
        progress.fail();
        assert_eq!(progress.progress_percent(), 0.0);
    }

    #[test]
    fn test_elapsed_ms() {
        let progress = test_progress();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(progress.elapsed_ms() >= 10);
    }

    #[test]
    fn test_challenge_id_accessor() {
        let id = test_challenge_id();
        let progress = EvaluationProgress::new(id, uuid::Uuid::new_v4());
        assert_eq!(
            progress.challenge_id().to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_job_id_accessor() {
        let job_id = uuid::Uuid::new_v4();
        let progress = EvaluationProgress::new(test_challenge_id(), job_id);
        assert_eq!(progress.job_id(), job_id);
    }

    #[test]
    fn test_multiple_transitions() {
        let mut progress = test_progress();
        progress.start();
        progress.complete();

        let transitions = progress.transitions();
        assert_eq!(transitions.len(), 2);
        assert_eq!(transitions[0].from, JobStatus::Pending);
        assert_eq!(transitions[0].to, JobStatus::Running);
        assert_eq!(transitions[1].from, JobStatus::Running);
        assert_eq!(transitions[1].to, JobStatus::Completed);
    }

    #[test]
    fn test_is_terminal_pending() {
        let progress = test_progress();
        assert!(!progress.is_terminal());
    }

    #[test]
    fn test_is_terminal_running() {
        let mut progress = test_progress();
        progress.start();
        assert!(!progress.is_terminal());
    }
}
