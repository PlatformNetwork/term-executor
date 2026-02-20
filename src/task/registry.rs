use anyhow::{Context, Result};
use std::path::Path;
use tracing::info;

use super::{extract_uploaded_archive, SweForgeTask, WorkspaceConfig};
use crate::swe_forge::types::{DatasetEntry, HuggingFaceDataset};
use crate::task::types::SweForgeTaskFields;

pub struct TaskRegistry {
    tasks: Vec<SweForgeTask>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn get_tasks(&self) -> &[SweForgeTask] {
        &self.tasks
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn clear(&mut self) {
        self.tasks.clear();
    }

    pub async fn load_from_archive(&mut self, data: &[u8], dest: &Path) -> Result<()> {
        let extracted = extract_uploaded_archive(data, dest).await?;
        info!(
            "Loaded {} tasks from archive (agent language: {})",
            extracted.tasks.len(),
            extracted.agent_language
        );
        self.tasks.extend(extracted.tasks);
        Ok(())
    }

    pub fn load_from_huggingface(&mut self, dataset: &HuggingFaceDataset) -> Result<()> {
        info!(
            "Loading {} entries from HuggingFace dataset {} (split: {})",
            dataset.entries.len(),
            dataset.dataset_id,
            dataset.split
        );

        let mut loaded = 0;
        for entry in &dataset.entries {
            let task = convert_dataset_entry_to_task(entry)
                .with_context(|| format!("Failed to convert entry {}", entry.instance_id))?;
            self.tasks.push(task);
            loaded += 1;
        }

        info!(
            "Loaded {} tasks from HuggingFace dataset {}",
            loaded, dataset.dataset_id
        );
        Ok(())
    }
}

fn build_repo_url(repo: &str) -> String {
    if repo.starts_with("http://") || repo.starts_with("https://") || repo.starts_with("git@") {
        repo.to_string()
    } else {
        format!("https://github.com/{}", repo)
    }
}

fn build_test_script(test_patch: &str, fail_to_pass: Option<&str>) -> String {
    let mut script = String::from("#!/bin/sh\nset -e\n\n");

    if !test_patch.is_empty() {
        script.push_str("# Apply test patch\n");
        script.push_str("cat <<'PATCH_EOF' | git apply --allow-empty -\n");
        script.push_str(test_patch);
        if !test_patch.ends_with('\n') {
            script.push('\n');
        }
        script.push_str("PATCH_EOF\n\n");
    }

    if let Some(fail_to_pass) = fail_to_pass {
        let tests = parse_test_list(fail_to_pass);
        if !tests.is_empty() {
            script.push_str("# Run fail-to-pass tests\n");
            for test in &tests {
                script.push_str(&format!("python -m pytest {} -x\n", test));
            }
        }
    } else if !test_patch.is_empty() {
        script.push_str("# Run test suite\n");
        script.push_str("python -m pytest -x\n");
    }

    script
}

fn parse_test_list(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if let Ok(parsed) = serde_json::from_str::<Vec<String>>(trimmed) {
        return parsed;
    }

    trimmed
        .split(',')
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn convert_dataset_entry_to_task(entry: &DatasetEntry) -> Result<SweForgeTask> {
    let repo_url = build_repo_url(&entry.repo);

    let workspace = WorkspaceConfig {
        repo: repo_url,
        version: entry.version.clone().unwrap_or_default(),
        base_commit: Some(entry.base_commit.clone()),
        install: None,
        language: Some("python".to_string()),
    };

    let test_script = build_test_script(&entry.test_patch, entry.fail_to_pass.as_deref());

    let test_scripts = vec![("run_tests.sh".to_string(), test_script)];

    let swe_forge_fields = SweForgeTaskFields {
        instance_id: entry.instance_id.clone(),
        problem_statement: entry.problem_statement.clone(),
        patch: entry.patch.clone(),
        test_patch: entry.test_patch.clone(),
        hints_text: entry.hints_text.clone(),
        version: entry.version.clone(),
        fail_to_pass: entry.fail_to_pass.clone(),
        pass_to_pass: entry.pass_to_pass.clone(),
    };

    Ok(SweForgeTask {
        id: entry.instance_id.clone(),
        workspace,
        prompt: entry.problem_statement.clone(),
        test_scripts,
        test_source_files: Vec::new(),
        swe_forge_fields: Some(swe_forge_fields),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new_is_empty() {
        let registry = TaskRegistry::new();
        assert_eq!(registry.task_count(), 0);
        assert!(registry.get_tasks().is_empty());
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = TaskRegistry::new();
        let dataset = HuggingFaceDataset {
            dataset_id: "test".to_string(),
            split: "test".to_string(),
            entries: vec![make_test_entry("test-1")],
            total_count: 1,
        };
        registry
            .load_from_huggingface(&dataset)
            .expect("should load");
        assert_eq!(registry.task_count(), 1);
        registry.clear();
        assert_eq!(registry.task_count(), 0);
    }

    #[test]
    fn test_load_from_huggingface_single_entry() {
        let mut registry = TaskRegistry::new();
        let dataset = HuggingFaceDataset {
            dataset_id: "CortexLM/swe-forge".to_string(),
            split: "test".to_string(),
            entries: vec![make_test_entry("django__django-12345")],
            total_count: 1,
        };

        registry
            .load_from_huggingface(&dataset)
            .expect("should load");
        assert_eq!(registry.task_count(), 1);

        let task = &registry.get_tasks()[0];
        assert_eq!(task.id, "django__django-12345");
        assert_eq!(task.prompt, "Fix the ORM query bug");
        assert_eq!(task.workspace.repo, "https://github.com/django/django");
        assert_eq!(task.workspace.base_commit.as_deref(), Some("abc123def456"));
        assert!(task.swe_forge_fields.is_some());

        let fields = task.swe_forge_fields.as_ref().unwrap();
        assert_eq!(fields.instance_id, "django__django-12345");
        assert_eq!(fields.patch, "diff --git a/file.py b/file.py");
    }

    #[test]
    fn test_load_from_huggingface_multiple_entries() {
        let mut registry = TaskRegistry::new();
        let dataset = HuggingFaceDataset {
            dataset_id: "CortexLM/swe-forge".to_string(),
            split: "test".to_string(),
            entries: vec![
                make_test_entry("django__django-1"),
                make_test_entry("django__django-2"),
                make_test_entry("django__django-3"),
            ],
            total_count: 3,
        };

        registry
            .load_from_huggingface(&dataset)
            .expect("should load");
        assert_eq!(registry.task_count(), 3);
    }

    #[test]
    fn test_load_from_huggingface_accumulates() {
        let mut registry = TaskRegistry::new();
        let dataset1 = HuggingFaceDataset {
            dataset_id: "test".to_string(),
            split: "test".to_string(),
            entries: vec![make_test_entry("task-1")],
            total_count: 1,
        };
        let dataset2 = HuggingFaceDataset {
            dataset_id: "test".to_string(),
            split: "test".to_string(),
            entries: vec![make_test_entry("task-2")],
            total_count: 1,
        };

        registry
            .load_from_huggingface(&dataset1)
            .expect("should load");
        registry
            .load_from_huggingface(&dataset2)
            .expect("should load");
        assert_eq!(registry.task_count(), 2);
    }

    #[test]
    fn test_build_repo_url_plain() {
        assert_eq!(
            build_repo_url("django/django"),
            "https://github.com/django/django"
        );
    }

    #[test]
    fn test_build_repo_url_already_full() {
        let url = "https://github.com/psf/requests";
        assert_eq!(build_repo_url(url), url);
    }

    #[test]
    fn test_build_repo_url_git_ssh() {
        let url = "git@github.com:django/django.git";
        assert_eq!(build_repo_url(url), url);
    }

    #[test]
    fn test_parse_test_list_json() {
        let tests = parse_test_list(r#"["test_a.py::test_1", "test_b.py::test_2"]"#);
        assert_eq!(tests, vec!["test_a.py::test_1", "test_b.py::test_2"]);
    }

    #[test]
    fn test_parse_test_list_csv() {
        let tests = parse_test_list("test_a.py, test_b.py");
        assert_eq!(tests, vec!["test_a.py", "test_b.py"]);
    }

    #[test]
    fn test_parse_test_list_empty() {
        assert!(parse_test_list("").is_empty());
        assert!(parse_test_list("  ").is_empty());
    }

    #[test]
    fn test_build_test_script_with_patch() {
        let script = build_test_script("diff --git a/t.py b/t.py", None);
        assert!(script.contains("git apply"));
        assert!(script.contains("diff --git a/t.py b/t.py"));
        assert!(script.contains("python -m pytest -x"));
    }

    #[test]
    fn test_build_test_script_with_fail_to_pass() {
        let script = build_test_script(
            "diff --git a/t.py b/t.py",
            Some(r#"["tests/test_orm.py::test_query"]"#),
        );
        assert!(script.contains("git apply"));
        assert!(script.contains("python -m pytest tests/test_orm.py::test_query -x"));
    }

    #[test]
    fn test_build_test_script_empty_patch() {
        let script = build_test_script("", None);
        assert!(!script.contains("git apply"));
    }

    #[test]
    fn test_convert_dataset_entry() {
        let entry = make_test_entry("psf__requests-5678");
        let task = convert_dataset_entry_to_task(&entry).expect("should convert");
        assert_eq!(task.id, "psf__requests-5678");
        assert!(!task.test_scripts.is_empty());
        assert_eq!(task.test_scripts[0].0, "run_tests.sh");
    }

    fn make_test_entry(instance_id: &str) -> DatasetEntry {
        DatasetEntry {
            repo: "django/django".to_string(),
            instance_id: instance_id.to_string(),
            base_commit: "abc123def456".to_string(),
            patch: "diff --git a/file.py b/file.py".to_string(),
            test_patch: "diff --git a/test.py b/test.py".to_string(),
            problem_statement: "Fix the ORM query bug".to_string(),
            hints_text: None,
            created_at: None,
            version: Some("4.2".to_string()),
            fail_to_pass: Some(r#"["tests/test_orm.py::test_query"]"#.to_string()),
            pass_to_pass: None,
            environment_setup_commit: None,
        }
    }
}
