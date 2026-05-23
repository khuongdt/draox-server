use crate::verdict::GuardVerdict;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use server_config::model::IpReputationConfig;
use server_core::ShutdownReceiver;
use std::net::IpAddr;
use std::sync::Arc;
use tracing::debug;

/// Per-IP reputation entry tracking score and timestamps.
#[derive(Debug, Clone)]
pub struct ReputationEntry {
    /// Current reputation score.
    pub score: u32,
    /// When the last violation was recorded.
    pub last_violation: Option<DateTime<Utc>>,
    /// When the score was last recovered.
    pub last_recovery: Option<DateTime<Utc>>,
}

/// Tracks IP reputation scores for connection quality assessment.
///
/// IPs start with an initial score and lose points on violations. Scores
/// recover automatically over time. IPs below the minimum score threshold
/// are blocked from connecting.
pub struct ReputationTracker {
    entries: DashMap<IpAddr, ReputationEntry>,
    config: IpReputationConfig,
}

impl ReputationTracker {
    /// Create a new ReputationTracker from configuration.
    pub fn new(config: IpReputationConfig) -> Self {
        Self {
            entries: DashMap::new(),
            config,
        }
    }

    /// Get the reputation score for an IP address.
    ///
    /// Returns the initial score if the IP has no recorded reputation.
    pub fn get_score(&self, ip: IpAddr) -> u32 {
        self.entries
            .get(&ip)
            .map(|e| e.score)
            .unwrap_or(self.config.initial_score)
    }

    /// Penalize an IP address by reducing its reputation score.
    pub fn penalize(&self, ip: IpAddr) {
        let mut entry = self.entries.entry(ip).or_insert_with(|| ReputationEntry {
            score: self.config.initial_score,
            last_violation: None,
            last_recovery: None,
        });

        entry.score = entry.score.saturating_sub(self.config.violation_penalty);
        entry.last_violation = Some(Utc::now());
        debug!(
            "Penalized IP {} — score now {} (penalty: {})",
            ip, entry.score, self.config.violation_penalty
        );
    }

    /// Check if an IP's reputation allows connection.
    pub fn check_reputation(&self, ip: IpAddr) -> GuardVerdict {
        if !self.config.enabled {
            return GuardVerdict::Allow;
        }

        let score = self.get_score(ip);
        if score < self.config.min_score_to_connect {
            GuardVerdict::Block(format!(
                "reputation score too low ({score} < {})",
                self.config.min_score_to_connect
            ))
        } else {
            GuardVerdict::Allow
        }
    }

    /// Start a background task that periodically recovers reputation scores.
    ///
    /// Runs every 60 seconds and increases all scores by the configured
    /// recovery rate (scaled to per-minute). Stops on shutdown signal.
    pub fn start_recovery_task(self: &Arc<Self>, mut shutdown: ShutdownReceiver) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Reputation recovery task shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                        this.recover_scores();
                    }
                }
            }
        });
    }

    /// Recover all reputation scores by the configured per-hour rate (applied per minute).
    fn recover_scores(&self) {
        let now = Utc::now();
        // recovery_rate_per_hour / 60 minutes, minimum 1 per tick if rate > 0
        let recovery_per_minute = if self.config.recovery_rate_per_hour >= 60 {
            self.config.recovery_rate_per_hour / 60
        } else if self.config.recovery_rate_per_hour > 0 {
            1 // Apply at least 1 point per minute for slow recovery
        } else {
            return;
        };

        let initial = self.config.initial_score;
        let mut recovered = 0u32;

        for mut entry in self.entries.iter_mut() {
            if entry.score < initial {
                entry.score = (entry.score + recovery_per_minute).min(initial);
                entry.last_recovery = Some(now);
                recovered += 1;
            }
        }

        if recovered > 0 {
            debug!(
                "Recovered reputation for {} IPs (+{} per tick)",
                recovered, recovery_per_minute
            );
        }
    }

    /// Get a reference to the entries map (for admin API inspection).
    pub fn entries(&self) -> &DashMap<IpAddr, ReputationEntry> {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> IpReputationConfig {
        IpReputationConfig {
            enabled: true,
            initial_score: 100,
            min_score_to_connect: 20,
            violation_penalty: 10,
            recovery_rate_per_hour: 5,
            score_persistence: "memory".to_string(),
        }
    }

    #[test]
    fn test_initial_score() {
        let tracker = ReputationTracker::new(test_config());
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Unknown IP should have initial score
        assert_eq!(tracker.get_score(ip), 100);
    }

    #[test]
    fn test_penalize() {
        let tracker = ReputationTracker::new(test_config());
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        tracker.penalize(ip);
        assert_eq!(tracker.get_score(ip), 90);

        tracker.penalize(ip);
        assert_eq!(tracker.get_score(ip), 80);
    }

    #[test]
    fn test_below_threshold_blocked() {
        let config = IpReputationConfig {
            enabled: true,
            initial_score: 100,
            min_score_to_connect: 20,
            violation_penalty: 15,
            recovery_rate_per_hour: 5,
            score_persistence: "memory".to_string(),
        };
        let tracker = ReputationTracker::new(config);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // Penalize until below threshold: 100 -> 85 -> 70 -> 55 -> 40 -> 25 -> 10
        for _ in 0..6 {
            tracker.penalize(ip);
        }

        assert_eq!(tracker.get_score(ip), 10);
        let verdict = tracker.check_reputation(ip);
        assert!(matches!(verdict, GuardVerdict::Block(_)));
    }
}
