use std::path::PathBuf;

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_SESSION_TTL: u64 = 7200;
const DEFAULT_MAX_CONCURRENT: usize = 8;
const DEFAULT_CLONE_TIMEOUT: u64 = 180;
const DEFAULT_AGENT_TIMEOUT: u64 = 600;
const DEFAULT_TEST_TIMEOUT: u64 = 300;
const DEFAULT_MAX_ARCHIVE_BYTES: usize = 500 * 1024 * 1024;
#[allow(dead_code)]
const DEFAULT_MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const DEFAULT_WORKSPACE_BASE: &str = "/tmp/sessions";
const DEFAULT_BITTENSOR_NETUID: u16 = 100;
const DEFAULT_MIN_VALIDATOR_STAKE_TAO: f64 = 10_000.0;
const DEFAULT_VALIDATOR_REFRESH_SECS: u64 = 300;
const DEFAULT_CONSENSUS_THRESHOLD: f64 = 0.5;
const DEFAULT_CONSENSUS_TTL_SECS: u64 = 60;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub session_ttl_secs: u64,
    pub max_concurrent_tasks: usize,
    pub clone_timeout_secs: u64,
    pub agent_timeout_secs: u64,
    pub test_timeout_secs: u64,
    pub max_archive_bytes: usize,
    #[allow(dead_code)]
    pub max_output_bytes: usize,
    pub workspace_base: PathBuf,
    pub bittensor_netuid: u16,
    pub min_validator_stake_tao: f64,
    pub validator_refresh_secs: u64,
    pub consensus_threshold: f64,
    pub consensus_ttl_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        let consensus_threshold: f64 =
            env_parse("CONSENSUS_THRESHOLD", DEFAULT_CONSENSUS_THRESHOLD);

        assert!(
            consensus_threshold > 0.0 && consensus_threshold <= 1.0,
            "CONSENSUS_THRESHOLD must be in range (0.0, 1.0], got {}",
            consensus_threshold
        );

        Self {
            port: env_parse("PORT", DEFAULT_PORT),
            session_ttl_secs: env_parse("SESSION_TTL_SECS", DEFAULT_SESSION_TTL),
            max_concurrent_tasks: env_parse("MAX_CONCURRENT_TASKS", DEFAULT_MAX_CONCURRENT),
            clone_timeout_secs: env_parse("CLONE_TIMEOUT_SECS", DEFAULT_CLONE_TIMEOUT),
            agent_timeout_secs: env_parse("AGENT_TIMEOUT_SECS", DEFAULT_AGENT_TIMEOUT),
            test_timeout_secs: env_parse("TEST_TIMEOUT_SECS", DEFAULT_TEST_TIMEOUT),
            max_archive_bytes: env_parse("MAX_ARCHIVE_BYTES", DEFAULT_MAX_ARCHIVE_BYTES),
            max_output_bytes: env_parse("MAX_OUTPUT_BYTES", DEFAULT_MAX_OUTPUT_BYTES),
            workspace_base: PathBuf::from(
                std::env::var("WORKSPACE_BASE").unwrap_or_else(|_| DEFAULT_WORKSPACE_BASE.into()),
            ),
            bittensor_netuid: env_parse("BITTENSOR_NETUID", DEFAULT_BITTENSOR_NETUID),
            min_validator_stake_tao: env_parse(
                "MIN_VALIDATOR_STAKE_TAO",
                DEFAULT_MIN_VALIDATOR_STAKE_TAO,
            ),
            validator_refresh_secs: env_parse(
                "VALIDATOR_REFRESH_SECS",
                DEFAULT_VALIDATOR_REFRESH_SECS,
            ),
            consensus_threshold,
            consensus_ttl_secs: env_parse("CONSENSUS_TTL_SECS", DEFAULT_CONSENSUS_TTL_SECS),
        }
    }

    pub fn print_banner(&self) {
        tracing::info!("╔══════════════════════════════════════════════════╗");
        tracing::info!(
            "║        term-executor v{}                  ║",
            env!("CARGO_PKG_VERSION")
        );
        tracing::info!("╠══════════════════════════════════════════════════╣");
        tracing::info!("║  Port:              {:<28}║", self.port);
        tracing::info!("║  Bittensor netuid:  {:<28}║", self.bittensor_netuid);
        tracing::info!(
            "║  Min stake (TAO):   {:<28}║",
            self.min_validator_stake_tao
        );
        tracing::info!(
            "║  Whitelist refresh: {:<25}s ║",
            self.validator_refresh_secs
        );
        tracing::info!("║  Consensus thresh:  {:<28}║", self.consensus_threshold);
        tracing::info!("║  Consensus TTL:     {:<25}s ║", self.consensus_ttl_secs);
        tracing::info!("║  Max concurrent:    {:<28}║", self.max_concurrent_tasks);
        tracing::info!("║  Session TTL:       {:<25}s ║", self.session_ttl_secs);
        tracing::info!("║  Clone timeout:     {:<25}s ║", self.clone_timeout_secs);
        tracing::info!("║  Agent timeout:     {:<25}s ║", self.agent_timeout_secs);
        tracing::info!("║  Test timeout:      {:<25}s ║", self.test_timeout_secs);
        tracing::info!(
            "║  Workspace:         {:<28}║",
            self.workspace_base.display()
        );
        tracing::info!("╚══════════════════════════════════════════════════╝");
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
        assert_eq!(cfg.max_concurrent_tasks, DEFAULT_MAX_CONCURRENT);
        assert_eq!(cfg.bittensor_netuid, 100);
        assert!((cfg.consensus_threshold - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_env_parse_fallback() {
        assert_eq!(env_parse::<u16>("NONEXISTENT_VAR_XYZ", 42), 42);
    }

    #[test]
    #[should_panic(expected = "CONSENSUS_THRESHOLD must be in range")]
    fn test_config_rejects_zero_threshold() {
        std::env::set_var("CONSENSUS_THRESHOLD", "0.0");
        let _cfg = Config::from_env();
        std::env::remove_var("CONSENSUS_THRESHOLD");
    }

    #[test]
    #[should_panic(expected = "CONSENSUS_THRESHOLD must be in range")]
    fn test_config_rejects_threshold_above_one() {
        std::env::set_var("CONSENSUS_THRESHOLD", "1.5");
        let _cfg = Config::from_env();
        std::env::remove_var("CONSENSUS_THRESHOLD");
    }
}
