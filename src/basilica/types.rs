use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Rental creation (community cloud) ──

#[derive(Debug, Clone, Serialize)]
pub struct StartRentalRequest {
    pub gpu_category: String,
    pub gpu_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_memory_gb: Option<u32>,
    pub max_hourly_rate_cents: u32,
    pub container_image: String,
    pub ssh_public_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RentalResponse {
    pub rental_id: String,
    pub ssh_credentials: Option<SshCredentials>,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshCredentials {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub ssh_command: Option<String>,
}

// ── Rental status ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalStatusResponse {
    pub rental_id: String,
    pub status: String,
    pub node: Option<String>,
    pub ssh_credentials: Option<SshCredentials>,
    pub port_mappings: Option<HashMap<String, String>>,
    pub ssh_public_key: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// ── Rental listing ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRentalsResponse {
    pub rentals: Vec<RentalListItem>,
    pub total_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalListItem {
    pub rental_id: String,
    pub status: String,
    pub gpu_type: Option<String>,
    pub gpu_count: Option<u32>,
    pub created_at: Option<String>,
}

// ── Secure cloud CPU ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuOffering {
    pub id: String,
    pub provider: Option<String>,
    pub vcpu_count: Option<u32>,
    pub system_memory_gb: Option<u32>,
    pub storage_gb: Option<u32>,
    pub hourly_rate: Option<String>,
    pub region: Option<String>,
    pub availability: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCpuOfferingsResponse {
    pub nodes: Vec<CpuOffering>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartCpuRentalRequest {
    pub offering_id: String,
    pub ssh_public_key_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecureCloudRentalResponse {
    pub rental_id: String,
    pub deployment_id: Option<String>,
    pub provider: Option<String>,
    pub status: String,
    pub ip_address: Option<String>,
    pub ssh_command: Option<String>,
    pub hourly_cost: Option<f64>,
    pub is_spot: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureCloudRentalListResponse {
    pub rentals: Vec<SecureCloudRentalListItem>,
    pub total_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureCloudRentalListItem {
    pub rental_id: String,
    pub status: String,
    pub ip_address: Option<String>,
    pub ssh_command: Option<String>,
    pub provider: Option<String>,
    pub hourly_cost: Option<f64>,
    pub created_at: Option<String>,
    pub stopped_at: Option<String>,
    pub vcpu_count: Option<u32>,
    pub system_memory_gb: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StopRentalResponse {
    pub rental_id: String,
    pub status: String,
    pub duration_hours: Option<f64>,
    pub total_cost: Option<f64>,
}

// ── SSH key management ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSshKeyRequest {
    pub name: String,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyResponse {
    pub id: String,
    pub user_id: Option<String>,
    pub name: String,
    pub public_key: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// ── Deployments ──

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeploymentRequest {
    pub instance_name: String,
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<DeploymentResources>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_billing: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentResources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_request: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_request: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpus: Option<GpuResources>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuResources {
    pub count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_gpu_memory_gb: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentResponse {
    pub instance_name: Option<String>,
    pub user_id: Option<String>,
    pub namespace: Option<String>,
    pub state: Option<String>,
    pub url: Option<String>,
    pub replicas: Option<u32>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDeploymentResponse {
    pub instance_name: Option<String>,
    pub state: Option<String>,
    pub message: Option<String>,
}

// ── Health ──

#[derive(Debug, Clone, Deserialize)]
pub struct HealthResponse {
    pub status: Option<String>,
    pub version: Option<String>,
}

// ── Balance ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub balance: Option<String>,
    pub last_updated: Option<String>,
}

// ── Container lifecycle (high-level abstraction) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub rental_id: String,
    pub status: String,
    pub ssh_host: Option<String>,
    pub ssh_port: Option<u16>,
    pub ssh_user: Option<String>,
    pub ssh_command: Option<String>,
    pub provider: Option<String>,
    pub created_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_rental_request_serializes() {
        let req = StartRentalRequest {
            gpu_category: "RTX_4090".to_string(),
            gpu_count: 1,
            min_memory_gb: Some(24),
            max_hourly_rate_cents: 100,
            container_image: "nvidia/cuda:12.8.0-runtime-ubuntu22.04".to_string(),
            ssh_public_key: "ssh-ed25519 AAAA...".to_string(),
            environment: None,
            ports: None,
            command: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("gpu_category"));
        assert!(!json.contains("environment"));
    }

    #[test]
    fn test_rental_response_deserializes() {
        let json = r#"{
            "rental_id": "abc-123",
            "status": "running",
            "ssh_credentials": {
                "host": "1.2.3.4",
                "port": 22,
                "username": "root",
                "ssh_command": "ssh root@1.2.3.4"
            },
            "container_id": null,
            "container_name": null
        }"#;
        let resp: RentalResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.rental_id, "abc-123");
        assert_eq!(resp.status, "running");
        let creds = resp.ssh_credentials.unwrap();
        assert_eq!(creds.host.as_deref(), Some("1.2.3.4"));
        assert_eq!(creds.port, Some(22));
    }

    #[test]
    fn test_container_info_roundtrip() {
        let info = ContainerInfo {
            rental_id: "r-123".to_string(),
            status: "running".to_string(),
            ssh_host: Some("10.0.0.1".to_string()),
            ssh_port: Some(22),
            ssh_user: Some("root".to_string()),
            ssh_command: Some("ssh root@10.0.0.1".to_string()),
            provider: Some("citadel".to_string()),
            created_at: Some("2026-03-04T12:00:00Z".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: ContainerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.rental_id, "r-123");
    }

    #[test]
    fn test_deployment_request_camel_case() {
        let req = CreateDeploymentRequest {
            instance_name: "my-app".to_string(),
            image: "nginx:latest".to_string(),
            replicas: Some(1),
            port: Some(80),
            command: None,
            args: None,
            env: None,
            resources: None,
            ttl_seconds: Some(3600),
            public: Some(true),
            enable_billing: Some(true),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("instanceName"));
        assert!(json.contains("ttlSeconds"));
        assert!(json.contains("enableBilling"));
        assert!(!json.contains("instance_name"));
    }

    #[test]
    fn test_cpu_offering_deserializes() {
        let json = r#"{
            "id": "hyperstack-127",
            "provider": "hyperstack",
            "vcpu_count": 4,
            "system_memory_gb": 4,
            "storage_gb": 100,
            "hourly_rate": "0.3832400",
            "region": "NORWAY-1",
            "availability": true
        }"#;
        let offering: CpuOffering = serde_json::from_str(json).unwrap();
        assert_eq!(offering.id, "hyperstack-127");
        assert_eq!(offering.vcpu_count, Some(4));
        assert_eq!(offering.availability, Some(true));
    }
}
