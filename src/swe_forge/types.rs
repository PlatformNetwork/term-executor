use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetEntry {
    pub repo: String,
    pub instance_id: String,
    pub base_commit: String,
    pub patch: String,
    pub test_patch: String,
    pub problem_statement: String,
    #[serde(default)]
    pub hints_text: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub fail_to_pass: Option<String>,
    #[serde(default)]
    pub pass_to_pass: Option<String>,
    #[serde(default)]
    pub environment_setup_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuggingFaceDataset {
    pub dataset_id: String,
    pub split: String,
    pub entries: Vec<DatasetEntry>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    pub dataset_id: String,
    #[serde(default = "default_split")]
    pub split: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_split() -> String {
    "test".to_string()
}

fn default_limit() -> usize {
    100
}

impl Default for DatasetConfig {
    fn default() -> Self {
        Self {
            dataset_id: "CortexLM/swe-forge".to_string(),
            split: default_split(),
            limit: default_limit(),
            offset: 0,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct HfRowsResponse {
    pub rows: Vec<HfRowWrapper>,
    #[serde(default)]
    pub num_rows_total: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HfRowWrapper {
    pub row: DatasetEntry,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataset_config_default() {
        let config = DatasetConfig::default();
        assert_eq!(config.dataset_id, "CortexLM/swe-forge");
        assert_eq!(config.split, "test");
        assert_eq!(config.limit, 100);
        assert_eq!(config.offset, 0);
    }

    #[test]
    fn test_dataset_entry_deserialize() {
        let json = r#"{
            "repo": "psf/requests",
            "instance_id": "psf__requests-1234",
            "base_commit": "abc123def456",
            "patch": "diff --git a/file.py b/file.py",
            "test_patch": "diff --git a/test_file.py b/test_file.py",
            "problem_statement": "Fix the bug in requests library"
        }"#;
        let entry: DatasetEntry = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(entry.repo, "psf/requests");
        assert_eq!(entry.instance_id, "psf__requests-1234");
        assert!(entry.hints_text.is_none());
        assert!(entry.version.is_none());
    }

    #[test]
    fn test_dataset_entry_deserialize_with_optional_fields() {
        let json = r#"{
            "repo": "psf/requests",
            "instance_id": "psf__requests-1234",
            "base_commit": "abc123def456",
            "patch": "diff --git a/file.py b/file.py",
            "test_patch": "diff --git a/test_file.py b/test_file.py",
            "problem_statement": "Fix the bug",
            "hints_text": "Check the encoding",
            "version": "2.31.0",
            "fail_to_pass": "[\"test_requests.py::test_encoding\"]",
            "pass_to_pass": "[\"test_requests.py::test_basic\"]"
        }"#;
        let entry: DatasetEntry = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(entry.hints_text.as_deref(), Some("Check the encoding"));
        assert_eq!(entry.version.as_deref(), Some("2.31.0"));
        assert!(entry.fail_to_pass.is_some());
    }

    #[test]
    fn test_huggingface_dataset_serialize_roundtrip() {
        let dataset = HuggingFaceDataset {
            dataset_id: "CortexLM/swe-forge".to_string(),
            split: "test".to_string(),
            entries: vec![],
            total_count: 0,
        };
        let json = serde_json::to_string(&dataset).expect("should serialize");
        let back: HuggingFaceDataset = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(back.dataset_id, "CortexLM/swe-forge");
        assert_eq!(back.total_count, 0);
    }
}
