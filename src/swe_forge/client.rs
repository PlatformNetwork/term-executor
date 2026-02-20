use anyhow::{Context, Result};
use tracing::{debug, info};

use super::types::{DatasetConfig, DatasetEntry, HfRowsResponse, HuggingFaceDataset};

const HF_DATASET_VIEWER_BASE: &str = "https://datasets-server.huggingface.co/rows";
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
