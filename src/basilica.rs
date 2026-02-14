use tracing::{info, warn};

/// On startup, if Basilica credentials are configured, attempt to enroll
/// this deployment for public metadata so validators can verify it.
pub async fn try_enroll_metadata(api_token: &str, instance_name: &str) {
    let url = format!(
        "https://api.basilica.ai/deployments/{}/enroll-metadata",
        urlencoding::encode(instance_name)
    );

    let client = reqwest::Client::new();
    let body = serde_json::json!({ "enabled": true });

    match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_token))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            info!(
                "Basilica public metadata enrolled for '{}'",
                instance_name
            );
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(
                "Failed to enroll Basilica metadata (HTTP {}): {}",
                status, body
            );
        }
        Err(e) => {
            warn!("Failed to reach Basilica API for metadata enrollment: {}", e);
        }
    }
}

/// Verify own health endpoint is reachable (self-test).
#[allow(dead_code)]
pub async fn self_health_check(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{}/health", port);
    match reqwest::get(&url).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
