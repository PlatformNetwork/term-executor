use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use super::types::*;

const DEFAULT_API_URL: &str = "https://api.basilica.ai";
const DEFAULT_TIMEOUT_SECS: u64 = 120;
const POLL_INTERVAL_SECS: u64 = 5;
const MAX_POLL_ATTEMPTS: u32 = 60;

pub struct BasilicaClient {
    client: reqwest::Client,
    base_url: String,
}

impl BasilicaClient {
    pub fn new(api_token: &str) -> Result<Self> {
        let base_url = std::env::var("BASILICA_API_URL")
            .unwrap_or_else(|_| DEFAULT_API_URL.to_string());

        let mut headers = reqwest::header::HeaderMap::new();
        let auth_value = format!("Bearer {}", api_token);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            auth_value
                .parse()
                .context("Invalid API token format")?,
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .default_headers(headers)
            .build()
            .context("Failed to build Basilica HTTP client")?;

        Ok(Self { client, base_url })
    }

    // ── Health ──

    pub async fn health(&self) -> Result<HealthResponse> {
        let url = format!("{}/health", self.base_url);
        let resp = self.client.get(&url).send().await
            .context("Basilica health check failed")?;
        self.handle_response(resp, "health").await
    }

    // ── SSH keys ──

    pub async fn register_ssh_key(&self, name: &str, public_key: &str) -> Result<SshKeyResponse> {
        let url = format!("{}/ssh-keys", self.base_url);
        let body = RegisterSshKeyRequest {
            name: name.to_string(),
            public_key: public_key.to_string(),
        };
        let resp = self.client.post(&url).json(&body).send().await
            .context("Failed to register SSH key")?;
        self.handle_response(resp, "register_ssh_key").await
    }

    pub async fn get_ssh_key(&self) -> Result<Option<SshKeyResponse>> {
        let url = format!("{}/ssh-keys", self.base_url);
        let resp = self.client.get(&url).send().await
            .context("Failed to get SSH key")?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let key: SshKeyResponse = self.handle_response(resp, "get_ssh_key").await?;
        Ok(Some(key))
    }

    pub async fn delete_ssh_key(&self) -> Result<()> {
        let url = format!("{}/ssh-keys", self.base_url);
        let resp = self.client.delete(&url).send().await
            .context("Failed to delete SSH key")?;
        self.handle_empty_response(resp, "delete_ssh_key").await
    }

    // ── Community cloud rentals (GPU) ──

    pub async fn start_rental(&self, req: &StartRentalRequest) -> Result<RentalResponse> {
        let url = format!("{}/rentals", self.base_url);
        info!("Starting Basilica rental: image={}", req.container_image);
        let resp = self.client.post(&url).json(req).send().await
            .context("Failed to start rental")?;
        self.handle_response(resp, "start_rental").await
    }

    pub async fn get_rental(&self, rental_id: &str) -> Result<RentalStatusResponse> {
        let url = format!("{}/rentals/{}", self.base_url, rental_id);
        let resp = self.client.get(&url).send().await
            .context("Failed to get rental status")?;
        self.handle_response(resp, "get_rental").await
    }

    pub async fn stop_rental(&self, rental_id: &str) -> Result<()> {
        let url = format!("{}/rentals/{}", self.base_url, rental_id);
        info!("Stopping Basilica rental: {}", rental_id);
        let resp = self.client.delete(&url).send().await
            .context("Failed to stop rental")?;
        self.handle_empty_response(resp, "stop_rental").await
    }

    pub async fn list_rentals(&self) -> Result<ListRentalsResponse> {
        let url = format!("{}/rentals", self.base_url);
        let resp = self.client.get(&url).send().await
            .context("Failed to list rentals")?;
        self.handle_response(resp, "list_rentals").await
    }

    // ── Secure cloud CPU rentals ──

    pub async fn list_cpu_offerings(&self) -> Result<ListCpuOfferingsResponse> {
        let url = format!("{}/secure-cloud/cpu-prices?available_only=true", self.base_url);
        let resp = self.client.get(&url).send().await
            .context("Failed to list CPU offerings")?;
        self.handle_response(resp, "list_cpu_offerings").await
    }

    pub async fn start_cpu_rental(&self, offering_id: &str, ssh_key_id: &str) -> Result<SecureCloudRentalResponse> {
        let url = format!("{}/secure-cloud/cpu-rentals/start", self.base_url);
        let body = StartCpuRentalRequest {
            offering_id: offering_id.to_string(),
            ssh_public_key_id: ssh_key_id.to_string(),
        };
        info!("Starting Basilica CPU rental: offering={}", offering_id);
        let resp = self.client.post(&url).json(&body).send().await
            .context("Failed to start CPU rental")?;
        self.handle_response(resp, "start_cpu_rental").await
    }

    pub async fn stop_cpu_rental(&self, rental_id: &str) -> Result<StopRentalResponse> {
        let url = format!("{}/secure-cloud/cpu-rentals/{}/stop", self.base_url, rental_id);
        info!("Stopping Basilica CPU rental: {}", rental_id);
        let resp = self.client.post(&url).json(&serde_json::json!({})).send().await
            .context("Failed to stop CPU rental")?;
        self.handle_response(resp, "stop_cpu_rental").await
    }

    // ── Secure cloud GPU rentals ──

    pub async fn list_gpu_offerings(&self) -> Result<serde_json::Value> {
        let url = format!("{}/secure-cloud/gpu-prices?available_only=true", self.base_url);
        let resp = self.client.get(&url).send().await
            .context("Failed to list GPU offerings")?;
        self.handle_response(resp, "list_gpu_offerings").await
    }

    pub async fn start_gpu_rental(&self, offering_id: &str, ssh_key_id: &str) -> Result<SecureCloudRentalResponse> {
        let url = format!("{}/secure-cloud/rentals/start", self.base_url);
        let body = StartCpuRentalRequest {
            offering_id: offering_id.to_string(),
            ssh_public_key_id: ssh_key_id.to_string(),
        };
        info!("Starting Basilica GPU rental: offering={}", offering_id);
        let resp = self.client.post(&url).json(&body).send().await
            .context("Failed to start GPU rental")?;
        self.handle_response(resp, "start_gpu_rental").await
    }

    pub async fn stop_gpu_rental(&self, rental_id: &str) -> Result<StopRentalResponse> {
        let url = format!("{}/secure-cloud/rentals/{}/stop", self.base_url, rental_id);
        info!("Stopping Basilica GPU rental: {}", rental_id);
        let resp = self.client.post(&url).json(&serde_json::json!({})).send().await
            .context("Failed to stop GPU rental")?;
        self.handle_response(resp, "stop_gpu_rental").await
    }

    // ── Deployments ──

    pub async fn create_deployment(&self, req: &CreateDeploymentRequest) -> Result<DeploymentResponse> {
        let url = format!("{}/deployments", self.base_url);
        info!("Creating Basilica deployment: name={}, image={}", req.instance_name, req.image);
        let resp = self.client.post(&url).json(req).send().await
            .context("Failed to create deployment")?;
        self.handle_response(resp, "create_deployment").await
    }

    pub async fn get_deployment(&self, name: &str) -> Result<DeploymentResponse> {
        let url = format!("{}/deployments/{}", self.base_url, name);
        let resp = self.client.get(&url).send().await
            .context("Failed to get deployment")?;
        self.handle_response(resp, "get_deployment").await
    }

    pub async fn delete_deployment(&self, name: &str) -> Result<DeleteDeploymentResponse> {
        let url = format!("{}/deployments/{}", self.base_url, name);
        info!("Deleting Basilica deployment: {}", name);
        let resp = self.client.delete(&url).send().await
            .context("Failed to delete deployment")?;
        self.handle_response(resp, "delete_deployment").await
    }

    // ── Balance ──

    pub async fn get_balance(&self) -> Result<BalanceResponse> {
        let url = format!("{}/billing/balance", self.base_url);
        let resp = self.client.get(&url).send().await
            .context("Failed to get balance")?;
        self.handle_response(resp, "get_balance").await
    }

    // ── High-level: provision CPU container and wait for SSH ──

    pub async fn provision_cpu_container(
        &self,
        ssh_key_id: &str,
        min_cpu: Option<u32>,
        min_memory_gb: Option<u32>,
    ) -> Result<ContainerInfo> {
        let offerings = self.list_cpu_offerings().await?;
        let offering = offerings.nodes.iter()
            .filter(|o| {
                min_cpu.map_or(true, |c| o.cpu_count.unwrap_or(0) >= c)
                    && min_memory_gb.map_or(true, |m| o.memory_gb.unwrap_or(0) >= m)
            })
            .min_by_key(|o| o.hourly_rate_cents.unwrap_or(u32::MAX))
            .context("No CPU offering matches the requested specs")?;

        info!(
            "Selected CPU offering: {} ({}cpu, {}GB, {}c/hr)",
            offering.id,
            offering.cpu_count.unwrap_or(0),
            offering.memory_gb.unwrap_or(0),
            offering.hourly_rate_cents.unwrap_or(0),
        );

        let rental = self.start_cpu_rental(&offering.id, ssh_key_id).await?;
        info!("CPU rental started: {} (status: {})", rental.rental_id, rental.status);

        self.wait_for_ssh(&rental.rental_id).await
    }

    /// Poll rental status until SSH is available or timeout.
    async fn wait_for_ssh(&self, rental_id: &str) -> Result<ContainerInfo> {
        for attempt in 1..=MAX_POLL_ATTEMPTS {
            tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

            let status = self.get_rental(rental_id).await;
            match status {
                Ok(s) => {
                    debug!(
                        "Rental {} poll {}/{}: status={}",
                        rental_id, attempt, MAX_POLL_ATTEMPTS, s.status
                    );

                    if s.status == "running" || s.status == "active" {
                        if let Some(ref creds) = s.ssh_credentials {
                            if creds.host.is_some() {
                                info!("Rental {} is ready with SSH access", rental_id);
                                return Ok(ContainerInfo {
                                    rental_id: rental_id.to_string(),
                                    status: s.status,
                                    ssh_host: creds.host.clone(),
                                    ssh_port: creds.port,
                                    ssh_user: creds.username.clone(),
                                    ssh_command: creds.ssh_command.clone(),
                                    provider: s.node.clone(),
                                    created_at: s.created_at.clone(),
                                });
                            }
                        }
                    }

                    if s.status == "failed" || s.status == "error" || s.status == "terminated" {
                        anyhow::bail!("Rental {} entered terminal state: {}", rental_id, s.status);
                    }
                }
                Err(e) => {
                    warn!("Failed to poll rental {}: {}", rental_id, e);
                }
            }
        }

        anyhow::bail!(
            "Rental {} did not become ready within {}s",
            rental_id,
            MAX_POLL_ATTEMPTS as u64 * POLL_INTERVAL_SECS
        )
    }

    // ── Response handling ──

    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        resp: reqwest::Response,
        operation: &str,
    ) -> Result<T> {
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "Basilica API {} failed (HTTP {}): {}",
                operation,
                status.as_u16(),
                &body[..body.len().min(500)]
            );
        }
        resp.json::<T>().await
            .with_context(|| format!("Failed to parse Basilica {} response", operation))
    }

    async fn handle_empty_response(
        &self,
        resp: reqwest::Response,
        operation: &str,
    ) -> Result<()> {
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "Basilica API {} failed (HTTP {}): {}",
                operation,
                status.as_u16(),
                &body[..body.len().min(500)]
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = BasilicaClient::new("test-token-123");
        assert!(client.is_ok());
    }

    #[test]
    fn test_default_api_url() {
        std::env::remove_var("BASILICA_API_URL");
        let client = BasilicaClient::new("test").unwrap();
        assert_eq!(client.base_url, DEFAULT_API_URL);
    }
}
