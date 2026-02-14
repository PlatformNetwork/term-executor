use std::path::PathBuf;

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_SESSION_TTL: u64 = 1800;
const DEFAULT_MAX_CONCURRENT: usize = 4;
const DEFAULT_DISK_QUOTA_MB: u64 = 2048;
const DEFAULT_CLONE_TIMEOUT: u64 = 120;
const DEFAULT_AGENT_TIMEOUT: u64 = 600;
const DEFAULT_TEST_TIMEOUT: u64 = 300;
const DEFAULT_MAX_AGENT_CODE_BYTES: usize = 5 * 1024 * 1024;
const DEFAULT_MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const DEFAULT_WORKSPACE_BASE: &str = "/tmp/sessions";

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub auth_token: Option<String>,
    pub session_ttl_secs: u64,
    pub max_concurrent_evals: usize,
    pub disk_quota_mb: u64,
    pub clone_timeout_secs: u64,
    pub agent_timeout_secs: u64,
    pub test_timeout_secs: u64,
    pub max_agent_code_bytes: usize,
    #[allow(dead_code)]
    pub max_output_bytes: usize,
    pub workspace_base: PathBuf,
    pub basilica_api_token: Option<String>,
    pub basilica_instance_name: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env_parse("PORT", DEFAULT_PORT),
            auth_token: std::env::var("AUTH_TOKEN").ok(),
            session_ttl_secs: env_parse("SESSION_TTL_SECS", DEFAULT_SESSION_TTL),
            max_concurrent_evals: env_parse("MAX_CONCURRENT_EVALS", DEFAULT_MAX_CONCURRENT),
            disk_quota_mb: env_parse("DISK_QUOTA_MB", DEFAULT_DISK_QUOTA_MB),
            clone_timeout_secs: env_parse("CLONE_TIMEOUT_SECS", DEFAULT_CLONE_TIMEOUT),
            agent_timeout_secs: env_parse("AGENT_TIMEOUT_SECS", DEFAULT_AGENT_TIMEOUT),
            test_timeout_secs: env_parse("TEST_TIMEOUT_SECS", DEFAULT_TEST_TIMEOUT),
            max_agent_code_bytes: env_parse("MAX_AGENT_CODE_BYTES", DEFAULT_MAX_AGENT_CODE_BYTES),
            max_output_bytes: env_parse("MAX_OUTPUT_BYTES", DEFAULT_MAX_OUTPUT_BYTES),
            workspace_base: PathBuf::from(
                std::env::var("WORKSPACE_BASE").unwrap_or_else(|_| DEFAULT_WORKSPACE_BASE.into()),
            ),
            basilica_api_token: std::env::var("BASILICA_API_TOKEN").ok(),
            basilica_instance_name: std::env::var("BASILICA_INSTANCE_NAME").ok(),
        }
    }

    pub fn print_banner(&self) {
        tracing::info!("╔══════════════════════════════════════════════════╗");
        tracing::info!("║           term-executor v{}              ║", env!("CARGO_PKG_VERSION"));
        tracing::info!("╠══════════════════════════════════════════════════╣");
        tracing::info!("║  Port:              {:<28}║", self.port);
        tracing::info!("║  Auth:              {:<28}║", if self.auth_token.is_some() { "enabled" } else { "disabled" });
        tracing::info!("║  Max concurrent:    {:<28}║", self.max_concurrent_evals);
        tracing::info!("║  Session TTL:       {:<25}s ║", self.session_ttl_secs);
        tracing::info!("║  Disk quota:        {:<24}MB ║", self.disk_quota_mb);
        tracing::info!("║  Clone timeout:     {:<25}s ║", self.clone_timeout_secs);
        tracing::info!("║  Agent timeout:     {:<25}s ║", self.agent_timeout_secs);
        tracing::info!("║  Test timeout:      {:<25}s ║", self.test_timeout_secs);
        tracing::info!("║  Workspace:         {:<28}║", self.workspace_base.display());
        tracing::info!("║  Basilica:          {:<28}║", if self.basilica_api_token.is_some() { "connected" } else { "standalone" });
        tracing::info!("╚══════════════════════════════════════════════════╝");

        if self.basilica_instance_name.is_some() {
            let name = self.basilica_instance_name.as_deref().unwrap();
            tracing::info!("");
            tracing::info!("Recommended Basilica deploy command:");
            tracing::info!("  basilica deploy ghcr.io/platformnetwork/term-executor:latest \\");
            tracing::info!("    --name {} \\", name);
            tracing::info!("    --port {} \\", self.port);
            tracing::info!("    --public-metadata \\");
            tracing::info!("    --health-path /health \\");
            tracing::info!("    --cpu 2 --memory 4Gi");
        }
    }
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let cfg = Config::from_env();
        assert_eq!(cfg.port, DEFAULT_PORT);
        assert_eq!(cfg.max_concurrent_evals, DEFAULT_MAX_CONCURRENT);
        assert_eq!(cfg.disk_quota_mb, DEFAULT_DISK_QUOTA_MB);
    }

    #[test]
    fn test_env_parse_fallback() {
        assert_eq!(env_parse::<u16>("NONEXISTENT_VAR_XYZ", 42), 42);
    }
}
