use chrono::{DateTime, Utc};
use dashmap::DashMap;
use server_config::model::BanningConfig;
use server_core::ShutdownReceiver;
use std::net::IpAddr;
use std::sync::Arc;
use tracing::{debug, info};

/// A record of an active ban for an IP address.
#[derive(Debug, Clone)]
pub struct BanEntry {
    /// When this ban was applied.
    pub banned_at: DateTime<Utc>,
    /// When this ban expires.
    pub expires_at: DateTime<Utc>,
    /// Number of times this IP has been banned (for escalation).
    pub ban_count: u32,
    /// The reason for the ban.
    pub reason: String,
}

/// A record tracking violation counts for an IP before banning.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ViolationRecord {
    /// Number of violations recorded.
    count: u32,
    /// When the first violation in the current window was recorded.
    first_violation_at: DateTime<Utc>,
}

/// Manages IP bans with escalating durations and automatic violation tracking.
///
/// When an IP accumulates enough violations (as configured), it is automatically
/// banned. Repeated bans result in exponentially longer ban durations, capped
/// at the configured maximum.
pub struct BanManager {
    bans: DashMap<IpAddr, BanEntry>,
    violations: DashMap<IpAddr, ViolationRecord>,
    config: BanningConfig,
}

impl BanManager {
    /// Create a new BanManager from configuration.
    pub fn new(config: BanningConfig) -> Self {
        Self {
            bans: DashMap::new(),
            violations: DashMap::new(),
            config,
        }
    }

    /// Check if an IP is currently banned. Returns Some(BanEntry) if banned.
    pub fn is_banned(&self, ip: IpAddr) -> Option<BanEntry> {
        if let Some(entry) = self.bans.get(&ip) {
            if Utc::now() < entry.expires_at {
                return Some(entry.clone());
            }
            // Ban has expired; will be cleaned up by the background task
        }
        None
    }

    /// Ban an IP address with escalating duration.
    ///
    /// The ban duration starts at `initial_ban_duration_secs` and increases
    /// by `ban_escalation_multiplier` for each subsequent ban, capped at
    /// `max_ban_duration_secs`.
    pub fn ban(&self, ip: IpAddr, reason: &str) -> BanEntry {
        let now = Utc::now();
        let previous_ban_count = self
            .bans
            .get(&ip)
            .map(|e| e.ban_count)
            .unwrap_or(0);

        let ban_count = previous_ban_count + 1;
        let multiplier = (self.config.ban_escalation_multiplier as u64)
            .pow(ban_count.saturating_sub(1));
        let duration_secs = (self.config.initial_ban_duration_secs * multiplier)
            .min(self.config.max_ban_duration_secs);

        let expires_at = now + chrono::Duration::seconds(duration_secs as i64);

        let entry = BanEntry {
            banned_at: now,
            expires_at,
            ban_count,
            reason: reason.to_string(),
        };

        info!(
            "Banned IP {} for {}s (ban #{}, reason: {})",
            ip, duration_secs, ban_count, reason
        );

        self.bans.insert(ip, entry.clone());
        // Clear violation record after banning
        self.violations.remove(&ip);

        entry
    }

    /// Manually unban an IP address.
    pub fn unban(&self, ip: IpAddr) -> bool {
        let removed = self.bans.remove(&ip).is_some();
        if removed {
            info!("Manually unbanned IP {}", ip);
        }
        removed
    }

    /// Record a violation for an IP address.
    ///
    /// When the violation count reaches the configured threshold, the IP
    /// is automatically banned.
    pub fn record_violation(&self, ip: IpAddr) -> Option<BanEntry> {
        if !self.config.enabled {
            return None;
        }

        let now = Utc::now();
        let mut entry = self.violations.entry(ip).or_insert_with(|| ViolationRecord {
            count: 0,
            first_violation_at: now,
        });

        entry.count += 1;
        debug!(
            "Violation recorded for IP {} (count: {})",
            ip, entry.count
        );

        if entry.count >= self.config.max_violations_before_ban {
            drop(entry);
            return Some(self.ban(ip, "max violations exceeded"));
        }

        None
    }

    /// Start a background task that periodically removes expired bans.
    ///
    /// Runs every 10 seconds and removes any bans whose expiration time
    /// has passed. Stops when the shutdown signal is received.
    pub fn start_cleanup_task(self: &Arc<Self>, mut shutdown: ShutdownReceiver) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Ban cleanup task shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                        this.cleanup_expired();
                    }
                }
            }
        });
    }

    /// Remove all expired bans.
    fn cleanup_expired(&self) {
        let now = Utc::now();
        let mut removed = 0u32;
        self.bans.retain(|ip, entry| {
            if now >= entry.expires_at {
                debug!("Expired ban removed for IP {}", ip);
                removed += 1;
                false
            } else {
                true
            }
        });
        if removed > 0 {
            debug!("Cleaned up {} expired bans", removed);
        }
    }

    /// Get a reference to the active bans map (for admin API inspection).
    pub fn active_bans(&self) -> &DashMap<IpAddr, BanEntry> {
        &self.bans
    }

    /// Number of currently active bans.
    pub fn active_ban_count(&self) -> usize {
        self.bans.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> BanningConfig {
        BanningConfig {
            enabled: true,
            max_violations_before_ban: 3,
            initial_ban_duration_secs: 60,
            ban_escalation_multiplier: 2,
            max_ban_duration_secs: 3600,
            auth_failure_threshold: 10,
            auth_failure_window_secs: 300,
        }
    }

    #[test]
    fn test_ban_and_check() {
        let manager = BanManager::new(test_config());
        let ip: IpAddr = "192.168.1.50".parse().unwrap();

        // Not banned initially
        assert!(manager.is_banned(ip).is_none());

        // Ban the IP
        manager.ban(ip, "test ban");

        // Should be banned now
        let entry = manager.is_banned(ip);
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.ban_count, 1);
        assert_eq!(entry.reason, "test ban");
    }

    #[test]
    fn test_ban_escalation() {
        let manager = BanManager::new(test_config());
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // First ban: 60s
        let entry1 = manager.ban(ip, "first offense");
        assert_eq!(entry1.ban_count, 1);
        let duration1 = (entry1.expires_at - entry1.banned_at).num_seconds();
        assert_eq!(duration1, 60);

        // Second ban: 60 * 2^1 = 120s
        let entry2 = manager.ban(ip, "second offense");
        assert_eq!(entry2.ban_count, 2);
        let duration2 = (entry2.expires_at - entry2.banned_at).num_seconds();
        assert_eq!(duration2, 120);

        // Third ban: 60 * 2^2 = 240s
        let entry3 = manager.ban(ip, "third offense");
        assert_eq!(entry3.ban_count, 3);
        let duration3 = (entry3.expires_at - entry3.banned_at).num_seconds();
        assert_eq!(duration3, 240);
    }

    #[test]
    fn test_violation_tracking() {
        let manager = BanManager::new(test_config());
        let ip: IpAddr = "172.16.0.1".parse().unwrap();

        // First two violations should not result in a ban
        assert!(manager.record_violation(ip).is_none());
        assert!(manager.record_violation(ip).is_none());

        // Third violation should trigger auto-ban (threshold is 3)
        let ban_entry = manager.record_violation(ip);
        assert!(ban_entry.is_some());
        assert!(manager.is_banned(ip).is_some());
    }

    #[test]
    fn test_ban_expiration() {
        let config = BanningConfig {
            enabled: true,
            max_violations_before_ban: 3,
            initial_ban_duration_secs: 0, // Expire immediately
            ban_escalation_multiplier: 1,
            max_ban_duration_secs: 0,
            auth_failure_threshold: 10,
            auth_failure_window_secs: 300,
        };
        let manager = BanManager::new(config);
        let ip: IpAddr = "1.2.3.4".parse().unwrap();

        manager.ban(ip, "short ban");

        // Ban with 0 duration should already be expired
        assert!(manager.is_banned(ip).is_none());
    }

    #[test]
    fn test_manual_unban() {
        let manager = BanManager::new(test_config());
        let ip: IpAddr = "192.168.1.100".parse().unwrap();

        manager.ban(ip, "test");
        assert!(manager.is_banned(ip).is_some());

        // Unban
        let removed = manager.unban(ip);
        assert!(removed);
        assert!(manager.is_banned(ip).is_none());

        // Unban again should return false
        let removed = manager.unban(ip);
        assert!(!removed);
    }
}
