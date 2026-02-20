use platform_challenge_sdk::server::{EvaluationRequest, EvaluationResponse};
use platform_challenge_sdk::types::AgentInfo;
use serde_json::Value;
use tracing::{info, warn};

pub struct Evaluator {
    default_epoch: u64,
    default_deadline: Option<i64>,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            default_epoch: 0,
            default_deadline: None,
        }
    }

    pub fn with_epoch(mut self, epoch: u64) -> Self {
        self.default_epoch = epoch;
        self
    }

    pub fn with_deadline(mut self, deadline: i64) -> Self {
        self.default_deadline = Some(deadline);
        self
    }

    pub fn build_request(
        &self,
        request_id: &str,
        submission_id: &str,
        agent: &AgentInfo,
        data: Value,
    ) -> EvaluationRequest {
        let metadata = serde_json::json!({
            "agent_hash": agent.hash,
            "agent_name": agent.name,
            "agent_version": agent.version,
        });

        EvaluationRequest {
            request_id: request_id.to_string(),
            submission_id: submission_id.to_string(),
            participant_id: agent
                .owner
                .as_ref()
                .map(|h| h.to_string())
                .unwrap_or_default(),
            data,
            metadata: Some(metadata),
            epoch: self.default_epoch,
            deadline: self.default_deadline,
        }
    }

    pub fn process_response(&self, response: &EvaluationResponse) -> EvaluationOutcome {
        if response.success {
            info!(
                request_id = %response.request_id,
                score = %response.score,
                execution_time_ms = %response.execution_time_ms,
                "Evaluation succeeded"
            );
        } else {
            warn!(
                request_id = %response.request_id,
                error = ?response.error,
                "Evaluation failed"
            );
        }

        EvaluationOutcome {
            request_id: response.request_id.clone(),
            success: response.success,
            score: response.score,
            execution_time_ms: response.execution_time_ms,
            error: response.error.clone(),
            results: response.results.clone(),
        }
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EvaluationOutcome {
    pub request_id: String,
    pub success: bool,
    pub score: f64,
    pub execution_time_ms: i64,
    pub error: Option<String>,
    pub results: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_agent() -> AgentInfo {
        AgentInfo::new("test-hash-abc".to_string())
    }

    #[test]
    fn test_evaluator_new() {
        let evaluator = Evaluator::new();
        assert_eq!(evaluator.default_epoch, 0);
        assert!(evaluator.default_deadline.is_none());
    }

    #[test]
    fn test_evaluator_with_epoch() {
        let evaluator = Evaluator::new().with_epoch(42);
        assert_eq!(evaluator.default_epoch, 42);
    }

    #[test]
    fn test_evaluator_with_deadline() {
        let evaluator = Evaluator::new().with_deadline(1700000000);
        assert_eq!(evaluator.default_deadline, Some(1700000000));
    }

    #[test]
    fn test_build_request() {
        let evaluator = Evaluator::new().with_epoch(5);
        let agent = test_agent();
        let data = json!({"code": "fn main() {}"});

        let req = evaluator.build_request("req-1", "sub-1", &agent, data.clone());

        assert_eq!(req.request_id, "req-1");
        assert_eq!(req.submission_id, "sub-1");
        assert_eq!(req.epoch, 5);
        assert_eq!(req.data, data);
        assert!(req.metadata.is_some());
        let meta = req.metadata.unwrap();
        assert_eq!(meta["agent_hash"], "test-hash-abc");
    }

    #[test]
    fn test_build_request_with_named_agent() {
        let evaluator = Evaluator::new();
        let mut agent = AgentInfo::new("hash-xyz".to_string());
        agent.name = Some("My Agent".to_string());
        agent.version = Some("2.0.0".to_string());

        let req = evaluator.build_request("req-2", "sub-2", &agent, json!({}));

        let meta = req.metadata.unwrap();
        assert_eq!(meta["agent_name"], "My Agent");
        assert_eq!(meta["agent_version"], "2.0.0");
    }

    #[test]
    fn test_process_response_success() {
        let evaluator = Evaluator::new();
        let response = EvaluationResponse::success("req-1", 0.85, json!({"passed": 17}));

        let outcome = evaluator.process_response(&response);

        assert!(outcome.success);
        assert_eq!(outcome.score, 0.85);
        assert!(outcome.error.is_none());
        assert_eq!(outcome.request_id, "req-1");
    }

    #[test]
    fn test_process_response_failure() {
        let evaluator = Evaluator::new();
        let response = EvaluationResponse::error("req-2", "Timeout exceeded");

        let outcome = evaluator.process_response(&response);

        assert!(!outcome.success);
        assert_eq!(outcome.score, 0.0);
        assert_eq!(outcome.error, Some("Timeout exceeded".to_string()));
    }

    #[test]
    fn test_process_response_with_execution_time() {
        let evaluator = Evaluator::new();
        let response = EvaluationResponse::success("req-3", 0.95, json!({})).with_time(1500);

        let outcome = evaluator.process_response(&response);

        assert_eq!(outcome.execution_time_ms, 1500);
    }

    #[test]
    fn test_evaluator_default() {
        let evaluator = Evaluator::default();
        assert_eq!(evaluator.default_epoch, 0);
        assert!(evaluator.default_deadline.is_none());
    }
}
