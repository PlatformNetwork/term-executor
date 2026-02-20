use platform_challenge_sdk::types::{ChallengeId, WeightAssignment};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub challenge_id: ChallengeId,
    pub stage_weights: HashMap<String, f64>,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

impl PipelineConfig {
    pub fn new(challenge_id: ChallengeId) -> Self {
        Self {
            challenge_id,
            stage_weights: HashMap::new(),
            timeout_secs: 600,
            max_retries: 0,
        }
    }

    pub fn with_stage_weight(mut self, stage: impl Into<String>, weight: f64) -> Self {
        self.stage_weights
            .insert(stage.into(), weight.clamp(0.0, 1.0));
        self
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageResult {
    pub stage_name: String,
    pub score: f64,
    pub weight: f64,
    pub execution_time_ms: u64,
    pub metadata: serde_json::Value,
}

pub struct EvaluationPipeline {
    config: PipelineConfig,
    stages: Vec<StageResult>,
}

impl EvaluationPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        info!(
            challenge_id = %config.challenge_id,
            stages = %config.stage_weights.len(),
            "Creating evaluation pipeline"
        );
        Self {
            config,
            stages: Vec::new(),
        }
    }

    pub fn challenge_id(&self) -> &ChallengeId {
        &self.config.challenge_id
    }

    pub fn record_stage(
        &mut self,
        stage_name: impl Into<String>,
        score: f64,
        execution_time_ms: u64,
        metadata: serde_json::Value,
    ) {
        let name = stage_name.into();
        let weight = self.config.stage_weights.get(&name).copied().unwrap_or(1.0);

        debug!(
            stage = %name,
            score = %score,
            weight = %weight,
            execution_time_ms = %execution_time_ms,
            "Recording pipeline stage result"
        );

        self.stages.push(StageResult {
            stage_name: name,
            score: score.clamp(0.0, 1.0),
            weight,
            execution_time_ms,
            metadata,
        });
    }

    pub fn weighted_score(&self) -> f64 {
        let total_weight: f64 = self.stages.iter().map(|s| s.weight).sum();
        if total_weight <= 0.0 {
            return 0.0;
        }

        let weighted_sum: f64 = self.stages.iter().map(|s| s.score * s.weight).sum();
        (weighted_sum / total_weight).clamp(0.0, 1.0)
    }

    pub fn total_execution_time_ms(&self) -> u64 {
        self.stages.iter().map(|s| s.execution_time_ms).sum()
    }

    pub fn stage_results(&self) -> &[StageResult] {
        &self.stages
    }

    pub fn to_weight_assignments(&self, participant_id: &str) -> Vec<WeightAssignment> {
        let score = self.weighted_score();
        if score > 0.0 {
            vec![WeightAssignment::new(participant_id.to_string(), score)]
        } else {
            vec![]
        }
    }

    pub fn is_complete(&self) -> bool {
        if self.config.stage_weights.is_empty() {
            return !self.stages.is_empty();
        }
        self.config
            .stage_weights
            .keys()
            .all(|name| self.stages.iter().any(|s| &s.stage_name == name))
    }

    pub fn reset(&mut self) {
        self.stages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_challenge_id() -> ChallengeId {
        ChallengeId::from_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

    #[test]
    fn test_pipeline_config_new() {
        let id = test_challenge_id();
        let config = PipelineConfig::new(id);
        assert!(config.stage_weights.is_empty());
        assert_eq!(config.timeout_secs, 600);
        assert_eq!(config.max_retries, 0);
    }

    #[test]
    fn test_pipeline_config_with_stages() {
        let config = PipelineConfig::new(test_challenge_id())
            .with_stage_weight("compilation", 0.3)
            .with_stage_weight("tests", 0.7);

        assert_eq!(config.stage_weights.len(), 2);
        assert_eq!(config.stage_weights["compilation"], 0.3);
        assert_eq!(config.stage_weights["tests"], 0.7);
    }

    #[test]
    fn test_pipeline_config_weight_clamping() {
        let config = PipelineConfig::new(test_challenge_id())
            .with_stage_weight("over", 1.5)
            .with_stage_weight("under", -0.5);

        assert_eq!(config.stage_weights["over"], 1.0);
        assert_eq!(config.stage_weights["under"], 0.0);
    }

    #[test]
    fn test_pipeline_config_with_timeout() {
        let config = PipelineConfig::new(test_challenge_id()).with_timeout(300);
        assert_eq!(config.timeout_secs, 300);
    }

    #[test]
    fn test_pipeline_config_with_max_retries() {
        let config = PipelineConfig::new(test_challenge_id()).with_max_retries(3);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_pipeline_new() {
        let config = PipelineConfig::new(test_challenge_id());
        let pipeline = EvaluationPipeline::new(config);
        assert_eq!(
            pipeline.challenge_id().to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert!(pipeline.stage_results().is_empty());
    }

    #[test]
    fn test_record_stage() {
        let config = PipelineConfig::new(test_challenge_id()).with_stage_weight("tests", 1.0);
        let mut pipeline = EvaluationPipeline::new(config);

        pipeline.record_stage("tests", 0.85, 1500, json!({"passed": 17, "total": 20}));

        assert_eq!(pipeline.stage_results().len(), 1);
        assert_eq!(pipeline.stage_results()[0].stage_name, "tests");
        assert_eq!(pipeline.stage_results()[0].score, 0.85);
        assert_eq!(pipeline.stage_results()[0].weight, 1.0);
    }

    #[test]
    fn test_record_stage_unknown_weight() {
        let config = PipelineConfig::new(test_challenge_id());
        let mut pipeline = EvaluationPipeline::new(config);

        pipeline.record_stage("unknown", 0.5, 100, json!({}));

        assert_eq!(pipeline.stage_results()[0].weight, 1.0);
    }

    #[test]
    fn test_weighted_score_single() {
        let config = PipelineConfig::new(test_challenge_id()).with_stage_weight("tests", 1.0);
        let mut pipeline = EvaluationPipeline::new(config);

        pipeline.record_stage("tests", 0.8, 100, json!({}));

        assert!((pipeline.weighted_score() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_weighted_score_multiple() {
        let config = PipelineConfig::new(test_challenge_id())
            .with_stage_weight("compilation", 0.3)
            .with_stage_weight("tests", 0.7);
        let mut pipeline = EvaluationPipeline::new(config);

        pipeline.record_stage("compilation", 1.0, 50, json!({}));
        pipeline.record_stage("tests", 0.8, 200, json!({}));

        let expected = (1.0 * 0.3 + 0.8 * 0.7) / (0.3 + 0.7);
        assert!((pipeline.weighted_score() - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_weighted_score_empty() {
        let config = PipelineConfig::new(test_challenge_id());
        let pipeline = EvaluationPipeline::new(config);
        assert_eq!(pipeline.weighted_score(), 0.0);
    }

    #[test]
    fn test_total_execution_time() {
        let config = PipelineConfig::new(test_challenge_id());
        let mut pipeline = EvaluationPipeline::new(config);

        pipeline.record_stage("a", 1.0, 100, json!({}));
        pipeline.record_stage("b", 1.0, 200, json!({}));
        pipeline.record_stage("c", 1.0, 300, json!({}));

        assert_eq!(pipeline.total_execution_time_ms(), 600);
    }

    #[test]
    fn test_to_weight_assignments() {
        let config = PipelineConfig::new(test_challenge_id());
        let mut pipeline = EvaluationPipeline::new(config);

        pipeline.record_stage("tests", 0.75, 100, json!({}));

        let weights = pipeline.to_weight_assignments("miner-hotkey-123");
        assert_eq!(weights.len(), 1);
        assert_eq!(weights[0].hotkey, "miner-hotkey-123");
        assert!((weights[0].weight - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_to_weight_assignments_zero_score() {
        let config = PipelineConfig::new(test_challenge_id());
        let mut pipeline = EvaluationPipeline::new(config);

        pipeline.record_stage("tests", 0.0, 100, json!({}));

        let weights = pipeline.to_weight_assignments("miner-hotkey");
        assert!(weights.is_empty());
    }

    #[test]
    fn test_is_complete_all_stages() {
        let config = PipelineConfig::new(test_challenge_id())
            .with_stage_weight("compile", 0.3)
            .with_stage_weight("test", 0.7);
        let mut pipeline = EvaluationPipeline::new(config);

        assert!(!pipeline.is_complete());

        pipeline.record_stage("compile", 1.0, 50, json!({}));
        assert!(!pipeline.is_complete());

        pipeline.record_stage("test", 0.9, 200, json!({}));
        assert!(pipeline.is_complete());
    }

    #[test]
    fn test_is_complete_no_configured_stages() {
        let config = PipelineConfig::new(test_challenge_id());
        let mut pipeline = EvaluationPipeline::new(config);

        assert!(!pipeline.is_complete());

        pipeline.record_stage("any", 1.0, 100, json!({}));
        assert!(pipeline.is_complete());
    }

    #[test]
    fn test_reset() {
        let config = PipelineConfig::new(test_challenge_id());
        let mut pipeline = EvaluationPipeline::new(config);

        pipeline.record_stage("a", 1.0, 100, json!({}));
        assert_eq!(pipeline.stage_results().len(), 1);

        pipeline.reset();
        assert!(pipeline.stage_results().is_empty());
        assert_eq!(pipeline.weighted_score(), 0.0);
    }

    #[test]
    fn test_score_clamping() {
        let config = PipelineConfig::new(test_challenge_id());
        let mut pipeline = EvaluationPipeline::new(config);

        pipeline.record_stage("over", 1.5, 100, json!({}));
        assert_eq!(pipeline.stage_results()[0].score, 1.0);

        pipeline.record_stage("under", -0.5, 100, json!({}));
        assert_eq!(pipeline.stage_results()[1].score, 0.0);
    }

    #[test]
    fn test_challenge_id_uuid_based() {
        let uuid = uuid::Uuid::new_v4();
        let id = ChallengeId::from_uuid(uuid);
        let config = PipelineConfig::new(id);
        let pipeline = EvaluationPipeline::new(config);
        assert_eq!(pipeline.challenge_id().0, uuid);
    }
}
