use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::swe_forge::types::DatasetConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskSource {
    Local { path: PathBuf },
    HuggingFace { config: DatasetConfig },
}

impl Default for TaskSource {
    fn default() -> Self {
        Self::Local {
            path: PathBuf::from("tasks"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskConfig {
    pub source: TaskSource,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_source_default_is_local() {
        let source = TaskSource::default();
        match source {
            TaskSource::Local { path } => assert_eq!(path, PathBuf::from("tasks")),
            _ => panic!("default should be Local"),
        }
    }

    #[test]
    fn test_task_config_default() {
        let config = TaskConfig::default();
        match config.source {
            TaskSource::Local { path } => assert_eq!(path, PathBuf::from("tasks")),
            _ => panic!("default source should be Local"),
        }
    }

    #[test]
    fn test_task_source_huggingface_serialize() {
        let source = TaskSource::HuggingFace {
            config: DatasetConfig::default(),
        };
        let json = serde_json::to_string(&source).expect("should serialize");
        assert!(json.contains("hugging_face"));
        assert!(json.contains("CortexLM/swe-forge"));
    }
}
