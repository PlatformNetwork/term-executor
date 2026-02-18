use anyhow::Context;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

pub struct ValidatorWhitelist {
    hotkeys: RwLock<HashSet<String>>,
}

impl ValidatorWhitelist {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            hotkeys: RwLock::new(HashSet::new()),
        })
    }

    pub fn is_whitelisted(&self, ss58_hotkey: &str) -> bool {
        self.hotkeys.read().contains(ss58_hotkey)
    }

    pub fn validator_count(&self) -> usize {
        self.hotkeys.read().len()
    }

    #[cfg(test)]
    pub fn insert_for_test(&self, hotkey: &str) {
        self.hotkeys.write().insert(hotkey.to_string());
    }

    pub async fn refresh_loop(self: Arc<Self>, netuid: u16, min_stake_tao: f64, refresh_secs: u64) {
        let mut interval = tokio::time::interval(Duration::from_secs(refresh_secs));
        loop {
            interval.tick().await;
            self.refresh_once(netuid, min_stake_tao).await;
        }
    }

    async fn refresh_once(&self, netuid: u16, min_stake_tao: f64) {
        let mut last_err = None;
        for attempt in 0..3u32 {
            if attempt > 0 {
                let delay = Duration::from_secs(2u64.pow(attempt));
                tokio::time::sleep(delay).await;
            }
            match self.try_refresh(netuid, min_stake_tao).await {
                Ok(count) => {
                    info!(count, netuid, "Validator whitelist refreshed successfully");
                    return;
                }
                Err(e) => {
                    warn!(
                        attempt = attempt + 1,
                        error = %e,
                        "Failed to refresh validator whitelist"
                    );
                    last_err = Some(e);
                }
            }
        }
        warn!(
            error = %last_err.unwrap_or_else(|| anyhow::anyhow!("unknown")),
            "All retry attempts failed for validator whitelist refresh, keeping cached whitelist"
        );
    }

    async fn try_refresh(&self, netuid: u16, min_stake_tao: f64) -> anyhow::Result<usize> {
        use bittensor_rs::ss58::encode_ss58;

        let client = bittensor_rs::BittensorClient::with_failover()
            .await
            .context("Failed to connect to subtensor")?;

        let metagraph = bittensor_rs::sync_metagraph(&client, netuid)
            .await
            .context("Failed to sync metagraph")?;

        let mut new_hotkeys = HashSet::new();
        for neuron in metagraph.neurons.values() {
            if neuron.validator_permit && neuron.active && neuron.stake.as_tao() >= min_stake_tao {
                new_hotkeys.insert(encode_ss58(&neuron.hotkey));
            }
        }

        let count = new_hotkeys.len();
        *self.hotkeys.write() = new_hotkeys;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starts_empty() {
        let wl = ValidatorWhitelist::new();
        assert_eq!(wl.validator_count(), 0);
    }

    #[test]
    fn test_is_whitelisted() {
        let wl = ValidatorWhitelist::new();
        let hotkey = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";
        assert!(!wl.is_whitelisted(hotkey));

        wl.hotkeys.write().insert(hotkey.to_string());
        assert!(wl.is_whitelisted(hotkey));
    }

    #[test]
    fn test_validator_count() {
        let wl = ValidatorWhitelist::new();
        assert_eq!(wl.validator_count(), 0);

        wl.hotkeys
            .write()
            .insert("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY".to_string());
        assert_eq!(wl.validator_count(), 1);

        wl.hotkeys
            .write()
            .insert("5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty".to_string());
        assert_eq!(wl.validator_count(), 2);
    }
}
