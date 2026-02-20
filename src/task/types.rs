use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweForgeTaskFields {
    pub instance_id: String,
    pub problem_statement: String,
    pub patch: String,
    pub test_patch: String,
    #[serde(default)]
    pub hints_text: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub fail_to_pass: Option<String>,
    #[serde(default)]
    pub pass_to_pass: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swe_forge_fields_serialize_roundtrip() {
        let fields = SweForgeTaskFields {
            instance_id: "django__django-12345".to_string(),
            problem_statement: "Fix the ORM query".to_string(),
            patch: "diff --git a/f.py b/f.py".to_string(),
            test_patch: "diff --git a/t.py b/t.py".to_string(),
            hints_text: Some("Check the queryset".to_string()),
            version: Some("4.2".to_string()),
            fail_to_pass: Some("[\"tests/test_orm.py::test_query\"]".to_string()),
            pass_to_pass: Some("[\"tests/test_orm.py::test_basic\"]".to_string()),
        };
        let json = serde_json::to_string(&fields).expect("should serialize");
        let back: SweForgeTaskFields = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(back.instance_id, "django__django-12345");
        assert_eq!(back.version.as_deref(), Some("4.2"));
    }

    #[test]
    fn test_swe_forge_fields_optional_defaults() {
        let json = r#"{
            "instance_id": "test-123",
            "problem_statement": "Fix it",
            "patch": "diff",
            "test_patch": "test diff"
        }"#;
        let fields: SweForgeTaskFields = serde_json::from_str(json).expect("should deserialize");
        assert!(fields.hints_text.is_none());
        assert!(fields.version.is_none());
        assert!(fields.fail_to_pass.is_none());
        assert!(fields.pass_to_pass.is_none());
    }
}
