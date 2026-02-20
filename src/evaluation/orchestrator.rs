use platform_challenge_sdk::error::ChallengeError;
use platform_challenge_sdk::server::{EvaluationRequest, EvaluationResponse, ServerChallenge};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};

pub struct Orchestrator<C: ServerChallenge> {
    challenge: Arc<C>,
    max_concurrent: usize,
    timeout_secs: u64,
}

impl<C: ServerChallenge + 'static> Orchestrator<C> {
    pub fn new(challenge: C) -> Self {
        Self {
            challenge: Arc::new(challenge),
            max_concurrent: 4,
            timeout_secs: 600,
        }
    }

    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    pub fn challenge_id(&self) -> &str {
        self.challenge.challenge_id()
    }

    pub fn challenge_name(&self) -> &str {
        self.challenge.name()
    }

    pub fn challenge_version(&self) -> &str {
        self.challenge.version()
    }

    pub async fn evaluate(
        &self,
        request: EvaluationRequest,
    ) -> Result<EvaluationResponse, ChallengeError> {
        let request_id = request.request_id.clone();
        let start = Instant::now();

        info!(
            challenge_id = %self.challenge.challenge_id(),
            request_id = %request_id,
            submission_id = %request.submission_id,
            participant_id = %request.participant_id,
            "Starting evaluation"
        );

        let deadline = std::time::Duration::from_secs(self.timeout_secs);
        let challenge = Arc::clone(&self.challenge);

        let result = tokio::time::timeout(deadline, challenge.evaluate(request)).await;

        let elapsed_ms = start.elapsed().as_millis() as i64;

        match result {
            Ok(Ok(mut response)) => {
                response.execution_time_ms = elapsed_ms;
                info!(
                    challenge_id = %self.challenge.challenge_id(),
                    request_id = %request_id,
                    score = %response.score,
                    execution_time_ms = %elapsed_ms,
                    "Evaluation completed"
                );
                Ok(response)
            }
            Ok(Err(e)) => {
                error!(
                    challenge_id = %self.challenge.challenge_id(),
                    request_id = %request_id,
                    error = %e,
                    execution_time_ms = %elapsed_ms,
                    "Evaluation failed"
                );
                Err(e)
            }
            Err(_) => {
                warn!(
                    challenge_id = %self.challenge.challenge_id(),
                    request_id = %request_id,
                    timeout_secs = %self.timeout_secs,
                    "Evaluation timed out"
                );
                Err(ChallengeError::Timeout(format!(
                    "Evaluation {} timed out after {}s",
                    request_id, self.timeout_secs
                )))
            }
        }
    }

    pub async fn evaluate_batch(
        &self,
        requests: Vec<EvaluationRequest>,
    ) -> Vec<Result<EvaluationResponse, ChallengeError>> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_concurrent));
        let mut handles = Vec::with_capacity(requests.len());

        for request in requests {
            let challenge = Arc::clone(&self.challenge);
            let sem = Arc::clone(&semaphore);
            let timeout_secs = self.timeout_secs;
            let _challenge_id = self.challenge.challenge_id().to_string();

            let handle = tokio::spawn(async move {
                let _permit = sem
                    .acquire()
                    .await
                    .map_err(|_| ChallengeError::Internal("Semaphore closed".to_string()))?;

                let request_id = request.request_id.clone();
                let deadline = std::time::Duration::from_secs(timeout_secs);
                let start = Instant::now();

                let result = tokio::time::timeout(deadline, challenge.evaluate(request)).await;
                let elapsed_ms = start.elapsed().as_millis() as i64;

                match result {
                    Ok(Ok(mut response)) => {
                        response.execution_time_ms = elapsed_ms;
                        Ok(response)
                    }
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(ChallengeError::Timeout(format!(
                        "Evaluation {} timed out after {}s",
                        request_id, timeout_secs
                    ))),
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(ChallengeError::Internal(format!(
                    "Task panicked: {}",
                    e
                )))),
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    struct MockChallenge {
        fail: bool,
        delay_ms: u64,
    }

    impl MockChallenge {
        fn passing() -> Self {
            Self {
                fail: false,
                delay_ms: 0,
            }
        }

        fn failing() -> Self {
            Self {
                fail: true,
                delay_ms: 0,
            }
        }

        fn slow(delay_ms: u64) -> Self {
            Self {
                fail: false,
                delay_ms,
            }
        }
    }

    #[async_trait]
    impl ServerChallenge for MockChallenge {
        fn challenge_id(&self) -> &str {
            "mock-challenge"
        }
        fn name(&self) -> &str {
            "Mock Challenge"
        }
        fn version(&self) -> &str {
            "1.0.0"
        }

        async fn evaluate(
            &self,
            request: EvaluationRequest,
        ) -> Result<EvaluationResponse, ChallengeError> {
            if self.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
            }
            if self.fail {
                return Err(ChallengeError::Evaluation("Mock failure".to_string()));
            }
            Ok(EvaluationResponse::success(
                &request.request_id,
                0.9,
                json!({"mock": true}),
            ))
        }
    }

    fn test_request(id: &str) -> EvaluationRequest {
        EvaluationRequest {
            request_id: id.to_string(),
            submission_id: "sub-1".to_string(),
            participant_id: "participant-1".to_string(),
            data: json!({}),
            metadata: None,
            epoch: 1,
            deadline: None,
        }
    }

    #[test]
    fn test_orchestrator_new() {
        let orch = Orchestrator::new(MockChallenge::passing());
        assert_eq!(orch.challenge_id(), "mock-challenge");
        assert_eq!(orch.challenge_name(), "Mock Challenge");
        assert_eq!(orch.challenge_version(), "1.0.0");
        assert_eq!(orch.max_concurrent, 4);
        assert_eq!(orch.timeout_secs, 600);
    }

    #[test]
    fn test_orchestrator_with_config() {
        let orch = Orchestrator::new(MockChallenge::passing())
            .with_max_concurrent(8)
            .with_timeout(300);
        assert_eq!(orch.max_concurrent, 8);
        assert_eq!(orch.timeout_secs, 300);
    }

    #[tokio::test]
    async fn test_evaluate_success() {
        let orch = Orchestrator::new(MockChallenge::passing());
        let req = test_request("req-1");

        let result = orch.evaluate(req).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.success);
        assert_eq!(response.score, 0.9);
        assert_eq!(response.request_id, "req-1");
        assert!(response.execution_time_ms >= 0);
    }

    #[tokio::test]
    async fn test_evaluate_failure() {
        let orch = Orchestrator::new(MockChallenge::failing());
        let req = test_request("req-fail");

        let result = orch.evaluate(req).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ChallengeError::Evaluation(_)));
    }

    #[tokio::test]
    async fn test_evaluate_timeout() {
        let orch = Orchestrator::new(MockChallenge::slow(5000)).with_timeout(1);
        let req = test_request("req-timeout");

        let result = orch.evaluate(req).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ChallengeError::Timeout(_)));
    }

    #[tokio::test]
    async fn test_evaluate_batch_all_pass() {
        let orch = Orchestrator::new(MockChallenge::passing());
        let requests = vec![
            test_request("b-1"),
            test_request("b-2"),
            test_request("b-3"),
        ];

        let results = orch.evaluate_batch(requests).await;

        assert_eq!(results.len(), 3);
        for result in &results {
            assert!(result.is_ok());
            assert!(result.as_ref().unwrap().success);
        }
    }

    #[tokio::test]
    async fn test_evaluate_batch_with_failure() {
        let orch = Orchestrator::new(MockChallenge::failing());
        let requests = vec![test_request("f-1"), test_request("f-2")];

        let results = orch.evaluate_batch(requests).await;

        assert_eq!(results.len(), 2);
        for result in &results {
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_evaluate_sets_execution_time() {
        let orch = Orchestrator::new(MockChallenge::slow(50));
        let req = test_request("req-time");

        let result = orch.evaluate(req).await.unwrap();

        assert!(result.execution_time_ms >= 50);
    }
}
