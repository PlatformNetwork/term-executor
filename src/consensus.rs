use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

const REAPER_INTERVAL_SECS: u64 = 30;

struct PendingConsensus {
    archive_data: Vec<u8>,
    voters: HashSet<String>,
    created_at: Instant,
    concurrent_tasks: Option<usize>,
}

pub enum ConsensusStatus {
    Pending {
        votes: usize,
        required: usize,
        total_validators: usize,
    },
    Reached {
        archive_data: Vec<u8>,
        concurrent_tasks: Option<usize>,
        votes: usize,
        required: usize,
    },
    AlreadyVoted {
        votes: usize,
        required: usize,
        total_validators: usize,
    },
}

pub struct ConsensusManager {
    pending: DashMap<String, PendingConsensus>,
    max_pending: usize,
}

impl ConsensusManager {
    pub fn new(max_pending: usize) -> Arc<Self> {
        Arc::new(Self {
            pending: DashMap::new(),
            max_pending,
        })
    }

    pub fn record_vote(
        &self,
        archive_hash: &str,
        hotkey: &str,
        archive_data: Vec<u8>,
        concurrent_tasks: Option<usize>,
        required: usize,
        total_validators: usize,
    ) -> ConsensusStatus {
        match self.pending.entry(archive_hash.to_string()) {
            Entry::Occupied(mut entry) => {
                let pending = entry.get_mut();

                if pending.voters.contains(hotkey) {
                    return ConsensusStatus::AlreadyVoted {
                        votes: pending.voters.len(),
                        required,
                        total_validators,
                    };
                }

                pending.voters.insert(hotkey.to_string());
                let votes = pending.voters.len();

                if votes >= required {
                    let (_, consensus) = entry.remove_entry();
                    info!(archive_hash, votes, required, "Consensus reached");
                    ConsensusStatus::Reached {
                        archive_data: consensus.archive_data,
                        concurrent_tasks: consensus.concurrent_tasks,
                        votes,
                        required,
                    }
                } else {
                    ConsensusStatus::Pending {
                        votes,
                        required,
                        total_validators,
                    }
                }
            }
            Entry::Vacant(entry) => {
                info!(archive_hash, "New consensus entry created");
                let mut voters = HashSet::new();
                voters.insert(hotkey.to_string());
                let votes = 1;

                if votes >= required {
                    info!(archive_hash, votes, required, "Consensus reached");
                    ConsensusStatus::Reached {
                        archive_data,
                        concurrent_tasks,
                        votes,
                        required,
                    }
                } else {
                    entry.insert(PendingConsensus {
                        archive_data,
                        voters,
                        created_at: Instant::now(),
                        concurrent_tasks,
                    });
                    ConsensusStatus::Pending {
                        votes,
                        required,
                        total_validators,
                    }
                }
            }
        }
    }

    #[cfg(test)]
    fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn is_at_capacity(&self) -> bool {
        self.pending.len() >= self.max_pending
    }

    pub async fn reaper_loop(self: Arc<Self>, ttl_secs: u64) {
        let mut interval = tokio::time::interval(Duration::from_secs(REAPER_INTERVAL_SECS));
        loop {
            interval.tick().await;
            let cutoff = Instant::now() - Duration::from_secs(ttl_secs);
            let before = self.pending.len();
            self.pending.retain(|hash, entry| {
                let keep = entry.created_at > cutoff;
                if !keep {
                    debug!(archive_hash = %hash, "Expired pending consensus entry");
                }
                keep
            });
            let removed = before.saturating_sub(self.pending.len());
            if removed > 0 {
                info!(
                    removed,
                    remaining = self.pending.len(),
                    "Reaped expired consensus entries"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_vote_does_not_trigger() {
        let mgr = ConsensusManager::new(100);
        let status = mgr.record_vote("abc123", "hotkey1", vec![1, 2, 3], Some(8), 2, 3);
        assert!(matches!(
            status,
            ConsensusStatus::Pending {
                votes: 1,
                required: 2,
                ..
            }
        ));
    }

    #[test]
    fn test_reaching_threshold_triggers() {
        let mgr = ConsensusManager::new(100);
        mgr.record_vote("abc123", "hotkey1", vec![1, 2, 3], Some(8), 2, 3);
        let status = mgr.record_vote("abc123", "hotkey2", vec![1, 2, 3], Some(8), 2, 3);
        assert!(matches!(status, ConsensusStatus::Reached { votes: 2, .. }));
    }

    #[test]
    fn test_duplicate_votes_no_double_count() {
        let mgr = ConsensusManager::new(100);
        mgr.record_vote("abc123", "hotkey1", vec![1, 2, 3], Some(8), 3, 5);
        let status = mgr.record_vote("abc123", "hotkey1", vec![1, 2, 3], Some(8), 3, 5);
        assert!(matches!(
            status,
            ConsensusStatus::AlreadyVoted { votes: 1, .. }
        ));
    }

    #[test]
    fn test_different_hashes_independent() {
        let mgr = ConsensusManager::new(100);
        mgr.record_vote("hash1", "hotkey1", vec![1], Some(8), 2, 3);
        mgr.record_vote("hash2", "hotkey1", vec![2], Some(8), 2, 3);
        assert_eq!(mgr.pending_count(), 2);
    }

    #[test]
    fn test_ttl_expiration() {
        let mgr = ConsensusManager::new(100);
        mgr.pending.insert(
            "old_hash".to_string(),
            PendingConsensus {
                archive_data: vec![1],
                voters: HashSet::from(["hotkey1".to_string()]),
                created_at: Instant::now() - Duration::from_secs(120),
                concurrent_tasks: None,
            },
        );
        mgr.pending.insert(
            "new_hash".to_string(),
            PendingConsensus {
                archive_data: vec![2],
                voters: HashSet::from(["hotkey2".to_string()]),
                created_at: Instant::now(),
                concurrent_tasks: None,
            },
        );

        let cutoff = Instant::now() - Duration::from_secs(60);
        mgr.pending.retain(|_, entry| entry.created_at > cutoff);

        assert_eq!(mgr.pending_count(), 1);
        assert!(mgr.pending.contains_key("new_hash"));
        assert!(!mgr.pending.contains_key("old_hash"));
    }

    #[test]
    fn test_capacity_check() {
        let mgr = ConsensusManager::new(2);
        assert!(!mgr.is_at_capacity());
        mgr.pending.insert(
            "h1".to_string(),
            PendingConsensus {
                archive_data: vec![],
                voters: HashSet::new(),
                created_at: Instant::now(),
                concurrent_tasks: None,
            },
        );
        mgr.pending.insert(
            "h2".to_string(),
            PendingConsensus {
                archive_data: vec![],
                voters: HashSet::new(),
                created_at: Instant::now(),
                concurrent_tasks: None,
            },
        );
        assert!(mgr.is_at_capacity());
    }

    #[test]
    fn test_single_validator_consensus() {
        let mgr = ConsensusManager::new(100);
        let status = mgr.record_vote("hash1", "hotkey1", vec![1, 2, 3], Some(4), 1, 1);
        assert!(matches!(status, ConsensusStatus::Reached { votes: 1, .. }));
        assert_eq!(mgr.pending_count(), 0);
    }

    #[test]
    fn test_entry_removed_after_consensus() {
        let mgr = ConsensusManager::new(100);
        mgr.record_vote("hash1", "hotkey1", vec![1], Some(8), 2, 3);
        mgr.record_vote("hash1", "hotkey2", vec![1], Some(8), 2, 3);
        assert_eq!(mgr.pending_count(), 0);
    }
}
