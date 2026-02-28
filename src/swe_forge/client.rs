use anyhow::{Context, Result};
use std::path::Path;
use tracing::{debug, info, warn};

use super::types::{DatasetConfig, DatasetEntry, HfRowsResponse, HuggingFaceDataset};

const HF_DATASET_VIEWER_BASE: &str = "https://datasets-server.huggingface.co/rows";
const HF_REPO_BASE: &str = "https://huggingface.co";
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_PAGE_SIZE: usize = 100;

pub struct HuggingFaceClient {
    client: reqwest::Client,
}

impl HuggingFaceClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .context("Failed to build HTTP client for HuggingFace")?;
        Ok(Self { client })
    }

    pub async fn fetch_dataset(&self, config: &DatasetConfig) -> Result<HuggingFaceDataset> {
        info!(
            "Fetching HuggingFace dataset: {} (split={}, offset={}, limit={})",
            config.dataset_id, config.split, config.offset, config.limit
        );

        let mut all_entries = Vec::new();
        let mut offset = config.offset;
        let mut total_count = 0;
        let remaining = config.limit;

        while all_entries.len() < remaining {
            let page_size = MAX_PAGE_SIZE.min(remaining - all_entries.len());

            let response = self
                .fetch_page(&config.dataset_id, &config.split, offset, page_size)
                .await?;

            if let Some(total) = response.num_rows_total {
                total_count = total;
            }

            let row_count = response.rows.len();
            if row_count == 0 {
                break;
            }

            for wrapper in response.rows {
                all_entries.push(wrapper.row);
            }

            offset += row_count;

            if row_count < page_size {
                break;
            }
        }

        info!(
            "Fetched {} entries from {} (total available: {})",
            all_entries.len(),
            config.dataset_id,
            total_count
        );

        Ok(HuggingFaceDataset {
            dataset_id: config.dataset_id.clone(),
            split: config.split.clone(),
            entries: all_entries,
            total_count,
        })
    }

    pub async fn fetch_entry(
        &self,
        dataset_id: &str,
        split: &str,
        index: usize,
    ) -> Result<DatasetEntry> {
        debug!(
            "Fetching single entry from {} at index {}",
            dataset_id, index
        );

        let response = self.fetch_page(dataset_id, split, index, 1).await?;

        response
            .rows
            .into_iter()
            .next()
            .map(|w| w.row)
            .context(format!(
                "No entry found at index {} in dataset {}",
                index, dataset_id
            ))
    }

    /// Download all task files for a given instance_id from the HF repo into a local directory.
    /// The directory will have workspace.yaml, prompt.md, tests/*.sh, etc.
    pub async fn download_task_files(
        &self,
        dataset_id: &str,
        instance_id: &str,
        dest_dir: &Path,
    ) -> Result<()> {
        let tree_url = format!(
            "{}/api/datasets/{}/tree/main/tasks/{}",
            HF_REPO_BASE, dataset_id, instance_id
        );
        info!("Listing HF task files: {}", tree_url);

        // List all files (including subdirectories)
        let files = self.list_tree_recursive(dataset_id, instance_id).await?;

        if files.is_empty() {
            anyhow::bail!(
                "No files found for task {} in dataset {}",
                instance_id,
                dataset_id
            );
        }

        tokio::fs::create_dir_all(dest_dir).await?;

        for file_path in &files {
            let relative = file_path
                .strip_prefix(&format!("tasks/{}/", instance_id))
                .unwrap_or(file_path);
            let local_path = dest_dir.join(relative);

            if let Some(parent) = local_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let download_url = format!(
                "{}/datasets/{}/resolve/main/{}",
                HF_REPO_BASE, dataset_id, file_path
            );
            debug!("Downloading {} -> {}", download_url, local_path.display());

            let resp = self
                .client
                .get(&download_url)
                .send()
                .await
                .with_context(|| format!("Failed to download {}", download_url))?;

            if !resp.status().is_success() {
                warn!("Failed to download {}: HTTP {}", file_path, resp.status());
                continue;
            }

            let bytes = resp.bytes().await?;
            tokio::fs::write(&local_path, &bytes).await?;
        }

        info!(
            "Downloaded {} files for task {} to {}",
            files.len(),
            instance_id,
            dest_dir.display()
        );
        Ok(())
    }

    async fn list_tree_recursive(
        &self,
        dataset_id: &str,
        instance_id: &str,
    ) -> Result<Vec<String>> {
        let mut all_files = Vec::new();
        let mut dirs_to_visit = vec![format!("tasks/{}", instance_id)];

        while let Some(dir_path) = dirs_to_visit.pop() {
            let url = format!(
                "{}/api/datasets/{}/tree/main/{}",
                HF_REPO_BASE, dataset_id, dir_path
            );
            let resp = self
                .client
                .get(&url)
                .send()
                .await
                .with_context(|| format!("Failed to list {}", url))?;

            if !resp.status().is_success() {
                warn!(
                    "Failed to list directory {}: HTTP {}",
                    dir_path,
                    resp.status()
                );
                continue;
            }

            let entries: Vec<serde_json::Value> = resp.json().await?;
            for entry in entries {
                let entry_type = entry["type"].as_str().unwrap_or("");
                let entry_path = entry["path"].as_str().unwrap_or("");
                if entry_path.is_empty() {
                    continue;
                }
                match entry_type {
                    "file" => all_files.push(entry_path.to_string()),
                    "directory" => dirs_to_visit.push(entry_path.to_string()),
                    _ => {}
                }
            }
        }

        Ok(all_files)
    }

    async fn fetch_page(
        &self,
        dataset_id: &str,
        split: &str,
        offset: usize,
        length: usize,
    ) -> Result<HfRowsResponse> {
        let url = format!(
            "{}?dataset={}&config=default&split={}&offset={}&length={}",
            HF_DATASET_VIEWER_BASE, dataset_id, split, offset, length
        );

        debug!("Requesting HuggingFace API: {}", url);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to HuggingFace dataset viewer")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "HuggingFace API returned HTTP {}: {}",
                status.as_u16(),
                &body[..body.len().min(500)]
            );
        }

        let response: HfRowsResponse = resp
            .json()
            .await
            .context("Failed to parse HuggingFace API response")?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = HuggingFaceClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_hf_rows_response_deserialize() {
        let json = r#"{
            "rows": [
                {
                    "row": {
                        "repo": "psf/requests",
                        "instance_id": "psf__requests-1234",
                        "base_commit": "abc123",
                        "patch": "diff --git a/f.py b/f.py",
                        "test_patch": "diff --git a/t.py b/t.py",
                        "problem_statement": "Fix bug"
                    }
                }
            ],
            "num_rows_total": 2294
        }"#;
        let resp: HfRowsResponse = serde_json::from_str(json).expect("should parse");
        assert_eq!(resp.rows.len(), 1);
        assert_eq!(resp.num_rows_total, Some(2294));
        assert_eq!(resp.rows[0].row.repo, "psf/requests");
    }

    #[test]
    fn test_hf_rows_response_empty() {
        let json = r#"{"rows": []}"#;
        let resp: HfRowsResponse = serde_json::from_str(json).expect("should parse");
        assert!(resp.rows.is_empty());
        assert!(resp.num_rows_total.is_none());
    }
}
