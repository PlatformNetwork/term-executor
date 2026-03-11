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
        let base_url =
            std::env::var("BASILICA_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());

        let mut headers = reqwest::header::HeaderMap::new();
        let auth_value = format!("Bearer {}", api_token);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            auth_value.parse().context("Invalid API token format")?,
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
        let resp = self
            .client
            .get(&url)
            .send()
            .await
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
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to register SSH key")?;
        self.handle_response(resp, "register_ssh_key").await
    }

    pub async fn get_ssh_key(&self) -> Result<Option<SshKeyResponse>> {
        let url = format!("{}/ssh-keys", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get SSH key")?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let key: SshKeyResponse = self.handle_response(resp, "get_ssh_key").await?;
        Ok(Some(key))
    }

    pub async fn delete_ssh_key(&self) -> Result<()> {
        let url = format!("{}/ssh-keys", self.base_url);
        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .context("Failed to delete SSH key")?;
        self.handle_empty_response(resp, "delete_ssh_key").await
    }

    // ── Community cloud rentals (GPU) ──

    pub async fn start_rental(&self, req: &StartRentalRequest) -> Result<RentalResponse> {
        let url = format!("{}/rentals", self.base_url);
        info!("Starting Basilica rental: image={}", req.container_image);
        let resp = self
            .client
            .post(&url)
            .json(req)
            .send()
            .await
            .context("Failed to start rental")?;
        self.handle_response(resp, "start_rental").await
    }

    pub async fn get_rental(&self, rental_id: &str) -> Result<RentalStatusResponse> {
        let url = format!("{}/rentals/{}", self.base_url, rental_id);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get rental status")?;
        self.handle_response(resp, "get_rental").await
    }

    pub async fn stop_rental(&self, rental_id: &str) -> Result<()> {
        let url = format!("{}/rentals/{}", self.base_url, rental_id);
        info!("Stopping Basilica rental: {}", rental_id);
        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .context("Failed to stop rental")?;
        self.handle_empty_response(resp, "stop_rental").await
    }

    pub async fn list_rentals(&self) -> Result<ListRentalsResponse> {
        let url = format!("{}/rentals", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to list rentals")?;
        self.handle_response(resp, "list_rentals").await
    }

    // ── Secure cloud CPU rentals ──

    pub async fn list_cpu_offerings(&self) -> Result<ListCpuOfferingsResponse> {
        let url = format!(
            "{}/secure-cloud/cpu-prices?available_only=true",
            self.base_url
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to list CPU offerings")?;
        self.handle_response(resp, "list_cpu_offerings").await
    }

    pub async fn start_cpu_rental(
        &self,
        offering_id: &str,
        ssh_key_id: &str,
    ) -> Result<SecureCloudRentalResponse> {
        let url = format!("{}/secure-cloud/cpu-rentals/start", self.base_url);
        let body = StartCpuRentalRequest {
            offering_id: offering_id.to_string(),
            ssh_public_key_id: ssh_key_id.to_string(),
        };
        info!("Starting Basilica CPU rental: offering={}", offering_id);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to start CPU rental")?;
        self.handle_response(resp, "start_cpu_rental").await
    }

    pub async fn list_cpu_rentals(&self) -> Result<SecureCloudRentalListResponse> {
        let url = format!("{}/secure-cloud/cpu-rentals", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to list CPU rentals")?;
        self.handle_response(resp, "list_cpu_rentals").await
    }

    pub async fn stop_cpu_rental(&self, rental_id: &str) -> Result<StopRentalResponse> {
        let url = format!(
            "{}/secure-cloud/cpu-rentals/{}/stop",
            self.base_url, rental_id
        );
        info!("Stopping Basilica CPU rental: {}", rental_id);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await
            .context("Failed to stop CPU rental")?;
        self.handle_response(resp, "stop_cpu_rental").await
    }

    // ── Secure cloud GPU rentals ──

    pub async fn list_gpu_offerings(&self) -> Result<serde_json::Value> {
        let url = format!(
            "{}/secure-cloud/gpu-prices?available_only=true",
            self.base_url
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to list GPU offerings")?;
        self.handle_response(resp, "list_gpu_offerings").await
    }

    pub async fn start_gpu_rental(
        &self,
        offering_id: &str,
        ssh_key_id: &str,
    ) -> Result<SecureCloudRentalResponse> {
        let url = format!("{}/secure-cloud/rentals/start", self.base_url);
        let body = StartCpuRentalRequest {
            offering_id: offering_id.to_string(),
            ssh_public_key_id: ssh_key_id.to_string(),
        };
        info!("Starting Basilica GPU rental: offering={}", offering_id);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to start GPU rental")?;
        self.handle_response(resp, "start_gpu_rental").await
    }

    pub async fn stop_gpu_rental(&self, rental_id: &str) -> Result<StopRentalResponse> {
        let url = format!("{}/secure-cloud/rentals/{}/stop", self.base_url, rental_id);
        info!("Stopping Basilica GPU rental: {}", rental_id);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await
            .context("Failed to stop GPU rental")?;
        self.handle_response(resp, "stop_gpu_rental").await
    }

    // ── Deployments ──

    pub async fn create_deployment(
        &self,
        req: &CreateDeploymentRequest,
    ) -> Result<DeploymentResponse> {
        let url = format!("{}/deployments", self.base_url);
        info!(
            "Creating Basilica deployment: name={}, image={}",
            req.instance_name, req.image
        );
        let resp = self
            .client
            .post(&url)
            .json(req)
            .send()
            .await
            .context("Failed to create deployment")?;
        self.handle_response(resp, "create_deployment").await
    }

    pub async fn get_deployment(&self, name: &str) -> Result<DeploymentResponse> {
        let url = format!("{}/deployments/{}", self.base_url, name);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get deployment")?;
        self.handle_response(resp, "get_deployment").await
    }

    pub async fn delete_deployment(&self, name: &str) -> Result<DeleteDeploymentResponse> {
        let url = format!("{}/deployments/{}", self.base_url, name);
        info!("Deleting Basilica deployment: {}", name);
        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .context("Failed to delete deployment")?;
        self.handle_response(resp, "delete_deployment").await
    }

    // ── Balance ──

    pub async fn get_balance(&self) -> Result<BalanceResponse> {
        let url = format!("{}/billing/balance", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
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
        let offering = offerings
            .nodes
            .iter()
            .filter(|o| {
                o.availability.unwrap_or(false)
                    && min_cpu.is_none_or(|c| o.vcpu_count.unwrap_or(0) >= c)
                    && min_memory_gb.is_none_or(|m| o.system_memory_gb.unwrap_or(0) >= m)
            })
            .min_by(|a, b| {
                let rate_a: f64 = a
                    .hourly_rate
                    .as_deref()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(f64::MAX);
                let rate_b: f64 = b
                    .hourly_rate
                    .as_deref()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(f64::MAX);
                rate_a
                    .partial_cmp(&rate_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .context("No CPU offering matches the requested specs")?;

        info!(
            "Selected CPU offering: {} ({}vcpu, {}GB, ${}/hr, {})",
            offering.id,
            offering.vcpu_count.unwrap_or(0),
            offering.system_memory_gb.unwrap_or(0),
            offering.hourly_rate.as_deref().unwrap_or("?"),
            offering.provider.as_deref().unwrap_or("?"),
        );

        let rental = self.start_cpu_rental(&offering.id, ssh_key_id).await?;
        info!(
            "CPU rental started: {} (status: {})",
            rental.rental_id, rental.status
        );

        self.wait_for_cpu_ssh(&rental.rental_id).await
    }

    /// Poll secure-cloud CPU rental status until SSH is available or timeout.
    async fn wait_for_cpu_ssh(&self, rental_id: &str) -> Result<ContainerInfo> {
        for attempt in 1..=MAX_POLL_ATTEMPTS {
            tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

            match self.list_cpu_rentals().await {
                Ok(list) => {
                    if let Some(r) = list.rentals.iter().find(|r| r.rental_id == rental_id) {
                        debug!(
                            "Rental {} poll {}/{}: status={} ip={:?}",
                            rental_id, attempt, MAX_POLL_ATTEMPTS, r.status, r.ip_address
                        );

                        if r.status == "running" || r.status == "active" {
                            if let Some(ref ip) = r.ip_address {
                                let ssh_user = r
                                    .ssh_command
                                    .as_deref()
                                    .and_then(|c| c.strip_prefix("ssh "))
                                    .and_then(|c| c.split('@').next())
                                    .unwrap_or("root")
                                    .to_string();

                                // Verify SSH is actually reachable before returning
                                let ssh_key_path = std::env::var("BASILICA_SSH_KEY").ok();
                                if !self
                                    .check_ssh_ready(ip, 22, &ssh_user, ssh_key_path.as_deref())
                                    .await
                                {
                                    debug!(
                                        "Rental {} has IP {} but SSH not ready yet",
                                        rental_id, ip
                                    );
                                    continue;
                                }

                                info!("Rental {} is ready: {}@{}", rental_id, ssh_user, ip);
                                return Ok(ContainerInfo {
                                    rental_id: rental_id.to_string(),
                                    status: r.status.clone(),
                                    ssh_host: Some(ip.clone()),
                                    ssh_port: Some(22),
                                    ssh_user: Some(ssh_user),
                                    ssh_command: r.ssh_command.clone(),
                                    provider: r.provider.clone(),
                                    created_at: r.created_at.clone(),
                                });
                            }
                        }

                        if r.status == "failed" || r.status == "error" || r.status == "stopped" {
                            anyhow::bail!(
                                "Rental {} entered terminal state: {}",
                                rental_id,
                                r.status
                            );
                        }
                    } else {
                        warn!("Rental {} not found in CPU rental list", rental_id);
                    }
                }
                Err(e) => {
                    warn!("Failed to poll CPU rentals: {}", e);
                }
            }
        }

        anyhow::bail!(
            "Rental {} did not become ready within {}s",
            rental_id,
            MAX_POLL_ATTEMPTS as u64 * POLL_INTERVAL_SECS
        )
    }

    /// Check if SSH is reachable on the given host by running a simple command.
    async fn check_ssh_ready(
        &self,
        host: &str,
        port: u16,
        user: &str,
        ssh_key: Option<&str>,
    ) -> bool {
        let target = format!("{}@{}", user, host);
        let port_str = port.to_string();
        let mut args = vec![
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "UserKnownHostsFile=/dev/null",
            "-o",
            "ConnectTimeout=5",
            "-o",
            "LogLevel=ERROR",
        ];
        if let Some(key) = ssh_key {
            args.extend_from_slice(&["-i", key]);
        }
        args.extend_from_slice(&["-p", &port_str, &target, "echo ok"]);
        let result = tokio::process::Command::new("ssh")
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await;

        match result {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
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
        resp.json::<T>()
            .await
            .with_context(|| format!("Failed to parse Basilica {} response", operation))
    }

    async fn handle_empty_response(&self, resp: reqwest::Response, operation: &str) -> Result<()> {
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
